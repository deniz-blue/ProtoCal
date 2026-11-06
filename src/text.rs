pub fn utf16_length(text: &str) -> usize {
    text.encode_utf16().count()
}

pub fn cut_to_utf16_length(text: &str, longest: usize) -> &str {
    let mut length = 0;
    for (position, character) in text.char_indices() {
        length += character.len_utf16();
        if length > longest {
            return &text[..position];
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_cut_by_utf16_units_without_splitting_a_character() {
        assert_eq!(cut_to_utf16_length("abcdef", 3), "abc");
        assert_eq!(cut_to_utf16_length("ab", 3), "ab");
        assert_eq!(cut_to_utf16_length("a👋b", 2), "a");
        assert_eq!(utf16_length("a👋"), 3);
    }
}
