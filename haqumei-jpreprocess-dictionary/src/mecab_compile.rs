//! MeCab 互換バイナリ辞書を Rust で構築します。

use encoding_rs::{EUC_JP, Encoding, SHIFT_JIS, UTF_8};

use std::{
    borrow::Cow,
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
};

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn number<T: std::str::FromStr>(s: &str) -> io::Result<T> {
    s.parse()
        .map_err(|_| invalid(format!("数値が不正です: {s}")))
}

#[derive(Clone)]
struct Charset {
    name: String,
    encoding: &'static Encoding,
}

impl Charset {
    fn new(name: &str) -> io::Result<Self> {
        let encoding = match name.to_ascii_lowercase().as_str() {
            "utf-8" | "utf8" | "utf_8" => UTF_8,
            "euc-jp" | "euc_jp" | "euc" => EUC_JP,
            "shift_jis" | "shift-jis" | "sjis" | "cp932" | "windows-31j" => SHIFT_JIS,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("対応していない辞書の文字コードです: {name}"),
                ));
            }
        };
        Ok(Self {
            name: name.to_owned(),
            encoding,
        })
    }

    fn decode<'a>(&self, bytes: &'a [u8]) -> io::Result<Cow<'a, str>> {
        self.encoding
            .decode_without_bom_handling_and_without_replacement(bytes)
            .ok_or_else(|| invalid(format!("{} として解釈できない文字があります", self.name)))
    }

    fn encode<'a>(&self, value: &'a str) -> io::Result<Cow<'a, [u8]>> {
        let (bytes, _, malformed) = self.encoding.encode(value);
        if malformed {
            Err(invalid(format!(
                "{} に変換できない文字があります",
                self.name
            )))
        } else {
            Ok(bytes)
        }
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        Ok(self.decode(&fs::read(path)?)?.into_owned())
    }

    fn header(&self) -> [u8; 32] {
        let mut header = [0; 32];
        header[..self.name.len()].copy_from_slice(self.name.as_bytes());
        header
    }
}

struct Charsets {
    input: Charset,
    output: Charset,
    config: Charset,
}

impl Charsets {
    fn new(directory: &Path, input: &str, output: &str) -> io::Result<Self> {
        let input = Charset::new(input)?;
        let output = Charset::new(output)?;
        let config = match fs::read(directory.join("dicrc")) {
            Ok(bytes) => String::from_utf8_lossy(&bytes)
                .lines()
                .filter_map(|line| line.split_once('='))
                .find(|(name, _)| name.trim() == "config-charset")
                .map(|(_, value)| Charset::new(value.trim()))
                .transpose()?
                .unwrap_or_else(|| input.clone()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => input.clone(),
            Err(error) => return Err(error),
        };
        Ok(Self {
            input,
            output,
            config,
        })
    }
}

/// 構築する辞書ファイルを指定します。すべて false なら一式を構築します。
#[derive(Default)]
pub struct BuildOptions {
    /// 未知語辞書と文字種を構築します。
    pub unknown: bool,
    /// 文字種を構築します。
    pub charcategory: bool,
    /// システム辞書を構築します。
    pub sysdic: bool,
    /// 接続行列を構築します。
    pub matrix: bool,
    /// model.def がある場合に model.bin を構築します。
    pub model: bool,
}

/// CSV と定義ファイルから MeCab 互換のシステム辞書を構築します。
pub fn build_system(input: &Path, output: &Path, options: &BuildOptions) -> io::Result<()> {
    build_system_with_charsets(input, output, options, "utf-8", "utf-8")
}

/// 入力 CSV と出力辞書の文字コードを指定して、システム辞書を構築します。
/// UTF-8、EUC-JP、Shift_JIS と、それぞれの別名を受け付けます。
pub fn build_system_with_charsets(
    input: &Path,
    output: &Path,
    options: &BuildOptions,
    dictionary_charset: &str,
    charset: &str,
) -> io::Result<()> {
    let charsets = Charsets::new(input, dictionary_charset, charset)?;
    fs::create_dir_all(output)?;
    let all = !(options.unknown
        || options.charcategory
        || options.sysdic
        || options.matrix
        || options.model);
    let sizes = if all || options.unknown || options.sysdic || options.matrix {
        matrix_sizes(input)?
    } else {
        (1, 1)
    };
    if all || options.charcategory || options.unknown {
        build_characters(input, &output.join("char.bin"), &charsets.config)?;
    }
    if all || options.matrix {
        build_matrix(input, &output.join("matrix.bin"), sizes)?;
    }
    if (all || options.model) && input.join("model.def").is_file() {
        fs::write(
            output.join("model.bin"),
            encode_model(
                &fs::read(input.join("model.def"))?,
                Some(&charsets.input),
                &charsets.output,
            )?,
        )?;
    }
    if all || options.unknown {
        build_lexicon(
            input,
            &[input.join("unk.def")],
            &output.join("unk.dic"),
            2,
            sizes,
            None,
            &charsets,
        )?;
    }
    if all || options.sysdic {
        let mut files = fs::read_dir(input)?
            .map(|e| e.map(|e| e.path()))
            .collect::<io::Result<Vec<_>>>()?;
        files.retain(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("csv"))
        });
        files.sort();
        build_lexicon(
            input,
            &files,
            &output.join("sys.dic"),
            0,
            sizes,
            None,
            &charsets,
        )?;
    }
    if input.join("dicrc").is_file() && input != output {
        fs::copy(input.join("dicrc"), output.join("dicrc"))?;
    }
    Ok(())
}

/// 指定した CSV から MeCab 互換のユーザー辞書を構築します。
pub fn build_user(input: &Path, files: &[PathBuf], output: &Path) -> io::Result<()> {
    build_user_with_model(input, files, output, None)
}

