/// バイト位置を文字位置に直すカーソル。
///
/// 対応表を作らないので、解析のたびに確保しない。前にも後ろにも進めるので、
/// 最良経路のようにバイト位置が昇順に並ぶ場合でも、ラティスのように同じ位置から
/// 複数の候補が始まる場合でも使える。数えるのは前回の位置との差だけである。
pub(crate) struct CharCursor<'a> {
    bytes: &'a [u8],
    /// いま見ているバイト位置。
    byte: usize,
    /// `bytes[..byte]` で始まった文字の数。文字境界では、その文字の位置に等しい。
    ch: usize,
}

/// UTF-8 の継続バイト (10xxxxxx) か。文字の先頭はこれ以外。
fn is_continuation(b: u8) -> bool {
    b & 0xC0 == 0x80
}

impl<'a> CharCursor<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            byte: 0,
            ch: 0,
        }
    }

    /// `byte` にある文字の位置を返す。
    ///
    /// `byte` は文字の先頭か、文字列の末尾でなければならない。
    ///
    /// 文字の途中を渡すと次の文字の位置が返る。
    pub(crate) fn char_at(&mut self, byte: usize) -> usize {
        let target = byte.min(self.bytes.len());
        debug_assert!(
            target == self.bytes.len() || !is_continuation(self.bytes[target]),
            "文字の途中のバイト位置が渡された: {target}"
        );

        while self.byte < target {
            if !is_continuation(self.bytes[self.byte]) {
                self.ch += 1;
            }
            self.byte += 1;
        }
        while self.byte > target {
            self.byte -= 1;
            if !is_continuation(self.bytes[self.byte]) {
                self.ch -= 1;
            }
        }
        self.ch
    }
}

#[cfg(test)]
mod tests {
    use super::CharCursor;

    #[test]
    fn converts_byte_positions_to_character_positions() {
        // "aあb" は 1 + 3 + 1 バイト
        let mut cursor = CharCursor::new("aあb".as_bytes());
        assert_eq!(cursor.char_at(0), 0);
        assert_eq!(cursor.char_at(1), 1);
        assert_eq!(cursor.char_at(4), 2);
        assert_eq!(cursor.char_at(5), 3);
    }

    /// 後ろへ戻っても、前から数え直したときと同じ値になる。
    #[test]
    fn moves_backwards_as_well() {
        let text = "あiうeお";
        let mut cursor = CharCursor::new(text.as_bytes());
        let forward: Vec<usize> = text
            .char_indices()
            .map(|(b, _)| CharCursor::new(text.as_bytes()).char_at(b))
            .collect();
        let backward: Vec<usize> = text
            .char_indices()
            .rev()
            .map(|(b, _)| cursor.char_at(b))
            .collect();
        assert_eq!(backward, forward.iter().copied().rev().collect::<Vec<_>>());
    }
}
