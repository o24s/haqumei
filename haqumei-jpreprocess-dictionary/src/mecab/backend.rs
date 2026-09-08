use super::{invalid, Analysis, Lexicon, Model, Node};
use sha2::{Digest, Sha256};
use std::{
    fmt, fs,
    io::{self, Write},
    sync::Arc,
};

pub(super) struct SharedTokenizer(Arc<vibrato::Tokenizer>);
impl fmt::Debug for SharedTokenizer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SharedTokenizer")
    }
}

/// 最良経路の解析用バッファを再利用します。
pub struct Worker(vibrato::tokenizer::worker::Worker, Model);
impl fmt::Debug for Worker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Worker")
    }
}
// 1-best のノードは添字で接続され、生ポインターの lpath は常に null になる。
// 内部ワーカーを公開せず、ポインターを使う N-best の呼び出しを禁止している。
unsafe impl Send for Worker {}

impl Lexicon {
    fn entries(&self) -> io::Result<Vec<(String, u32)>> {
        let mut result = Vec::new();
        let mut stack = vec![(0usize, Vec::new())];
        let mut visited = std::collections::HashSet::new();
        while let Some((index, key)) = stack.pop() {
            if !visited.insert(index) {
                return Err(invalid("辞書のトライが循環しています"));
            }
            let (base, _) = self
                .unit(index)
                .ok_or_else(|| invalid("トライの参照が不正です"))?;
            if base < 0 {
                return Err(invalid("トライの遷移先が不正です"));
            }
            if let Some((value, check)) = self.unit(base as usize) {
                if value < 0 && check == base as u32 {
                    result.push((
                        String::from_utf8(key.clone())
                            .map_err(|_| invalid("辞書の見出し語が UTF-8 ではありません"))?,
                        value.wrapping_neg().wrapping_sub(1) as u32,
                    ));
                }
            }
            for byte in (0u8..=255).rev() {
                let next = base as usize + byte as usize + 1;
                if let Some((_, check)) = self.unit(next) {
                    if check == base as u32 {
                        let mut child = key.clone();
                        child.push(byte);
                        stack.push((next, child));
                    }
                }
            }
        }
        Ok(result)
    }

    fn csv(&self, index: u8, unknown: bool) -> io::Result<Vec<u8>> {
        let mut csv = Vec::new();
        for (surface, value) in self.entries()? {
            if surface.is_empty() {
                continue;
            }
            let mut nodes = Vec::new();
            self.append(value, 0..0, index, unknown, &mut nodes)?;
            // MeCab の同点候補の選択順に合わせ、同じ見出し語のエントリを逆順で登録する。
            for (rank, node) in nodes.into_iter().enumerate().rev() {
                let escaped = surface.replace('"', "\"\"");
                writeln!(
                    csv,
                    "\"{escaped}\",{},{},{},{},{},{rank},{}",
                    node.left_id,
                    node.right_id,
                    node.word_cost,
                    node.pos_id,
                    node.dictionary_index,
                    node.feature
                )?;
            }
        }
        Ok(csv)
    }
}