/// 指定した学習モデルから、コストが空欄のユーザー辞書エントリのコストを推定します。
pub fn build_user_with_model(
    input: &Path,
    files: &[PathBuf],
    output: &Path,
    model: Option<&Path>,
) -> io::Result<()> {
    build_user_with_model_and_charsets(input, files, output, model, "utf-8", "utf-8")
}

/// 入力 CSV と出力辞書の文字コードを指定して、ユーザー辞書を構築します。
/// コストが空欄のエントリは、指定した学習モデルから推定します。
pub fn build_user_with_model_and_charsets(
    input: &Path,
    files: &[PathBuf],
    output: &Path,
    model: Option<&Path>,
    dictionary_charset: &str,
    charset: &str,
) -> io::Result<()> {
    let charsets = Charsets::new(input, dictionary_charset, charset)?;
    build_lexicon(
        input,
        files,
        output,
        1,
        matrix_sizes(input)?,
        model,
        &charsets,
    )
}

fn matrix_sizes(input: &Path) -> io::Result<(u16, u16)> {
    if input.join("matrix.def").is_file() {
        let text = fs::read_to_string(input.join("matrix.def"))?;
        let mut fields = text.split_whitespace();
        Ok((
            number(
                fields
                    .next()
                    .ok_or_else(|| invalid("matrix.def が空です"))?,
            )?,
            number(
                fields
                    .next()
                    .ok_or_else(|| invalid("matrix.def の次元がありません"))?,
            )?,
        ))
    } else {
        let bytes = fs::read(input.join("matrix.bin"))?;
        let header = bytes
            .get(..4)
            .ok_or_else(|| invalid("matrix.bin が短すぎます"))?;
        Ok((
            u16::from_le_bytes([header[0], header[1]]),
            u16::from_le_bytes([header[2], header[3]]),
        ))
    }
}

fn build_matrix(input: &Path, output: &Path, (left, right): (u16, u16)) -> io::Result<()> {
    if !input.join("matrix.def").is_file() {
        fs::copy(input.join("matrix.bin"), output)?;
        return Ok(());
    }
    let text = fs::read_to_string(input.join("matrix.def"))?;
    let mut values = vec![0i16; usize::from(left) * usize::from(right)];
    for line in text.lines().skip(1).filter(|l| !l.trim().is_empty()) {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() != 3 {
            return Err(invalid(format!("接続行列の行が不正です: {line}")));
        }
        let l = number::<usize>(fields[0])?;
        let r = number::<usize>(fields[1])?;
        if l >= left as usize || r >= right as usize {
            return Err(invalid("接続 ID が行列の範囲外です"));
        }
        values[l + left as usize * r] = number::<i32>(fields[2])? as i16;
    }
    let mut bytes = Vec::with_capacity(4 + values.len() * 2);
    bytes.extend(left.to_le_bytes());
    bytes.extend(right.to_le_bytes());
    for value in values {
        bytes.extend(value.to_le_bytes());
    }
    fs::write(output, bytes)
}

fn build_characters(input: &Path, output: &Path, charset: &Charset) -> io::Result<()> {
    fs::write(output, character_bytes(input, charset)?)
}

fn character_bytes(input: &Path, charset: &Charset) -> io::Result<Vec<u8>> {
    let text = charset.read(&input.join("char.def"))?;
    let mut categories = Vec::<(String, u32)>::new();
    let mut ranges = Vec::new();
    for line in text.lines() {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields[0].starts_with("0x") {
            ranges.push(fields);
        } else {
            if fields.len() != 4
                || categories.len() >= 17
                || fields[0].len() >= 32
                || categories.iter().any(|(name, _)| name == fields[0])
            {
                return Err(invalid(format!("文字種の定義が不正です: {line}")));
            }
            let invoke = number::<u32>(fields[1])?;
            let group = number::<u32>(fields[2])?;
            let length = number::<u32>(fields[3])?;
            if invoke > 1 || group > 1 || length > 15 {
                return Err(invalid("文字種の値が範囲外です"));
            }
            let index = categories.len() as u32;
            categories.push((
                fields[0].to_owned(),
                (index << 18) | (length << 26) | (group << 30) | (invoke << 31),
            ));
        }
    }
    let encode = |names: &[&str]| -> io::Result<u32> {
        let first = names.first().ok_or_else(|| invalid("文字種がありません"))?;
        let mut bits = categories
            .iter()
            .find(|(name, _)| name == first)
            .ok_or_else(|| invalid(format!("未定義の文字種: {first}")))?
            .1;
        for name in names {
            let idx = categories
                .iter()
                .position(|(n, _)| n == name)
                .ok_or_else(|| invalid(format!("未定義の文字種: {name}")))?;
            bits |= 1 << idx;
        }
        Ok(bits)
    };
    let mut table = vec![encode(&["DEFAULT"])?; 0xffff];
    encode(&["SPACE"])?;
    for fields in ranges {
        let (low, high) = fields[0].split_once("..").unwrap_or((fields[0], fields[0]));
        let parse = |s: &str| {
            usize::from_str_radix(s.trim_start_matches("0x"), 16)
                .map_err(|_| invalid("文字範囲が不正です"))
        };
        let (low, high) = (parse(low)?, parse(high)?);
        if low > high || high >= table.len() {
            return Err(invalid("文字範囲が範囲外です"));
        }
        table[low..=high].fill(encode(&fields[1..])?);
    }
    let mut bytes = Vec::new();
    bytes.extend((categories.len() as u32).to_le_bytes());
    for (name, _) in categories {
        let mut field = [0; 32];
        let encoded = charset.encode(&name)?;
        if encoded.len() >= field.len() {
            return Err(invalid("文字種の名前が長すぎます"));
        }
        field[..encoded.len()].copy_from_slice(&encoded);
        bytes.extend(field);
    }
    for value in table {
        bytes.extend(value.to_le_bytes());
    }
    Ok(bytes)
}

