/// 単語 ID のエントリ、または辞書形式の識別子を返します。
pub fn get_word_data<'a>(idx: &[u8], data: &'a [u8], word_id: Option<usize>) -> Option<&'a [u8]> {
    let get_idx = |id: usize| -> Option<usize> {
        let start = id.checked_mul(4)?;
        let bytes = idx.get(start..start.checked_add(4)?)?;
        Some(u32::from_le_bytes(bytes.try_into().ok()?) as usize)
    };
    if !idx.len().is_multiple_of(4) {
        return None;
    }
    let id = word_id.unwrap_or(0);
    let start = get_idx(id)?;
    if word_id.is_none() {
        return data.get(..start);
    }
    let next = id.checked_add(1)?;
    let end = if next == idx.len() / 4 {
        data.len()
    } else {
        get_idx(next)?
    };
    data.get(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_indices_are_rejected() {
        assert_eq!(get_word_data(&[0; 4], b"x", Some(usize::MAX)), None);
        assert_eq!(get_word_data(&[0; 3], b"x", Some(0)), None);
        let idx = [3u32.to_le_bytes(), 1u32.to_le_bytes()].concat();
        assert_eq!(get_word_data(&idx, b"abcd", Some(0)), None);
        assert_eq!(get_word_data(&8u32.to_le_bytes(), b"abcd", None), None);
    }

    #[test]
    fn reads_header_and_final_entry() {
        let idx = [2u32.to_le_bytes(), 3u32.to_le_bytes()].concat();
        assert_eq!(get_word_data(&idx, b"idAB", None), Some(&b"id"[..]));
        assert_eq!(get_word_data(&idx, b"idAB", Some(0)), Some(&b"A"[..]));
        assert_eq!(get_word_data(&idx, b"idAB", Some(1)), Some(&b"B"[..]));
    }
}
