use super::{Analysis, Lexicon, Model, Node, invalid, read_characters, read_matrix};
use std::{
    fmt, fs,
    io::{self, Write},
    path::Path,
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
            if let Some((value, check)) = self.unit(base as usize)
                && value < 0
                && check == base as u32
            {
                result.push((
                    String::from_utf8(key.clone())
                        .map_err(|_| invalid("辞書の見出し語が UTF-8 ではありません"))?,
                    value.wrapping_neg().wrapping_sub(1) as u32,
                ));
            }
            for byte in (0u8..=255).rev() {
                let next = base as usize + byte as usize + 1;
                if let Some((_, check)) = self.unit(next)
                    && check == base as u32
                {
                    let mut child = key.clone();
                    child.push(byte);
                    stack.push((next, child));
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
        let dict = vibrato::Dictionary::from_path(&self.0.system_path, vibrato::LoadMode::Validate)
            .map_err(io::Error::other)?;
        let mut tokenizer = Self::tokenizer_from(dict)?.0.as_ref().clone();
        if !self.0.user_dictionaries.is_empty() {
            let mut csv = Vec::new();
            for (index, lexicon) in self.0.user_dictionaries.iter().enumerate().rev() {
                csv.extend(lexicon.csv(index as u8 + 1, false)?);
            }
            tokenizer = tokenizer
                .with_user_lexicon(csv.as_slice())
                .map_err(io::Error::other)?;
        }
        Ok(SharedTokenizer(Arc::new(tokenizer)))
    }

    fn tokenizer_from(dict: vibrato::Dictionary) -> io::Result<SharedTokenizer> {
        Ok(SharedTokenizer(Arc::new(
            vibrato::Tokenizer::new(dict)
                .prefer_dictionary_on_tie(true)
                .suppress_unknown_for_user_lexicon(true)
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

pub(super) fn write_system_dictionary(directory: &Path, path: &Path) -> io::Result<()> {
    let system = Lexicon::open(&directory.join("sys.dic"))?;
    if system.kind != 0 {
        return Err(invalid("システム辞書の種類が不正です"));
    }
    let unknown = Lexicon::open(&directory.join("unk.dic"))?;
    if unknown.kind != 2
        || unknown.left_size != system.left_size
        || unknown.right_size != system.right_size
    {
        return Err(invalid("未知語辞書がシステム辞書と互換ではありません"));
    }
    let (category_names, char_infos) = read_characters(&directory.join("char.bin"))?;
    let (left_size, right_size, costs) = read_matrix(&directory.join("matrix.bin"))?;
    if left_size != system.left_size || right_size != system.right_size {
        return Err(invalid("接続行列と辞書の文脈 ID の範囲が一致しません"));
    }

    let mut chars = String::new();
    use std::fmt::Write as _;
    for (index, name) in category_names.iter().enumerate() {
        let info = char_infos
            .iter()
            .find(|info| info.category() == index)
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
    while start < char_infos.len() {
        let info = char_infos[start];
        let mut end = start + 1;
        while end < char_infos.len() && char_infos[end].0 == info.0 {
            end += 1;
        }
        write!(
            chars,
            "0x{start:04X}..0x{:04X} {}",
            end - 1,
            category_names[info.category()]
        )
        .unwrap();
        for (index, name) in category_names.iter().enumerate() {
            if index != info.category() && info.0 & (1 << index) != 0 {
                write!(chars, " {name}").unwrap();
            }
        }
        chars.push('\n');
        start = end;
    }

    let mut matrix = Vec::new();
    writeln!(matrix, "{left_size} {right_size}")?;
    for left in 0..right_size {
        for right in 0..left_size {
            writeln!(matrix, "{right} {left} {}", costs[right + left_size * left])?;
        }
    }
    let lexicon = system.csv(0, false)?;
    let unknown = unknown.csv(255, true)?;
    let dictionary = vibrato::SystemDictionaryBuilder::from_readers(
        lexicon.as_slice(),
        matrix.as_slice(),
        chars.as_bytes(),
        unknown.as_slice(),
    )
    .map_err(io::Error::other)?;
    let mut bytes = Vec::new();
    dictionary.write(&mut bytes).map_err(io::Error::other)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(&bytes)?;
    file.persist(path).map_err(io::Error::other)?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::Model;

    #[test]
    fn user_lexicon_suppression_is_enabled_in_the_backend() {
        let dict = vibrato::SystemDictionaryBuilder::from_readers(
            "東京,0,0,0,system\n".as_bytes(),
            "1 1\n0 0 0\n".as_bytes(),
            "DEFAULT 1 1 2\nSPACE 0 1 0\n0x0020 SPACE\n".as_bytes(),
            "DEFAULT,0,0,100,unknown\n".as_bytes(),
        )
        .unwrap();
        let tokenizer = Model::tokenizer_from(vibrato::Dictionary::from_inner(dict)).unwrap();
        let tokenizer = tokenizer
            .0
            .as_ref()
            .clone()
            .with_user_lexicon("カキ,0,0,150,user\n東京,0,0,150,user\n".as_bytes())
            .unwrap();
        let mut worker = tokenizer.new_worker();
        for (text, expected) in [("カキ", "user"), ("カキク", "unknown"), ("東京", "system")]
        {
            worker.reset_sentence(text);
            worker.tokenize();
            assert_eq!(worker.num_tokens(), 1);
            assert_eq!(worker.token(0).feature(), expected);
            let lattice = worker.lattice_snapshot().unwrap();
            assert!(
                !lattice
                    .nodes
                    .iter()
                    .any(|n| n.feature == "unknown" && n.range_char == (0..2))
            );
        }
    }
}