fn matches_pattern(pattern: &str, fields: &[&str]) -> bool {
    let pattern: Vec<_> = pattern.split(',').collect();
    pattern.len() <= fields.len()
        && pattern.iter().zip(fields).all(|(p, v)| {
            p.starts_with('*')
                || *p == *v
                || p.strip_prefix('(')
                    .and_then(|s| s.strip_suffix(')'))
                    .is_some_and(|s| s.split('|').any(|alternative| alternative == *v))
        })
}

fn escape_field(value: &str) -> String {
    if value.contains([',', '"']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn expand_rewrite(template: &str, fields: &[&str]) -> io::Result<String> {
    let mut result = String::new();
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            result.push(ch);
            continue;
        }
        let mut digits = String::new();
        while chars.peek().is_some_and(char::is_ascii_digit) {
            digits.push(chars.next().unwrap());
        }
        let index = number::<usize>(&digits)?
            .checked_sub(1)
            .ok_or_else(|| invalid("書き換え列は 1 から指定します"))?;
        result.push_str(
            fields
                .get(index)
                .ok_or_else(|| invalid("書き換え規則が存在しない列を参照しています"))?,
        );
    }
    Ok(escape_field(&result))
}

fn rewrite_feature(
    input: &Path,
    feature: &[&str],
    side: &str,
    charset: &Charset,
) -> io::Result<String> {
    let rewrite = charset.read(&input.join("rewrite.def"))?;
    let mut section = false;
    for line in rewrite
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        if line.starts_with('[') {
            section = line == format!("[{side} rewrite]");
            continue;
        }
        if !section {
            continue;
        }
        let Some((pattern, replacement)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        if matches_pattern(pattern, feature) {
            return replacement
                .trim()
                .split(',')
                .map(|value| expand_rewrite(value, feature))
                .collect::<io::Result<Vec<_>>>()
                .map(|fields| fields.join(","));
        }
    }
    Err(invalid("文脈 ID の書き換え規則がありません"))
}

fn context_id(input: &Path, feature: &[&str], side: &str, charset: &Charset) -> io::Result<u16> {
    let key = rewrite_feature(input, feature, side, charset)?;
    for line in charset.read(&input.join(format!("{side}-id.def")))?.lines() {
        if let Some((id, name)) = line.split_once(char::is_whitespace)
            && name.trim() == key
        {
            return number(id);
        }
    }
    Err(invalid(format!("文脈 ID がありません: {key}")))
}

fn entry_columns(bytes: &[u8]) -> io::Result<(Vec<Vec<u8>>, &[u8])> {
    let mut offset = 0;
    let mut columns = Vec::new();
    for _ in 0..4 {
        while bytes.get(offset).is_some_and(|b| *b == b' ' || *b == b'\t') {
            offset += 1;
        }
        let mut value = Vec::new();
        if bytes.get(offset) == Some(&b'"') {
            offset += 1;
            while let Some(&byte) = bytes.get(offset) {
                offset += 1;
                if byte == b'"' {
                    if bytes.get(offset) != Some(&b'"') {
                        break;
                    }
                    offset += 1;
                }
                value.push(byte);
            }
            while bytes.get(offset).is_some_and(|b| *b != b',') {
                offset += 1;
            }
        } else {
            while let Some(&byte) = bytes.get(offset).filter(|b| **b != b',') {
                value.push(byte);
                offset += 1;
            }
        }
        if bytes.get(offset) != Some(&b',') {
            return Err(invalid("CSV の列が足りません"));
        }
        offset += 1;
        columns.push(value);
    }
    while bytes.get(offset).is_some_and(|b| *b == b' ' || *b == b'\t') {
        offset += 1;
    }
    let feature = &bytes[offset..];
    if feature.is_empty() {
        return Err(invalid("CSV の素性がありません"));
    }
    Ok((columns, feature))
}

