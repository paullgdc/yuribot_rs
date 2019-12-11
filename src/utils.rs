pub fn utf8_pos_from_utf16(data: &str, pos: usize) -> Option<usize> {
    let mut pos_utf8 = 0_usize;
    let mut curr_pos_utf16 = 0_usize;
    for c in data.chars() {
        if curr_pos_utf16 == pos {
            return Some(pos_utf8);
        }
        curr_pos_utf16 += c.len_utf16();
        pos_utf8 += c.len_utf8();
    }
    if curr_pos_utf16 == pos {
        return Some(pos_utf8);
    }
    return None;
}
