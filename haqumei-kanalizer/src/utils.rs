#[inline(always)]
pub const fn get_ascii_index(c: char) -> Option<i64> {
    let lower_c = c.to_ascii_lowercase();
    if lower_c.is_ascii_lowercase() {
        Some((lower_c as u8 - b'a') as i64 + 2)
    } else {
        None
    }
}