fn build_lexicon(
    input: &Path,
    files: &[PathBuf],
    output: &Path,
    kind: u32,
    (left, right): (u16, u16),
    model_path: Option<&Path>,
    charsets: &Charsets,
) -> io::Result<()> {
    let pos_text = match charsets.config.read(&input.join("pos-id.def")) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => "* 1".to_owned(),
        Err(error) => return Err(error),
    };
    let pos: Vec<_> = pos_text
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some((fields.next()?, fields.next()?.parse::<u16>().ok()?))
        })
        .collect();
    let mut entries = Vec::<(Vec<u8>, [u8; 16])>::new();
    let mut features = Vec::new();
    let mut ids = HashMap::new();
    let mut model = None;
    let mut skipped = 0;
    for path in files {
        let bytes = fs::read(path)?;
        for line in bytes.split(|byte| *byte == b'\n') {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if line.is_empty() {
                continue;
            }
            let parsed = (|| -> io::Result<(Vec<u8>, [u8; 16], Vec<u8>)> {
                let (raw_record, raw_feature) = entry_columns(line)?;
                let record = raw_record
                    .iter()
                    .map(|value| charsets.input.decode(value))
                    .collect::<io::Result<Vec<_>>>()?;
                let feature = charsets.input.decode(raw_feature)?;
                let feature = feature.as_ref();
                let surface = record[0].as_ref();
                if surface.is_empty() || surface.contains('\0') || feature.contains('\0') {
                    return Err(invalid("辞書のエントリが空か NUL を含んでいます"));
                }
                let parsed = csv::ReaderBuilder::new()
                    .has_headers(false)
                    .flexible(true)
                    .from_reader(feature.as_bytes())
                    .records()
                    .next()
                    .ok_or_else(|| invalid("素性がありません"))??;
                let fields: Vec<_> = parsed.iter().collect();
                let parse_id = |s: &str| {
                    if s.is_empty() {
                        Ok(-1)
                    } else {
                        number::<i32>(s)
                    }
                };
                let (mut lid, mut rid) = (parse_id(&record[1])?, parse_id(&record[2])?);
                if lid < 0 || rid < 0 || lid == i32::MAX || rid == i32::MAX {
                    if let Some(&(l, r)) = ids.get(feature) {
                        lid = l;
                        rid = r;
                    } else {
                        lid = i32::from(context_id(input, &fields, "left", &charsets.config)?);
                        rid = i32::from(context_id(input, &fields, "right", &charsets.config)?);
                        ids.insert(feature.to_owned(), (lid, rid));
                    }
                }
                if lid >= i32::from(right) || rid >= i32::from(left) {
                    return Err(invalid("辞書の文脈 ID が行列の範囲外です"));
                }
                let cost = if record[3].is_empty() || record[3] == i32::MAX.to_string() {
                    if kind != 1 {
                        return Err(invalid("システム辞書と未知語辞書のコストは省略できません"));
                    }
                    if model.is_none() {
                        model = Some(CostModel::open(input, model_path, charsets)?);
                    }
                    i32::from(model.as_ref().unwrap().cost(
                        input,
                        surface,
                        &fields,
                        &charsets.config,
                    )?)
                } else {
                    number::<i32>(&record[3])?
                };
                let pid = pos
                    .iter()
                    .find(|(p, _)| matches_pattern(p, &fields))
                    .map_or(u16::MAX, |(_, id)| *id);
                let offset =
                    u32::try_from(features.len()).map_err(|_| invalid("辞書が大きすぎます"))?;
                let mut token = [0; 16];
                token[0..2].copy_from_slice(&(lid as u16).to_le_bytes());
                token[2..4].copy_from_slice(&(rid as u16).to_le_bytes());
                token[4..6].copy_from_slice(&pid.to_le_bytes());
                token[6..8].copy_from_slice(&(cost as i16).to_le_bytes());
                token[8..12].copy_from_slice(&offset.to_le_bytes());
                // 同じ文字コードでも再符号化すると、Shift_JIS の重複符号が変わることがある。
                let (surface, feature) = if charsets.input.encoding == charsets.output.encoding {
                    (raw_record[0].clone(), raw_feature.to_vec())
                } else {
                    (
                        charsets.output.encode(surface)?.into_owned(),
                        charsets.output.encode(feature)?.into_owned(),
                    )
                };
                Ok((surface, token, feature))
            })();
            match parsed {
                Ok((surface, token, feature)) => {
                    entries.push((surface, token));
                    features.extend(feature);
                    features.push(0);
                }
                Err(error) if error.kind() != io::ErrorKind::InvalidData => return Err(error),
                Err(_) => skipped += 1,
            }
        }
    }
    if skipped > 0 {
        eprintln!("辞書の不正なエントリを {skipped} 件除外しました");
    }
    if entries.is_empty() {
        return Err(invalid("辞書のエントリがありません"));
    }
    // 同じ表層形の同点候補は格納順で勝敗が決まるため、CSV 内の順序を保つ。
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut keys = Vec::new();
    let mut start = 0;
    while start < entries.len() {
        let mut end = start + 1;
        while end < entries.len() && entries[end].0 == entries[start].0 {
            end += 1;
        }
        if end - start > 255 || start > 0x7fffff {
            return Err(invalid("MeCab の辞書形式の上限を超えています"));
        }
        keys.push((
            entries[start].0.as_slice(),
            ((start as u32) << 8) | (end - start) as u32,
        ));
        start = end;
    }
    let trie = DoubleArray::build(&keys);
    let size = 72usize + trie.len() * 8 + entries.len() * 16 + features.len();
    let size = u32::try_from(size).map_err(|_| invalid("辞書が大きすぎます"))?;
    let mut bytes = Vec::with_capacity(size as usize);
    for value in [
        size ^ 0xef718f77,
        102,
        kind,
        entries.len() as u32,
        left as u32,
        right as u32,
        trie.len() as u32 * 8,
        entries.len() as u32 * 16,
        features.len() as u32,
        0,
    ] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend(charsets.output.header());
    for (base, check) in trie {
        bytes.extend(base.to_le_bytes());
        bytes.extend(check.to_le_bytes());
    }
    for (_, token) in entries {
        bytes.extend(token);
    }
    bytes.extend(features);
    fs::write(output, bytes)
}

struct DoubleArray {
    units: Vec<(i32, u32)>,
    used: Vec<bool>,
    next: usize,
    size: usize,
}

impl DoubleArray {
    fn build(keys: &[(&[u8], u32)]) -> Vec<(i32, u32)> {
        let mut trie = Self {
            units: vec![(0, 0); 1024],
            used: vec![false; 1024],
            next: 0,
            size: 1,
        };
        let root = trie.insert(keys, 0);
        trie.units[0].0 = root as i32;
        trie.units.resize(trie.size + 257, (0, 0));
        trie.units
    }

    fn reserve(&mut self, index: usize) {
        if index >= self.units.len() {
            let length = (index + 1).max(self.units.len() * 2);
            self.units.resize(length, (0, 0));
            self.used.resize(length, false);
        }
    }