impl Model {
    fn make_tokenizer(&self) -> io::Result<SharedTokenizer> {
        let data = &self.0;
        let mut hash = Sha256::new();
        hash.update(b"haqumei-vibrato-archive-v1");
        for lex in data
            .dictionaries
            .iter()
            .chain(std::iter::once(&data.unknown))
        {
            hash.update((lex.bytes.len() as u64).to_le_bytes());
            hash.update(&lex.bytes);
        }
        for name in &data.category_names {
            hash.update(name.as_bytes());
            hash.update([0]);
        }
        for info in &data.chars {
            hash.update(info.0.to_le_bytes());
        }
        for cost in &data.matrix {
            hash.update(cost.to_le_bytes());
        }
        let cache = dirs::cache_dir().map(|p| {
            p.join("haqumei")
                .join("vibrato")
                .join(format!("{}.dict", hex::encode(hash.finalize())))
        });
        if let Some(path) = &cache {
            if let Ok(dict) = vibrato::Dictionary::from_path(path, vibrato::LoadMode::Validate) {
                return Self::tokenizer_from(dict);
            }
        }
        let mut chars = String::new();
        use std::fmt::Write as _;
        for (index, name) in data.category_names.iter().enumerate() {
            let info = data
                .chars
                .iter()
                .find(|c| c.category() == index)
                .copied()
                .unwrap_or(super::CharInfo((index as u32) << 18));
            writeln!(
                chars,
                "{name} {} {} {}",
                u8::from(info.invoke()),
                u8::from(info.group()),
                info.length()
            )
            .unwrap();
        }
        let mut start = 0usize;
        while start < data.chars.len() {
            let info = data.chars[start];
            let mut end = start + 1;
            while end < data.chars.len() && data.chars[end].0 == info.0 {
                end += 1;
            }
            write!(
                chars,
                "0x{start:04X}..0x{:04X} {}",
                end - 1,
                data.category_names[info.category()]
            )
            .unwrap();
            for (i, name) in data.category_names.iter().enumerate() {
                if i != info.category() && info.0 & (1 << i) != 0 {
                    write!(chars, " {name}").unwrap();
                }
            }
            chars.push('\n');
            start = end;
        }
        let mut matrix = Vec::new();
        writeln!(matrix, "{} {}", data.left_size, data.right_size)?;
        for left in 0..data.right_size {
            for right in 0..data.left_size {
                writeln!(
                    matrix,
                    "{right} {left} {}",
                    data.matrix[right + data.left_size * left]
                )?;
            }
        }
        let mut lexicon = Vec::new();
        // 同点では先に指定された辞書を選ぶため、vibrato への登録順を反転する。
        for (i, lex) in data.dictionaries.iter().enumerate().rev() {
            lexicon.extend(lex.csv(i as u8, false)?);
        }
        let unknown = data.unknown.csv(255, true)?;
        let dict = vibrato::SystemDictionaryBuilder::from_readers(
            lexicon.as_slice(),
            matrix.as_slice(),
            chars.as_bytes(),
            unknown.as_slice(),
        )
        .map_err(io::Error::other)?;
        let mut bytes = Vec::new();
        dict.write(&mut bytes).map_err(io::Error::other)?;
        if let Some(path) = cache {
            if let Some(parent) = path.parent() {
                if fs::create_dir_all(parent).is_ok() {
                    // 読み込み中の mmap を切り詰めないよう、別ファイルへの書き込み後に置き換える。
                    if let Ok(mut file) = tempfile::NamedTempFile::new_in(parent) {
                        if file.write_all(&bytes).is_ok() {
                            let _ = file.persist(path);
                        }
                    }
                }
            }
        }
        Self::tokenizer_from(vibrato::Dictionary::from_bytes(&bytes).map_err(io::Error::other)?)
    }

    fn tokenizer_from(dict: vibrato::Dictionary) -> io::Result<SharedTokenizer> {
        Ok(SharedTokenizer(Arc::new(
            vibrato::Tokenizer::new(dict)
                .prefer_dictionary_on_tie(true)
                .ignore_space(true)
                .map_err(io::Error::other)?
                .max_grouping_len(24),
        )))
    }

