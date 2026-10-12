//! Cursor positions stay on boundaries between complete displayed characters.
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Default)]
pub(crate) struct Cursor {
    pub(crate) byte: usize,
}
impl Cursor {
    pub(crate) fn end(&mut self, text: &str) {
        self.byte = text.len();
    }
    pub(crate) fn left(&mut self, text: &str) {
        self.byte = text
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .take_while(|i| *i < self.byte)
            .last()
            .unwrap_or(0);
    }
    pub(crate) fn right(&mut self, text: &str) {
        self.byte = text
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .find(|i| *i > self.byte)
            .unwrap_or(text.len());
    }
    fn settle(&mut self, text: &str) {
        // Insertion or deletion can join neighbours into a new grapheme.
        self.byte = text
            .grapheme_indices(true)
            .map(|(i, _)| i)
            .find(|i| *i >= self.byte)
            .unwrap_or(text.len());
    }
    pub(crate) fn insert(&mut self, text: &mut String, value: char) -> bool {
        if text.len() + value.len_utf8() > netscope_core::REQUEST_BODY_LIMIT {
            return false;
        }
        text.insert(self.byte, value);
        self.byte += value.len_utf8();
        self.settle(text);
        true
    }
    pub(crate) fn backspace(&mut self, text: &mut String) {
        let end = self.byte;
        self.left(text);
        text.replace_range(self.byte..end, "");
        self.settle(text);
    }
    pub(crate) fn delete(&mut self, text: &mut String) {
        let start = self.byte;
        self.right(text);
        text.replace_range(start..self.byte, "");
        self.byte = start;
        self.settle(text);
    }
    pub(crate) fn clear(&mut self, text: &mut String) {
        text.clear();
        self.byte = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn movement_and_deletion_follow_graphemes_not_bytes() {
        let mut text = "Ae\u{301}👩‍💻界Z".to_owned();
        let mut cursor = Cursor::default();
        cursor.end(&text);
        cursor.left(&text);
        cursor.left(&text);
        assert_eq!(&text[cursor.byte..], "界Z");
        cursor.backspace(&mut text);
        assert_eq!(text, "Ae\u{301}界Z");
        cursor.left(&text);
        assert_eq!(cursor.byte, 1);
        cursor.delete(&mut text);
        assert_eq!(text, "A界Z");
        assert!(cursor.insert(&mut text, 'é'));
        assert_eq!(text, "Aé界Z");
        cursor.byte = 0;
        cursor.backspace(&mut text);
        assert_eq!(text, "Aé界Z");
        cursor.end(&text);
        cursor.delete(&mut text);
        assert_eq!(text, "Aé界Z");
    }
    #[test]
    fn mutations_recompute_boundaries_when_neighbours_join() {
        let mut text = "👩💻".to_owned();
        let mut cursor = Cursor::default();
        cursor.right(&text);
        assert!(cursor.insert(&mut text, '\u{200d}'));
        assert_eq!(text, "👩‍💻");
        assert_eq!(cursor.byte, text.len());
        cursor.backspace(&mut text);
        assert!(text.is_empty());
        text = "🇦x🇧".into();
        cursor.byte = "🇦".len();
        cursor.delete(&mut text);
        assert_eq!(text, "🇦🇧");
        assert_eq!(cursor.byte, text.len());
        cursor.left(&text);
        assert_eq!(cursor.byte, 0);
    }
    #[test]
    fn insertion_limit_is_atomic_and_clear_resets_position() {
        let mut text = "a".repeat(netscope_core::REQUEST_BODY_LIMIT - 1);
        let mut cursor = Cursor { byte: 1 };
        assert!(!cursor.insert(&mut text, 'é'));
        assert_eq!(cursor.byte, 1);
        assert_eq!(text.len(), netscope_core::REQUEST_BODY_LIMIT - 1);
        assert!(cursor.insert(&mut text, '\n'));
        assert_eq!(cursor.byte, 2);
        assert!(!cursor.insert(&mut text, '\t'));
        cursor.clear(&mut text);
        assert_eq!(cursor.byte, 0);
        assert!(text.is_empty());
    }
}