    fn insert(&mut self, keys: &[(&[u8], u32)], depth: usize) -> usize {
        let mut groups = Vec::new();
        let mut start = 0;
        while start < keys.len() {
            let code = keys[start].0.get(depth).map_or(0, |b| *b as usize + 1);
            let mut end = start + 1;
            while end < keys.len() && keys[end].0.get(depth).map_or(0, |b| *b as usize + 1) == code
            {
                end += 1;
            }
            groups.push((code, start, end));
            start = end;
        }
        let first = groups[0].0;
        let last = groups.last().unwrap().0;
        let mut pos = (first + 1).max(self.next);
        let mut first_free = None;
        let mut occupied = 0;
        let begin = loop {
            self.reserve(pos);
            if self.units[pos].1 != 0 {
                occupied += 1;
                pos += 1;
                continue;
            }
            first_free.get_or_insert(pos);
            let begin = pos - first;
            self.reserve(begin + last);
            if !self.used[begin]
                && groups
                    .iter()
                    .all(|(code, _, _)| self.units[begin + code].1 == 0)
            {
                break begin;
            }
            pos += 1;
        };
        self.next = first_free.unwrap();
        if occupied as f64 / (pos - self.next + 1) as f64 >= 0.95 {
            self.next = pos;
        }
        self.used[begin] = true;
        self.size = self.size.max(begin + last + 1);
        for (code, _, _) in &groups {
            self.units[begin + code].1 = begin as u32;
        }
        for (code, start, end) in groups {
            let base = if code == 0 {
                -(keys[start].1 as i32) - 1
            } else {
                self.insert(&keys[start..end], depth + 1) as i32
            };
            self.units[begin + code].0 = base;
        }
        begin
    }
}

/// 学習モデルを UTF-8 の MeCab 形式の model.bin に変換します。
pub fn build_model(input: &Path, output: &Path) -> io::Result<()> {
    fs::write(
        output,
        encode_model(&fs::read(input)?, None, &Charset::new("utf-8")?)?,
    )
}

fn fingerprint(bytes: &[u8]) -> u64 {
    let constants = [0x239b961bu32, 0xab0e9789, 0x38b34ae5, 0xa1e38b93];
    let mut h = [0xfd14deffu32; 4];
    let rotations = [19, 17, 15, 13];
    let additions = [0x561ccd1b, 0x0bcaa747, 0x96cd1c35, 0x32ac3b17];
    let (blocks, remainder) = bytes.as_chunks::<16>();
    for block in blocks {
        for index in 0..4 {
            let key = u32::from_le_bytes(block[4 * index..4 * index + 4].try_into().unwrap());
            h[index] ^= key
                .wrapping_mul(constants[index])
                .rotate_left(15 + index as u32)
                .wrapping_mul(constants[(index + 1) % 4]);
            h[index] = h[index]
                .rotate_left(rotations[index])
                .wrapping_add(h[(index + 1) % 4])
                .wrapping_mul(5)
                .wrapping_add(additions[index]);
        }
    }
    for (index, tail) in remainder.chunks(4).enumerate() {
        let mut bytes = [0; 4];
        bytes[..tail.len()].copy_from_slice(tail);
        h[index] ^= u32::from_le_bytes(bytes)
            .wrapping_mul(constants[index])
            .rotate_left(15 + index as u32)
            .wrapping_mul(constants[(index + 1) % 4]);
    }
    for value in &mut h {
        *value ^= bytes.len() as u32;
    }
    h[0] = h.iter().fold(0u32, |sum, value| sum.wrapping_add(*value));
    for index in 1..4 {
        h[index] = h[index].wrapping_add(h[0]);
    }
    for value in &mut h {
        *value ^= *value >> 16;
        *value = value.wrapping_mul(0x85ebca6b);
        *value ^= *value >> 13;
        *value = value.wrapping_mul(0xc2b2ae35);
        *value ^= *value >> 16;
    }
    h[0] = h.iter().fold(0u32, |sum, value| sum.wrapping_add(*value));
    h[1] = h[1].wrapping_add(h[0]);
    u64::from(h[0]) | (u64::from(h[1]) << 32)
}

fn encode_model(
    bytes: &[u8],
    expected_input: Option<&Charset>,
    output: &Charset,
) -> io::Result<Vec<u8>> {
    let mut lines = bytes.split(|byte| *byte == b'\n');
    let mut charset = None;
    for line in lines.by_ref() {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            break;
        }
        let line = std::str::from_utf8(line).map_err(|_| invalid("モデルのヘッダーが不正です"))?;
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| invalid("モデルのヘッダーが不正です"))?;
        if name == "charset" {
            charset = Some(Charset::new(value.trim())?);
        }
    }
    let input = charset.ok_or_else(|| invalid("学習モデルに文字コードの指定がありません"))?;
    if expected_input.is_some_and(|expected| expected.encoding != input.encoding) {
        return Err(invalid(
            "入力 CSV と学習モデルの文字コードが一致していません",
        ));
    }
    let mut entries = Vec::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let separator = line
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or_else(|| invalid("モデルの特徴量の行が不正です"))?;
        let weight = number::<f64>(
            std::str::from_utf8(&line[..separator])
                .map_err(|_| invalid("モデルの重みが不正です"))?,
        )?;
        if !weight.is_finite() {
            return Err(invalid("モデルの重みが有限値ではありません"));
        }
        let feature = &line[separator + 1..];
        let key = if input.encoding == output.encoding {
            fingerprint(feature)
        } else {
            fingerprint(&output.encode(&input.decode(feature)?)?)
        };
        entries.push((key, weight));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
    let mut bytes = Vec::new();
    bytes.extend(
        u32::try_from(entries.len())
            .map_err(|_| invalid("モデルが大きすぎます"))?
            .to_le_bytes(),
    );
    bytes.extend(output.header());
    for (_, weight) in &entries {
        bytes.extend(weight.to_le_bytes());
    }
    for (key, _) in entries {
        bytes.extend(key.to_le_bytes());
    }
    Ok(bytes)
}

