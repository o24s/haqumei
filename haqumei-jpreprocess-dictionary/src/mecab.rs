//! MeCab の辞書形式を読み、全候補と最良経路を解析します。

use std::{
    fs, io,
    ops::Range,
    path::{Path, PathBuf},
    sync::Arc,
};

mod backend;
pub use backend::Worker;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

#[derive(Debug)]
struct Lexicon {
    bytes: Vec<u8>,
    trie_end: usize,
    token_end: usize,
    left_size: usize,
    right_size: usize,
    kind: u32,
}

impl Lexicon {
    fn open(path: &Path) -> io::Result<Self> {
        let bytes = fs::read(path)?;
        let header = |at| u32_at(&bytes, at).ok_or_else(|| invalid("辞書のヘッダーが短すぎます"));
        if header(0)? ^ 0xef71_8f77 != bytes.len() as u32 || header(4)? != 102 {
            return Err(invalid("MeCab 辞書の形式またはバージョンが一致しません"));
        }
        let kind = header(8)?;
        let left_size = header(16)? as usize;
        let right_size = header(20)? as usize;
        let trie_size = header(24)? as usize;
        let token_size = header(28)? as usize;
        let feature_size = header(32)? as usize;
        if trie_size < 8 || !trie_size.is_multiple_of(8) || !token_size.is_multiple_of(16) {
            return Err(invalid("辞書のトライまたはエントリの長さが不正です"));
        }
        let trie_end = 72usize
            .checked_add(trie_size)
            .ok_or_else(|| invalid("辞書が大きすぎます"))?;
        let token_end = trie_end
            .checked_add(token_size)
            .ok_or_else(|| invalid("辞書が大きすぎます"))?;
        if token_end.checked_add(feature_size) != Some(bytes.len()) {
            return Err(invalid("辞書のファイル長がヘッダーと一致しません"));
        }
        let charset = &bytes[40..72];
        let end = charset
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(charset.len());
        if !charset[..end].eq_ignore_ascii_case(b"utf-8")
            && !charset[..end].eq_ignore_ascii_case(b"utf8")
        {
            return Err(invalid("辞書の文字コードは UTF-8 である必要があります"));
        }
        for offset in (trie_end..token_end).step_by(16) {
            if u16_at(&bytes, offset).unwrap() as usize >= right_size
                || u16_at(&bytes, offset + 2).unwrap() as usize >= left_size
                || u32_at(&bytes, offset + 8).unwrap() as usize >= feature_size
            {
                return Err(invalid("辞書のエントリが有効な範囲を超えています"));
            }
        }
        if bytes.last() != Some(&0) {
            return Err(invalid("辞書の特徴量の終端がありません"));
        }
        Ok(Self {
            bytes,
            trie_end,
            token_end,
            left_size,
            right_size,
            kind,
        })
    }

    fn unit(&self, index: usize) -> Option<(i32, u32)> {
        let offset = index.checked_mul(8)?.checked_add(72)?;
        if offset.checked_add(8)? > self.trie_end {
            return None;
        }
        Some((
            u32_at(&self.bytes, offset)? as i32,
            u32_at(&self.bytes, offset + 4)?,
        ))
    }

    fn prefixes(&self, key: &[u8]) -> Vec<(usize, u32)> {
        let mut results = Vec::new();
        let Some((mut base, _)) = self.unit(0) else {
            return results;
        };
        for offset in 0..=key.len() {
            if base < 0 {
                break;
            }
            if let Some((value, check)) = self.unit(base as usize)
                && check == base as u32
                && value < 0
            {
                results.push((offset, value.wrapping_neg().wrapping_sub(1) as u32));
            }
            let Some(&byte) = key.get(offset) else {
                break;
            };
            let Some((next, check)) = self.unit(base as usize + byte as usize + 1) else {
                break;
            };
            if check != base as u32 {
                break;
            }
            base = next;
        }
        results
    }