    /// 読み込んだ辞書を使う vibrato-rkyv ワーカーを作ります。
    pub fn worker(&self) -> io::Result<Worker> {
        let tokenizer = self
            .0
            .tokenizer
            .get_or_init(|| self.make_tokenizer().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(|e| invalid(e))?;
        Ok(Worker(tokenizer.0.new_worker(), self.clone()))
    }
}

impl Worker {
    /// 正規化済み文字列の最良経路を返します。全候補は含みません。
    pub fn analyze(&mut self, text: &str) -> io::Result<Analysis> {
        self.0.reset_sentence(text);
        self.0.tokenize();
        let mut nodes = Vec::with_capacity(self.0.num_tokens());
        for token in self.0.token_iter() {
            let mut fields = token.feature().splitn(4, ',');
            let pos_id = fields
                .next()
                .and_then(|x| x.parse().ok())
                .ok_or_else(|| invalid("品詞 ID が不正です"))?;
            let dictionary_index = fields
                .next()
                .and_then(|x| x.parse().ok())
                .ok_or_else(|| invalid("辞書番号が不正です"))?;
            let _rank = fields.next().ok_or_else(|| invalid("登録順がありません"))?;
            let feature = fields
                .next()
                .ok_or_else(|| invalid("特徴量がありません"))?
                .to_owned();
            nodes.push(Node {
                byte_span: token.range_byte(),
                feature,
                left_id: token.left_id(),
                right_id: token.right_id(),
                pos_id,
                word_cost: token.word_cost(),
                dictionary_index,
                is_unknown: dictionary_index == 255,
                cost: token.total_cost() as i64,
                delta: 0,
            });
        }
        let best_path = (0..nodes.len()).collect();
        Ok(Analysis {
            total_cost: self
                .0
                .total_cost()
                .unwrap_or_else(|| i64::from(self.1.transition_cost(0, 0).unwrap_or(0))),
            nodes,
            best_path,
        })
    }
}

impl Worker {
    /// 全候補と、各候補を通る経路のコスト差を返します。
    pub fn analyze_lattice(&mut self, text: &str) -> io::Result<Analysis> {
        self.0.reset_sentence(text);
        self.0.tokenize();
        let Some(snapshot) = self.0.lattice_snapshot() else {
            return Ok(Analysis {
                nodes: Vec::new(),
                best_path: Vec::new(),
                total_cost: i64::from(self.1.transition_cost(0, 0).unwrap_or(0)),
            });
        };
        let mut candidates = Vec::with_capacity(snapshot.nodes.len());
        for (old_index, candidate) in snapshot.nodes.into_iter().enumerate() {
            let mut fields = candidate.feature.splitn(4, ',');
            let pos_id = fields
                .next()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| invalid("品詞 ID が不正です"))?;
            let dictionary_index = fields
                .next()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| invalid("辞書番号が不正です"))?;
            let rank: usize = fields
                .next()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| invalid("登録順が不正です"))?;
            let feature = fields
                .next()
                .ok_or_else(|| invalid("特徴量がありません"))?
                .to_owned();
            let node = Node {
                byte_span: candidate.range_byte,
                feature,
                left_id: candidate.left_id,
                right_id: candidate.right_id,
                pos_id,
                word_cost: candidate.word_cost,
                dictionary_index,
                is_unknown: dictionary_index == 255,
                cost: candidate.cost,
                delta: candidate.delta,
            };
            let (kind, width) = self.1.info(text, node.byte_span.start, text.len());
            let grouped = node.is_unknown
                && kind.group()
                && self
                    .1
                    .seek(text, node.byte_span.start + width, text.len(), kind, 25)
                    .0
                    == node.byte_span.end;
            candidates.push((candidate.start_node, grouped, rank, old_index, node));
        }
        // MeCab は未知語の短い候補、グループ候補、既知語の順に連結して返す。
        candidates.sort_by_key(|(start, grouped, rank, _, node)| {
            (
                *start,
                std::cmp::Reverse(node.dictionary_index),
                *grouped,
                std::cmp::Reverse(node.byte_span.end),
                std::cmp::Reverse(*rank),
            )
        });
        let mut remap = vec![0; candidates.len()];
        let nodes = candidates
            .into_iter()
            .enumerate()
            .map(|(new, (_, _, _, old, node))| {
                remap[old] = new;
                node
            })
            .collect();
        let best_path = snapshot
            .best_path
            .into_iter()
            .map(|old| remap[old])
            .collect();
        Ok(Analysis {
            nodes,
            best_path,
            total_cost: snapshot.total_cost,
        })
    }
}