struct CostModel {
    weights: Vec<(u64, f64)>,
    templates: Vec<String>,
    categories: Vec<u8>,
    charset: Charset,
}

impl CostModel {
    fn open(input: &Path, model: Option<&Path>, charsets: &Charsets) -> io::Result<Self> {
        let model = model
            .map(Path::to_path_buf)
            .unwrap_or_else(|| input.join("model.bin"));
        let mut bytes = fs::read(model)?;
        let length = |bytes: &[u8]| {
            bytes
                .get(..4)
                .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
        };
        if length(&bytes).and_then(|n| n.checked_mul(16)?.checked_add(36)) != Some(bytes.len()) {
            bytes = encode_model(&bytes, Some(&charsets.input), &charsets.output)?;
        }
        let count = length(&bytes).ok_or_else(|| invalid("モデルのヘッダーがありません"))?;
        let charset = &bytes[4..36];
        let end = charset
            .iter()
            .position(|b| *b == 0)
            .unwrap_or(charset.len());
        let charset = Charset::new(
            std::str::from_utf8(&charset[..end])
                .map_err(|_| invalid("モデルの文字コード名が不正です"))?,
        )?;
        if charset.encoding != charsets.output.encoding {
            return Err(invalid("モデルと出力辞書の文字コードが一致していません"));
        }
        let mut weights = Vec::with_capacity(count);
        for i in 0..count {
            let weight = f64::from_le_bytes(bytes[36 + 8 * i..44 + 8 * i].try_into().unwrap());
            let start = 36 + 8 * count + 8 * i;
            let key = u64::from_le_bytes(bytes[start..start + 8].try_into().unwrap());
            if !weight.is_finite() {
                return Err(invalid("モデルの重みが有限値ではありません"));
            }
            weights.push((key, weight));
        }
        if weights.windows(2).any(|pair| pair[0].0 > pair[1].0) {
            return Err(invalid("モデルの特徴量がハッシュ順ではありません"));
        }
        let template_bytes = fs::read(input.join("feature.def"))?;
        // 元の定義ファイルには UTF-8 以外のコメントがある。特徴量の定義行だけを読む。
        let templates = template_bytes
            .split(|byte| *byte == b'\n')
            .filter_map(|line| line.strip_prefix(b"UNIGRAM"))
            .map(|line| {
                charsets
                    .config
                    .decode(line)
                    .map(|line| line.trim().to_owned())
            })
            .collect::<io::Result<Vec<_>>>()?;
        let chars = if input.join("char.bin").is_file() {
            fs::read(input.join("char.bin"))?
        } else {
            character_bytes(input, &charsets.config)?
        };
        let names = chars
            .get(..4)
            .map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize)
            .ok_or_else(|| invalid("文字種のヘッダーがありません"))?;
        let start = names
            .checked_mul(32)
            .and_then(|n| n.checked_add(4))
            .ok_or_else(|| invalid("文字種が大きすぎます"))?;
        if start.checked_add(4 * 0xffff) != Some(chars.len()) {
            return Err(invalid("文字種のファイル長が不正です"));
        }
        let categories = chars[start..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| ((u32::from_le_bytes(*b) >> 18) & 0xff) as u8)
            .collect();
        Ok(Self {
            weights,
            templates,
            categories,
            charset,
        })
    }

    fn cost(
        &self,
        input: &Path,
        surface: &str,
        fields: &[&str],
        config: &Charset,
    ) -> io::Result<i16> {
        let rewritten = rewrite_feature(input, fields, "unigram", config)?;
        let record = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(rewritten.as_bytes())
            .records()
            .next()
            .ok_or_else(|| invalid("単語素性がありません"))??;
        let code = surface.chars().next().map_or(0, |ch| ch as usize);
        let category = self.categories[if code < 0xffff { code } else { 0 }];
        let mut sum = 0.0;
        for template in &self.templates {
            let Some(feature) = expand_template(template, &record, &rewritten, category)? else {
                continue;
            };
            let key = fingerprint(&self.charset.encode(feature.split('\0').next().unwrap())?);
            let index = self.weights.partition_point(|(value, _)| *value < key);
            if let Some(&(value, weight)) = self.weights.get(index)
                && value == key
            {
                sum += weight;
            }
        }
        Ok((-800.0 * sum).clamp(-32767.0, 32767.0) as i16)
    }
}