    fn append(
        &self,
        value: u32,
        span: Range<usize>,
        dictionary_index: u8,
        unknown: bool,
        output: &mut Vec<Node>,
    ) -> io::Result<()> {
        let first = (value >> 8) as usize;
        for index in first..first + (value & 0xff) as usize {
            let offset = self.trie_end + index * 16;
            if offset + 16 > self.token_end {
                return Err(invalid("トライが存在しないエントリを指しています"));
            }
            let feature_start = self.token_end + u32_at(&self.bytes, offset + 8).unwrap() as usize;
            let bytes = &self.bytes[feature_start..];
            let feature_end = bytes
                .iter()
                .position(|b| *b == 0)
                .ok_or_else(|| invalid("特徴量の終端がありません"))?;
            output.push(Node {
                byte_span: span.clone(),
                feature: String::from_utf8_lossy(&bytes[..feature_end]).into_owned(),
                left_id: u16_at(&self.bytes, offset).unwrap(),
                right_id: u16_at(&self.bytes, offset + 2).unwrap(),
                pos_id: u16_at(&self.bytes, offset + 4).unwrap(),
                word_cost: u16_at(&self.bytes, offset + 6).unwrap() as i16,
                dictionary_index,
                is_unknown: unknown,
                cost: 0,
                delta: 0,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct CharInfo(u32);

impl CharInfo {
    fn matches(self, other: Self) -> bool {
        self.0 & other.0 & 0x3ffff != 0
    }
    fn category(self) -> usize {
        ((self.0 >> 18) & 0xff) as usize
    }
    fn length(self) -> usize {
        ((self.0 >> 26) & 0xf) as usize
    }
    fn group(self) -> bool {
        self.0 & (1 << 30) != 0
    }
    fn invoke(self) -> bool {
        self.0 & (1 << 31) != 0
    }
}

/// 解析された候補形態素。特徴量には表層形を含みません。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Node {
    /// 解析文字列上のバイト位置。
    pub byte_span: Range<usize>,
    /// 品詞から始まる MeCab の特徴量文字列。
    pub feature: String,
    /// 左文脈 ID。
    pub left_id: u16,
    /// 右文脈 ID。
    pub right_id: u16,
    /// 品詞 ID。
    pub pos_id: u16,
    /// 辞書の単語コスト。
    pub word_cost: i16,
    /// システム辞書は 0、ユーザー辞書は指定順に 1 以降、未知語は 255。
    pub dictionary_index: u8,
    /// 未知語候補から生成したかどうか。
    pub is_unknown: bool,
    /// 文頭から候補までの最小累積コスト。
    pub cost: i64,
    /// 候補を通る最良経路と、文全体の最良経路のコスト差。
    pub delta: i64,
}

/// 全候補と、最良経路に選ばれた候補の添字。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    /// 位置の昇順で並べた候補。BOS と EOS は含みません。
    pub nodes: Vec<Node>,
    /// 文頭から文末へ並べた [`Self::nodes`] の添字。
    pub best_path: Vec<usize>,
    /// BOS から EOS までの最小累積コスト。
    pub total_cost: i64,
}

#[derive(Debug)]
struct Data {
    dictionaries: Vec<Lexicon>,
    unknown: Lexicon,
    #[cfg(test)]
    unknown_values: Vec<u32>,
    category_names: Vec<String>,
    tokenizer: std::sync::OnceLock<Result<backend::SharedTokenizer, String>>,
    chars: Vec<CharInfo>,
    matrix: Vec<i16>,
    left_size: usize,
    right_size: usize,
}

/// 辞書を共有する形態素解析器。
#[derive(Debug, Clone)]
pub struct Model(Arc<Data>);

impl Model {
    /// MeCab 形式の UTF-8 辞書と、指定順のユーザー辞書を読み込みます。
    pub fn open(directory: &Path, user_dictionaries: &[PathBuf]) -> io::Result<Self> {
        if user_dictionaries.len() >= 255 {
            return Err(invalid("ユーザー辞書は 254 個まで指定できます"));
        }
        let mut dictionaries = vec![Lexicon::open(&directory.join("sys.dic"))?];
        if dictionaries[0].kind != 0 {
            return Err(invalid("システム辞書の種類が不正です"));
        }
        for path in user_dictionaries {
            let dictionary = Lexicon::open(path)?;
            if dictionary.kind != 1
                || dictionary.left_size != dictionaries[0].left_size
                || dictionary.right_size != dictionaries[0].right_size
            {
                return Err(invalid("ユーザー辞書がシステム辞書と互換ではありません"));
            }
            dictionaries.push(dictionary);
        }
        let unknown = Lexicon::open(&directory.join("unk.dic"))?;
        if unknown.kind != 2
            || unknown.left_size != dictionaries[0].left_size
            || unknown.right_size != dictionaries[0].right_size
        {
            return Err(invalid("未知語辞書がシステム辞書と互換ではありません"));
        }
        let bytes = fs::read(directory.join("char.bin"))?;
        let count =
            u32_at(&bytes, 0).ok_or_else(|| invalid("文字種のヘッダーがありません"))? as usize;
        let names_end = 4usize
            .checked_add(
                count
                    .checked_mul(32)
                    .ok_or_else(|| invalid("文字種が多すぎます"))?,
            )
            .ok_or_else(|| invalid("文字種が多すぎます"))?;
        if count == 0 || count > 18 || names_end.checked_add(4 * 0xffff) != Some(bytes.len()) {
            return Err(invalid("文字種のファイル長が不正です"));
        }
        let mut unknown_values = Vec::with_capacity(count);
        let mut category_names = Vec::with_capacity(count);
        for i in 0..count {
            let name = &bytes[4 + 32 * i..4 + 32 * (i + 1)];
            let end = name.iter().position(|b| *b == 0).unwrap_or(name.len());
            category_names.push(
                String::from_utf8(name[..end].to_vec())
                    .map_err(|_| invalid("文字種名が UTF-8 ではありません"))?,
            );
            let value = unknown
                .prefixes(&name[..end])
                .into_iter()
                .find(|(n, _)| *n == end)
                .map(|(_, v)| v)
                .ok_or_else(|| invalid("未知語辞書に文字種がありません"))?;
            unknown_values.push(value);
        }
        let chars: Vec<_> = bytes[names_end..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| CharInfo(u32::from_le_bytes(*b)))
            .collect();
        if chars.iter().any(|c| c.category() >= count) {
            return Err(invalid("文字種の番号が不正です"));
        }
        let bytes = fs::read(directory.join("matrix.bin"))?;
        let left_size =
            u16_at(&bytes, 0).ok_or_else(|| invalid("接続行列のヘッダーがありません"))? as usize;
        let right_size =
            u16_at(&bytes, 2).ok_or_else(|| invalid("接続行列のヘッダーがありません"))? as usize;
        let expected_length = left_size
            .checked_mul(right_size)
            .and_then(|size| size.checked_mul(2))
            .and_then(|size| size.checked_add(4));
        if left_size != dictionaries[0].left_size
            || right_size != dictionaries[0].right_size
            || expected_length != Some(bytes.len())
        {
            return Err(invalid("接続行列と辞書の文脈 ID の範囲が一致しません"));
        }
        let matrix = bytes[4..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b))
            .collect();
        Ok(Self(Arc::new(Data {
            dictionaries,
            unknown,
            #[cfg(test)]
            unknown_values,
            category_names,
            tokenizer: std::sync::OnceLock::new(),
            chars,
            matrix,
            left_size,
            right_size,
        })))
    }

    /// 前の形態素の右文脈 ID と、次の形態素の左文脈 ID から接続コストを返します。
    pub fn transition_cost(&self, right_id: u16, left_id: u16) -> Option<i16> {
        if right_id as usize >= self.0.left_size || left_id as usize >= self.0.right_size {
            return None;
        }
        Some(self.0.matrix[right_id as usize + self.0.left_size * left_id as usize])
    }

    fn info(&self, text: &str, at: usize, end: usize) -> (CharInfo, usize) {
        let Some(rest) = text.get(at..end) else {
            return (self.0.chars[0], 1);
        };
        let Some(ch) = rest.chars().next() else {
            return (self.0.chars[0], 1);
        };
        // MeCab は UCS-2 の表を引くため、補助平面の文字には DEFAULT を使う。
        let code = if ch as usize >= 0xffff {
            0
        } else {
            ch as usize
        };
        (self.0.chars[code], ch.len_utf8())
    }

    fn seek(
        &self,
        text: &str,
        mut at: usize,
        end: usize,
        mut kind: CharInfo,
        limit: usize,
    ) -> (usize, CharInfo, usize, usize) {
        let mut count = 0;
        let (mut failed, mut width) = (CharInfo::default(), 0);
        while at < end && count < limit {
            (failed, width) = self.info(text, at, end);
            if !kind.matches(failed) {
                break;
            }
            at += width;
            count += 1;
            kind = failed;
        }
        (at, failed, width, count)
    }

    #[cfg(test)]
    fn lookup(&self, text: &str, position: usize) -> io::Result<Vec<Node>> {
        let mut end = text.len().min(position.saturating_add(65535));
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let (start, kind, width, _) =
            self.seek(text, position, end, self.0.chars[0x20], usize::MAX);
        if start == end {
            return Ok(Vec::new());
        }
        let mut nodes = Vec::new();
        for (index, dictionary) in self.0.dictionaries.iter().enumerate() {
            for (length, value) in dictionary.prefixes(&text.as_bytes()[start..end]) {
                if length > 0 {
                    if !text.is_char_boundary(start + length) {
                        return Err(invalid(
                            "辞書の見出し語が UTF-8 の文字境界で終わっていません",
                        ));
                    }
                    dictionary.append(
                        value,
                        start..start + length,
                        index as u8,
                        false,
                        &mut nodes,
                    )?;
                }
            }
        }
        if nodes.is_empty() || kind.invoke() {
            let mut next = start + width;
            let mut grouped_end = None;
            let add = |stop, nodes: &mut Vec<Node>| {
                self.0.unknown.append(
                    self.0.unknown_values[kind.category()],
                    start..stop,
                    255,
                    true,
                    nodes,
                )
            };
            if kind.group() {
                // 先頭を含めて 26 文字あれば候補にしないため、残りの同種文字は調べない。
                let (group_end, _, _, count) = self.seek(text, next, end, kind, 25);
                if count <= 24 {
                    add(group_end, &mut nodes)?;
                }
                grouped_end = Some(group_end);
            }
            for _ in 1..=kind.length() {
                if next > end {
                    break;
                }
                if Some(next) == grouped_end {
                    continue;
                }
                add(next, &mut nodes)?;
                let (following, width) = self.info(text, next, end);
                if !kind.matches(following) {
                    break;
                }
                next += width;
            }
            if nodes.is_empty() {
                add(next.min(end), &mut nodes)?;
            }
        }
        // MeCab は候補を連結リストの先頭へ追加する。同コスト時の選択もこの順序に従う。
        nodes.reverse();
        Ok(nodes)
    }

    /// 正規化済み文字列の全候補を生成し、Viterbi で最良経路を求めます。
    pub fn analyze(&self, text: &str) -> io::Result<Analysis> {
        self.worker()?.analyze_lattice(text)
    }

    #[cfg(test)]
    fn analyze_reference(&self, text: &str) -> io::Result<Analysis> {
        struct State {
            node: Node,
            previous: usize,
            backward: i64,
        }
        let mut states: Vec<State> = Vec::new();
        let mut begin: Vec<Vec<usize>> = vec![Vec::new(); text.len() + 1];
        let mut end: Vec<Vec<usize>> = vec![Vec::new(); text.len() + 1];
        for position in 0..text.len() {
            if position != 0 && end[position].is_empty() {
                continue;
            }
            for mut node in self.lookup(text, position)? {
                let mut best_cost = i64::MAX;
                let mut previous = usize::MAX;
                if position == 0 {
                    best_cost = i64::from(
                        self.transition_cost(0, node.left_id)
                            .ok_or_else(|| invalid("文脈 ID が不正です"))?,
                    ) + i64::from(node.word_cost);
                } else {
                    // end_node_list も先頭へ追加されるため、後から接続した候補を先に調べる。
                    for &index in end[position].iter().rev() {
                        let left = &states[index].node;
                        let cost = left.cost
                            + i64::from(
                                self.transition_cost(left.right_id, node.left_id)
                                    .ok_or_else(|| invalid("文脈 ID が不正です"))?,
                            )
                            + i64::from(node.word_cost);
                        if cost < best_cost {
                            best_cost = cost;
                            previous = index;
                        }
                    }
                }
                node.cost = best_cost;
                let index = states.len();
                end[node.byte_span.end].push(index);
                begin[position].push(index);
                states.push(State {
                    node,
                    previous,
                    backward: i64::MAX,
                });
            }
        }
        let last_position = (0..=text.len())
            .rev()
            .find(|&i| !end[i].is_empty())
            .unwrap_or(0);
        let mut total_cost = if last_position == 0 {
            i64::from(
                self.transition_cost(0, 0)
                    .ok_or_else(|| invalid("BOS の文脈 ID がありません"))?,
            )
        } else {
            i64::MAX
        };
        let mut best_end = usize::MAX;
        for &index in end[last_position].iter().rev() {
            let node = &states[index].node;
            let rest = i64::from(
                self.transition_cost(node.right_id, 0)
                    .ok_or_else(|| invalid("EOS の文脈 ID がありません"))?,
            );
            let cost = node.cost + rest;
            if cost < total_cost {
                total_cost = cost;
                best_end = index;
            }
            states[index].backward = rest;
        }
        for index in (0..states.len()).rev() {
            let node = &states[index].node;
            let mut rest = states[index].backward;
            for &right_index in &begin[node.byte_span.end] {
                let right = &states[right_index];
                if right.backward == i64::MAX {
                    continue;
                }
                let cost = right.backward
                    + i64::from(right.node.word_cost)
                    + i64::from(
                        self.transition_cost(node.right_id, right.node.left_id)
                            .ok_or_else(|| invalid("文脈 ID が不正です"))?,
                    );
                rest = rest.min(cost);
            }
            states[index].backward = rest;
        }
        let mut old_best = Vec::new();
        while best_end != usize::MAX {
            old_best.push(best_end);
            best_end = states[best_end].previous;
        }
        old_best.reverse();
        let mut old_to_new = vec![usize::MAX; states.len()];
        let mut nodes = Vec::with_capacity(states.len());
        for indices in begin {
            for index in indices {
                let state = &states[index];
                if state.backward == i64::MAX {
                    continue;
                }
                let mut node = state.node.clone();
                node.delta = node.cost + state.backward - total_cost;
                old_to_new[index] = nodes.len();
                nodes.push(node);
            }
        }
        let best_path = old_best
            .into_iter()
            .map(|index| old_to_new[index])
            .collect();
        Ok(Analysis {
            nodes,
            best_path,
            total_cost,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn lexicon(entries: &[(&str, i16, u16, &str)], kind: u32) -> Lexicon {
        let mut children = vec![BTreeMap::<u8, usize>::new()];
        let mut groups = BTreeMap::<&str, Vec<usize>>::new();
        for (index, (key, _, _, _)) in entries.iter().enumerate() {
            groups.entry(key).or_default().push(index);
        }
        let mut terminals = Vec::new();
        let mut tokens = Vec::new();
        let mut features = Vec::new();
        for (key, indices) in groups {
            let mut parent = 0;
            for &byte in key.as_bytes() {
                parent = if let Some(&child) = children[parent].get(&byte) {
                    child
                } else {
                    let child = children.len();
                    children.push(BTreeMap::new());
                    children[parent].insert(byte, child);
                    child
                };
            }
            let value = ((tokens.len() / 16) << 8) | indices.len();
            terminals.push((parent, value));
            for index in indices {
                let (_, cost, context, feature) = entries[index];
                tokens.extend_from_slice(&context.to_le_bytes());
                tokens.extend_from_slice(&context.to_le_bytes());
                tokens.extend_from_slice(&0u16.to_le_bytes());
                tokens.extend_from_slice(&cost.to_le_bytes());
                tokens.extend_from_slice(&(features.len() as u32).to_le_bytes());
                tokens.extend_from_slice(&0u32.to_le_bytes());
                features.extend_from_slice(feature.as_bytes());
                features.push(0);
            }
        }
        let mut units = vec![(0i32, 0u32); children.len() * 257 + 1];
        units[0].0 = 1;
        for (parent, edges) in children.iter().enumerate() {
            let base = parent * 257 + 1;
            for (&byte, &child) in edges {
                units[base + byte as usize + 1] = ((child * 257 + 1) as i32, base as u32);
            }
        }
        for (parent, value) in terminals {
            let base = parent * 257 + 1;
            units[base] = (-(value as i32) - 1, base as u32);
        }
        let mut bytes = vec![0u8; 72];
        for (base, check) in units {
            bytes.extend_from_slice(&base.to_le_bytes());
            bytes.extend_from_slice(&check.to_le_bytes());
        }
        let trie_end = bytes.len();
        bytes.extend(tokens);
        let token_end = bytes.len();
        bytes.extend(features);
        for (offset, value) in [
            (0, bytes.len() as u32 ^ 0xef71_8f77),
            (4, 102),
            (8, kind),
            (12, entries.len() as u32),
            (16, 2),
            (20, 2),
            (24, (trie_end - 72) as u32),
            (28, (token_end - trie_end) as u32),
            (32, (bytes.len() - token_end) as u32),
        ] {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes[40..45].copy_from_slice(b"utf-8");
        Lexicon {
            bytes,
            trie_end,
            token_end,
            left_size: 2,
            right_size: 2,
            kind,
        }
    }

    fn model(entries: &[(&str, i16, u16, &str)]) -> Model {
        let unknown = lexicon(&[("DEFAULT", 100, 0, "unknown")], 2);
        let unknown_values = vec![unknown.prefixes(b"DEFAULT").last().unwrap().1];
        Model(Arc::new(Data {
            category_names: Vec::new(),
            tokenizer: std::sync::OnceLock::new(),
            dictionaries: vec![lexicon(entries, 0)],
            unknown,
            #[cfg(test)]
            unknown_values,
            chars: vec![CharInfo(1 | (1 << 26)); 0xffff],
            matrix: vec![0; 4],
            left_size: 2,
            right_size: 2,
        }))
    }

    fn write_model(model: &Model) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        let data = &model.0;
        fs::write(
            directory.path().join("sys.dic"),
            &data.dictionaries[0].bytes,
        )
        .unwrap();
        fs::write(directory.path().join("unk.dic"), &data.unknown.bytes).unwrap();
        let mut chars = 1u32.to_le_bytes().to_vec();
        let mut name = [0; 32];
        name[..7].copy_from_slice(b"DEFAULT");
        chars.extend(name);
        for info in &data.chars {
            chars.extend(info.0.to_le_bytes());
        }
        fs::write(directory.path().join("char.bin"), chars).unwrap();
        let mut matrix = Vec::new();
        matrix.extend((data.left_size as u16).to_le_bytes());
        matrix.extend((data.right_size as u16).to_le_bytes());
        for cost in &data.matrix {
            matrix.extend(cost.to_le_bytes());
        }
        fs::write(directory.path().join("matrix.bin"), matrix).unwrap();
        directory
    }

    #[test]
    fn unknown_dictionary_requires_matching_kind_and_context_sizes() {
        let model = model(&[("a", 0, 0, "system")]);
        let directory = write_model(&model);
        Model::open(directory.path(), &[]).unwrap();
        for (offset, value) in [(8, 1u32), (16, 3u32), (20, 3u32)] {
            let mut bytes = model.0.unknown.bytes.clone();
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            fs::write(directory.path().join("unk.dic"), bytes).unwrap();
            assert_eq!(
                Model::open(directory.path(), &[]).unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn oversized_matrix_header_returns_an_error() {
        let model = model(&[("a", 0, 0, "system")]);
        let directory = write_model(&model);
        for (name, lexicon) in [
            ("sys.dic", &model.0.dictionaries[0]),
            ("unk.dic", &model.0.unknown),
        ] {
            let mut bytes = lexicon.bytes.clone();
            for offset in [16, 20] {
                bytes[offset..offset + 4].copy_from_slice(&65535u32.to_le_bytes());
            }
            fs::write(directory.path().join(name), bytes).unwrap();
        }
        fs::write(directory.path().join("matrix.bin"), [255; 4]).unwrap();
        assert_eq!(
            Model::open(directory.path(), &[]).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn partial_utf8_key_in_user_dictionary_returns_an_error() {
        let mut model = model(&[("a", 0, 0, "system")]);
        Arc::get_mut(&mut model.0).unwrap().chars[0x20] = CharInfo(2);
        let directory = write_model(&model);
        let mut user = lexicon(&[("a", 0, 0, "user")], 1);
        let from = 72 + (1 + b'a' as usize + 1) * 8;
        let to = 72 + (1 + "あ".as_bytes()[0] as usize + 1) * 8;
        user.bytes.copy_within(from..from + 8, to);
        user.bytes[from..from + 8].fill(0);
        let path = directory.path().join("user.dic");
        fs::write(&path, user.bytes).unwrap();
        let model = Model::open(directory.path(), &[path]).unwrap();
        assert_eq!(
            model.analyze_reference("あ").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn long_unknown_runs_remain_connected_to_eos() {
        let mut model = model(&[("a", 0, 0, "system")]);
        let data = Arc::get_mut(&mut model.0).unwrap();
        data.chars.fill(CharInfo(1 | (1 << 26) | (1 << 30)));
        data.chars[0x20] = CharInfo(2);
        let text = "x".repeat(100_000);
        let analysis = model.analyze_reference(&text).unwrap();
        let mut position = 0;
        for index in analysis.best_path {
            let node = &analysis.nodes[index];
            assert_eq!(node.byte_span.start, position);
            position = node.byte_span.end;
            assert!(node.is_unknown);
        }
        assert_eq!(position, text.len());
        assert_eq!(analysis.nodes.last().unwrap().byte_span.end, text.len());
    }

    #[test]
    fn ties_follow_mecab_predecessor_order() {
        let mut model = model(&[
            ("a", 0, 0, "first"),
            ("a", 0, 0, "second"),
            ("aa", 0, 0, "long"),
        ]);
        Arc::get_mut(&mut model.0).unwrap().chars[0x20] = CharInfo(2);
        let analysis = model.analyze_reference("aa").unwrap();
        let best: Vec<_> = analysis
            .best_path
            .iter()
            .map(|&i| &analysis.nodes[i].feature)
            .collect();
        assert_eq!(best, ["first", "first"]);
        assert!(analysis.nodes.iter().all(|node| node.delta == 0));
    }

    #[test]
    fn delta_includes_both_connections_and_the_word_cost() {
        let mut model = model(&[("a", 2, 1, "a"), ("b", 3, 1, "b"), ("ab", 13, 0, "ab")]);
        let data = Arc::get_mut(&mut model.0).unwrap();
        data.chars[0x20] = CharInfo(2);
        data.matrix = vec![0, 5, 7, -2];
        let analysis = model.analyze_reference("ab").unwrap();
        assert_eq!(analysis.total_cost, 13);
        let long = analysis.nodes.iter().find(|n| n.feature == "ab").unwrap();
        let short = analysis.nodes.iter().find(|n| n.feature == "a").unwrap();
        assert_eq!(long.delta, 0);
        assert_eq!(short.delta, 2);
        assert_eq!(model.transition_cost(0, 1), Some(7));
        assert_eq!(model.transition_cost(1, 0), Some(5));
        assert_eq!(model.transition_cost(2, 0), None);
    }

    #[test]
    fn unknown_grouping_respects_the_size_limit_and_dictionary_origin() {
        let mut model = model(&[("a", 0, 0, "system")]);
        let data = Arc::get_mut(&mut model.0).unwrap();
        data.chars
            .fill(CharInfo(1 | (1 << 26) | (1 << 30) | (1 << 31)));
        data.chars[0x20] = CharInfo(2);
        data.dictionaries.push(lexicon(&[("a", -1, 0, "user")], 1));
        let analysis = model.analyze_reference("a").unwrap();
        assert_eq!(analysis.nodes[analysis.best_path[0]].dictionary_index, 1);
        assert!(
            analysis
                .nodes
                .iter()
                .any(|node| node.is_unknown && node.dictionary_index == 255)
        );
        assert!(
            model
                .lookup(&"x".repeat(25), 0)
                .unwrap()
                .iter()
                .any(|n| n.byte_span.end == 25)
        );
        assert!(
            model
                .lookup(&"x".repeat(26), 0)
                .unwrap()
                .iter()
                .all(|n| n.byte_span.end != 26)
        );
    }

    #[test]
    fn supplementary_plane_characters_use_default_category() {
        let mut model = model(&[("a", 0, 0, "system")]);
        Arc::get_mut(&mut model.0).unwrap().chars[0x20] = CharInfo(2);
        let analysis = model.analyze_reference("𠮷").unwrap();
        assert_eq!(analysis.nodes[0].byte_span, 0..4);
        assert!(analysis.nodes[0].is_unknown);
    }
}

#[cfg(test)]
mod comparison_tests {
    use super::*;

    #[test]
    fn converted_dictionary_preserves_ties_users_and_unknowns() {
        use crate::mecab_compile::{BuildOptions, build_system, build_user};
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path();
        fs::write(src.join("matrix.def"), "1 1\n0 0 3\n").unwrap();
        fs::write(
            src.join("char.def"),
            "DEFAULT 1 0 1\nSPACE 0 1 0\nALPHA 1 1 2\n0x0020 SPACE\n0x0061..0x007A ALPHA\n",
        )
        .unwrap();
        fs::write(
            src.join("unk.def"),
            "DEFAULT,0,0,100,unknown-default\nSPACE,0,0,100,space\nALPHA,0,0,0,unknown-alpha\n",
        )
        .unwrap();
        fs::write(
            src.join("lex.csv"),
            "a,0,0,0,first\na,0,0,0,second\nb,0,0,-10,bee\n",
        )
        .unwrap();
        let output = src.join("compiled");
        build_system(src, &output, &BuildOptions::default()).unwrap();
        let mut users = Vec::new();
        for i in 1..=2 {
            let csv = src.join(format!("user{i}.csv"));
            fs::write(&csv, format!("a,0,0,0,user{i}\nx,0,0,-20,user{i}\n")).unwrap();
            let path = src.join(format!("user{i}.dic"));
            build_user(src, &[csv], &path).unwrap();
            users.push(path);
        }
        let model = Model::open(&output, &users).unwrap();
        let mut worker = model.worker().unwrap();
        for text in [
            "",
            "   ",
            "a",
            "b a ",
            "axb",
            "a?b",
            "𠮷",
            "aaaaaaaaaaaaaaaaaaaaaaaaaa",
        ] {
            let expected = model.analyze_reference(text).unwrap();
            let actual = worker.analyze_lattice(text).unwrap();
            assert_eq!(expected, actual, "{text:?}");
            let best = worker.analyze(text).unwrap();
            assert_eq!(
                best.nodes,
                expected
                    .best_path
                    .iter()
                    .map(|&i| expected.nodes[i].clone())
                    .collect::<Vec<_>>()
            );
            assert_eq!(best.total_cost, expected.total_cost);
        }
    }

    #[test]
    #[ignore = "HAQUMEI_TEST_DICTIONARY と HAQUMEI_TEST_SENTENCES の指定が必要です"]
    fn vibrato_matches_reference() {
        let directory = std::env::var_os("HAQUMEI_TEST_DICTIONARY").unwrap();
        let input =
            fs::read_to_string(std::env::var_os("HAQUMEI_TEST_SENTENCES").unwrap()).unwrap();
        let model = Model::open(Path::new(&directory), &[]).unwrap();
        let mut worker = model.worker().unwrap();
        let mut failures = Vec::new();
        let mut order_only = 0;
        let mut best_diff = 0;
        for (index, text) in input.lines().enumerate() {
            let expected = model.analyze_reference(text).unwrap();
            let actual = worker.analyze_lattice(text).unwrap();
            if expected != actual {
                let mut old_counts = std::collections::HashMap::new();
                let mut new_counts = std::collections::HashMap::new();
                for n in &expected.nodes {
                    *old_counts.entry(n).or_insert(0usize) += 1;
                }
                for n in &actual.nodes {
                    *new_counts.entry(n).or_insert(0usize) += 1;
                }
                let old_best: Vec<_> = expected
                    .best_path
                    .iter()
                    .map(|&i| &expected.nodes[i])
                    .collect();
                let new_best: Vec<_> = actual.best_path.iter().map(|&i| &actual.nodes[i]).collect();
                if old_best != new_best {
                    best_diff += 1;
                    eprintln!(
                        "best differs line {}\nold={old_best:?}\nnew={new_best:?}",
                        index + 1
                    );
                }
                if old_counts == new_counts && old_best == new_best {
                    order_only += 1;
                }

                if failures.len() < 5 {
                    eprintln!(
                        "line {}: nodes {} vs {}, costs {} vs {}",
                        index + 1,
                        expected.nodes.len(),
                        actual.nodes.len(),
                        expected.total_cost,
                        actual.total_cost
                    );
                    for (old, new) in expected.nodes.iter().zip(&actual.nodes) {
                        if old != new {
                            eprintln!("old={old:?}\nnew={new:?}");
                            break;
                        }
                    }
                }
                failures.push(index + 1);
            }
        }
        eprintln!("order_only={order_only}, best_diff={best_diff}");
        assert!(
            failures.is_empty(),
            "{} / {} sentences differ: {:?}",
            failures.len(),
            input.lines().count(),
            &failures[..failures.len().min(20)]
        );
    }
}