fn expand_template(
    template: &str,
    fields: &csv::StringRecord,
    unigram: &str,
    category: u8,
) -> io::Result<Option<String>> {
    let mut output = String::new();
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => output.push(
                match chars
                    .next()
                    .ok_or_else(|| invalid("エスケープ文字がありません"))?
                {
                    '0' => '\0',
                    'a' => '\x07',
                    'b' => '\x08',
                    't' => '\t',
                    'n' => '\n',
                    'v' => '\x0b',
                    'f' => '\x0c',
                    'r' => '\r',
                    ch => ch,
                },
            ),
            '%' => match chars
                .next()
                .ok_or_else(|| invalid("特徴量の参照先がありません"))?
            {
                't' => output.push_str(&category.to_string()),
                'u' => output.push_str(unigram),
                'w' => {}
                'F' => {
                    let optional = if chars.peek() == Some(&'?') {
                        chars.next();
                        true
                    } else {
                        false
                    };
                    if chars.next() != Some('[') {
                        return Err(invalid("特徴量の添字が不正です"));
                    }
                    let mut digits = String::new();
                    while chars.peek().is_some_and(char::is_ascii_digit) {
                        digits.push(chars.next().unwrap());
                    }
                    if chars.next() != Some(']') {
                        return Err(invalid("特徴量の添字が閉じていません"));
                    }
                    let index = number::<usize>(&digits)?;
                    let Some(value) = fields.get(index) else {
                        return Ok(None);
                    };
                    if optional && (value.is_empty() || value == "*") {
                        return Ok(None);
                    }
                    output.push_str(value);
                }
                _ => return Err(invalid("未知の特徴量テンプレートです")),
            },
            ch => output.push(ch),
        }
    }
    Ok(Some(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "haqumei-mecab-compiler-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
        fn write(&self, name: &str, contents: &str) {
            fs::write(self.0.join(name), contents).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn model_fingerprints_match_mecab() {
        for (input, expected) in [
            ("U1:名詞", 0x8743602d30b27cde),
            ("U2:名詞,一般", 0x5b85934d881e13a3),
            ("q", 0x5f0822cbf0afe0a0),
            ("qqqqqqq", 0xdecd19715dbdaf60),
            ("qqqqqqqqqqqqqqq", 0x0854e03bef083bc7),
            ("qqqqqqqqqqqqqqqq", 0xa7eddeed6c6bfcb1),
            ("qqqqqqqqqqqqqqqqq", 0xa34d2f26c87384ca),
            ("qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq", 0x0073651f0dd4c276),
            ("qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq", 0x8a5f469457f0a1b9),
            ("qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq", 0xe4d71bfa90272f78),
        ] {
            assert_eq!(fingerprint(input.as_bytes()), expected, "{input}");
        }
    }

    #[test]
    fn csv_keeps_quoted_features_and_decodes_the_surface() {
        let (fields, feature) =
            entry_columns("\"仮,名\",1,1,2,名詞,一般,*,*,*,*,\"仮,名\",カナ".as_bytes()).unwrap();
        assert_eq!(
            fields,
            ["仮,名", "1", "1", "2"].map(|value| value.as_bytes().to_vec())
        );
        assert_eq!(feature, "名詞,一般,*,*,*,*,\"仮,名\",カナ".as_bytes());
        assert_eq!(
            expand_rewrite("prefix-$2/$1", &["a", "b,c"]).unwrap(),
            "\"prefix-b,c/a\""
        );
        assert!(expand_rewrite("$3", &["a", "b"]).is_err());
    }

    fn source() -> Fixture {
        let fixture = Fixture::new();
        fixture.write("matrix.def", "2 2\n0 0 0\n0 1 5\n1 0 7\n1 1 -3\n");
        fixture.write("char.def", "DEFAULT 0 1 1\nSPACE 0 1 0\n0x0020 SPACE\n");
        fixture.write(
            "unk.def",
            "DEFAULT,0,0,100,名詞,一般,*,*,*,*,*\nSPACE,0,0,100,記号,空白,*,*,*,*,*\n",
        );
        fixture.write("rewrite.def", "[unigram rewrite]\n* $1,$2,$3,$4,$5,$6,$7\n[left rewrite]\n* $1,$2,$3,$4,$5,$6,*\n[right rewrite]\n* $1,$2,$3,$4,$5,$6,*\n");
        fixture.write(
            "left-id.def",
            "0 BOS/EOS,*,*,*,*,*,*\n1 名詞,一般,*,*,*,*,*\n",
        );
        fixture.write(
            "right-id.def",
            "0 BOS/EOS,*,*,*,*,*,*\n1 名詞,一般,*,*,*,*,*\n",
        );
        fixture.write("feature.def", "UNIGRAM U:%F[0]\nUNIGRAM missing:%F?[2]\n");
        fixture.write("model.def", "charset: utf-8\n\n1.25\tU:名詞\n");
        fixture.write("words.csv", "a,1,1,100,名詞,一般,*,*,*,*,a\n");
        fixture
    }

    #[test]
    fn user_costs_and_context_ids_match_for_text_and_binary_models() {
        let input = source();
        let output = Fixture::new();
        build_system(&input.0, &output.0, &BuildOptions::default()).unwrap();
        let user = input.0.join("user.csv");
        fs::write(&user, "a,,0,,名詞,一般,*,*,*,*,a\ninvalid\n").unwrap();
        let text = output.0.join("text.dic");
        let binary = output.0.join("binary.dic");
        build_user_with_model(
            &input.0,
            std::slice::from_ref(&user),
            &text,
            Some(&input.0.join("model.def")),
        )
        .unwrap();
        build_user_with_model(
            &input.0,
            &[user],
            &binary,
            Some(&output.0.join("model.bin")),
        )
        .unwrap();
        assert_eq!(fs::read(&text).unwrap(), fs::read(&binary).unwrap());
        let model = crate::mecab::Model::open(&output.0, &[text]).unwrap();
        let analysis = model.analyze("a").unwrap();
        let node = &analysis.nodes[analysis.best_path[0]];
        assert_eq!(
            (node.left_id, node.right_id, node.word_cost, node.pos_id),
            (1, 1, -1000, 1)
        );
        assert_eq!(node.dictionary_index, 1);
    }

    #[test]
    fn unknown_only_build_also_produces_characters() {
        let input = source();
        let output = Fixture::new();
        build_system(
            &input.0,
            &output.0,
            &BuildOptions {
                unknown: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(output.0.join("char.bin").is_file());
        assert!(output.0.join("unk.dic").is_file());
        assert!(!output.0.join("matrix.bin").exists());
    }

    #[test]
    fn model_only_build_does_not_require_a_dictionary() {
        let input = Fixture::new();
        let output = Fixture::new();
        let options = BuildOptions {
            model: true,
            ..Default::default()
        };
        build_system(&input.0, &output.0, &options).unwrap();
        assert!(!output.0.join("model.bin").exists());
        input.write("model.def", "charset: utf-8\n\n1.0\tU1:名詞\n");
        build_system(&input.0, &output.0, &options).unwrap();
        assert_eq!(fs::metadata(output.0.join("model.bin")).unwrap().len(), 52);
    }
    fn encoded_source(charset: &str) -> Fixture {
        let input = source();
        input.write("dicrc", "cost-factor = 800\n");
        input.write("pos-id.def", "名詞 4\n記号 2\n* 1\n");
        input.write("words.csv", "表,1,1,110,名詞,一般,*,*,*,*,表\nあ,1,1,120,名詞,一般,*,*,*,*,あ\n音,1,1,130,名詞,一般,*,*,*,*,音\n声,1,1,140,名詞,一般,*,*,*,*,声\n\"仮,名\",1,1,150,名詞,一般,*,*,*,*,\"仮,名\"\n");
        input.write("user.txt", "仮名,,0,,名詞,一般,*,*,*,*,仮名\n");
        input.write(
            "model.def",
            &format!("charset: {charset}\n\n1.25\tU:名詞\n"),
        );
        let encoding = Charset::new(charset).unwrap();
        for entry in fs::read_dir(&input.0).unwrap() {
            let path = entry.unwrap().path();
            let text = fs::read_to_string(&path).unwrap();
            fs::write(path, encoding.encode(&text).unwrap()).unwrap();
        }
        input
    }

    fn checksum(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
    }

    #[test]
    fn all_charset_pairs_match_mecab_dictionary_and_model_bytes() {
        // C のコンパイラで各文字コードの入力と出力を揃えて作った辞書と比較する。
        let expected = [
            (
                "utf-8",
                [
                    0x2199b7d333bc03f9,
                    0xf1e807a11861b3e8,
                    0x4186be172367dbef,
                    0xe5d4a1e5af88dd28,
                ],
            ),
            (
                "EUC-JP",
                [
                    0x055d5af8c37cff4f,
                    0x1ffa04cf05d52dd9,
                    0x363a132f4a3c1ad7,
                    0xefda5e09bcbeecdd,
                ],
            ),
            (
                "Shift_JIS",
                [
                    0x0960cbb3a4fb660b,
                    0x4a7d19b45fea98aa,
                    0xe7d6fff1b4807ed9,
                    0x4d76fdc824d394bc,
                ],
            ),
        ];
        for (from, _) in expected {
            let input = encoded_source(from);
            for (to, hashes) in expected {
                let output = Fixture::new();
                build_system_with_charsets(&input.0, &output.0, &BuildOptions::default(), from, to)
                    .unwrap();
                for model in [input.0.join("model.def"), output.0.join("model.bin")] {
                    build_user_with_model_and_charsets(
                        &input.0,
                        &[input.0.join("user.txt")],
                        &output.0.join("user.dic"),
                        Some(&model),
                        from,
                        to,
                    )
                    .unwrap();
                    for (file, hash) in ["sys.dic", "unk.dic", "model.bin", "user.dic"]
                        .into_iter()
                        .zip(hashes)
                    {
                        assert_eq!(
                            checksum(&fs::read(output.0.join(file)).unwrap()),
                            hash,
                            "{from} -> {to}: {file}"
                        );
                    }
                }
                assert_eq!(
                    checksum(&fs::read(output.0.join("char.bin")).unwrap()),
                    0x4f67f89e8525a404
                );
                assert_eq!(
                    checksum(&fs::read(output.0.join("matrix.bin")).unwrap()),
                    0x94a03aa4ae95f2b3
                );
            }
        }
    }

    #[test]
    fn definition_files_can_use_a_different_charset_from_the_csv() {
        let input = encoded_source("EUC-JP");
        for file in [
            "pos-id.def",
            "rewrite.def",
            "left-id.def",
            "right-id.def",
            "feature.def",
        ] {
            let path = input.0.join(file);
            let text = Charset::new("EUC-JP").unwrap().read(&path).unwrap();
            fs::write(path, text).unwrap();
        }
        input.write("dicrc", "config-charset = UTF-8\ncost-factor = 800\n");
        input.write(
            "char.def",
            "# 文字種の定義\nDEFAULT 0 1 1\nSPACE 0 1 0\n0x0020 SPACE\n",
        );
        let output = Fixture::new();
        build_user_with_model_and_charsets(
            &input.0,
            &[input.0.join("user.txt")],
            &output.0.join("user.dic"),
            Some(&input.0.join("model.def")),
            "EUC-JP",
            "utf-8",
        )
        .unwrap();
        assert_eq!(
            checksum(&fs::read(output.0.join("user.dic")).unwrap()),
            0xe5d4a1e5af88dd28
        );
    }

    #[test]
    fn unmappable_entries_are_skipped_without_character_replacement() {
        let input = encoded_source("utf-8");
        input.write("words.csv", "仮名,1,1,100,名詞,一般,*,*,*,*,仮名\n\u{1f600},1,1,100,名詞,一般,*,*,*,*,仮名\n音,1,1,100,名詞,一般,*,*,*,*,\u{1f600}\n");
        let output = Fixture::new();
        let options = BuildOptions {
            sysdic: true,
            ..Default::default()
        };
        build_system_with_charsets(&input.0, &output.0, &options, "utf-8", "Shift_JIS").unwrap();
        let bytes = fs::read(output.0.join("sys.dic")).unwrap();
        assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 1);
        assert_eq!(&bytes[40..49], b"Shift_JIS");
        assert!(Charset::new("UTF-16").is_err());
        assert_eq!(Charset::new("euc").unwrap().encoding, EUC_JP);
        assert_eq!(Charset::new("CP932").unwrap().encoding, SHIFT_JIS);
        assert_eq!(Charset::new("utf_8").unwrap().encoding, UTF_8);
    }
}
