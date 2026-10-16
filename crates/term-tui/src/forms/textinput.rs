#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormAction {
    None,
    TextModified { ref_id: u32, char_inserted: Option<char> },
    Submit { ref_id: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextInputWidget {
    pub ref_id: u32,
    pub backend_node_id: u32,
    pub value: String,
    pub cursor_pos: usize,
    pub is_focused: bool,
    pub is_multiline: bool,
    pub placeholder: Option<String>,
}

impl TextInputWidget {
    pub fn new(ref_id: u32, backend_node_id: u32, placeholder: Option<String>) -> Self {
        Self {
            ref_id,
            backend_node_id,
            value: String::new(),
            cursor_pos: 0,
            is_focused: false,
            is_multiline: false,
            placeholder,
        }
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> FormAction {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char(c) => {
                self.value.insert(self.cursor_pos, c);
                self.cursor_pos += c.len_utf8();
                FormAction::TextModified {
                    ref_id: self.ref_id,
                    char_inserted: Some(c),
                }
            }
            KeyCode::Backspace if self.cursor_pos > 0 => {
                if let Some(ch) = self.value[..self.cursor_pos].chars().next_back() {
                    let ch_len = ch.len_utf8();
                    self.cursor_pos -= ch_len;
                    self.value.remove(self.cursor_pos);
                    FormAction::TextModified {
                        ref_id: self.ref_id,
                        char_inserted: None,
                    }
                } else {
                    FormAction::None
                }
            }
            KeyCode::Left if self.cursor_pos > 0 => {
                if let Some(ch) = self.value[..self.cursor_pos].chars().next_back() {
                    self.cursor_pos -= ch.len_utf8();
                }
                FormAction::None
            }
            KeyCode::Right if self.cursor_pos < self.value.len() => {
                if let Some(ch) = self.value[self.cursor_pos..].chars().next() {
                    self.cursor_pos += ch.len_utf8();
                }
                FormAction::None
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                FormAction::None
            }
            KeyCode::End => {
                self.cursor_pos = self.value.len();
                FormAction::None
            }
            KeyCode::Enter => {
                if self.is_multiline {
                    self.value.insert(self.cursor_pos, '\n');
                    self.cursor_pos += 1;
                    FormAction::TextModified {
                        ref_id: self.ref_id,
                        char_inserted: Some('\n'),
                    }
                } else {
                    FormAction::Submit { ref_id: self.ref_id }
                }
            }
            _ => FormAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_text_insertion() {
        let mut widget = TextInputWidget::new(10, 100, Some("Type here...".to_string()));
        assert_eq!(widget.value, "");
        assert_eq!(widget.cursor_pos, 0);

        let action = widget.handle_key(make_key(KeyCode::Char('H')));
        assert_eq!(action, FormAction::TextModified { ref_id: 10, char_inserted: Some('H') });
        assert_eq!(widget.value, "H");
        assert_eq!(widget.cursor_pos, 1);

        widget.handle_key(make_key(KeyCode::Char('i')));
        assert_eq!(widget.value, "Hi");
        assert_eq!(widget.cursor_pos, 2);
    }

    #[test]
    fn test_backspace() {
        let mut widget = TextInputWidget::new(10, 100, None);

        // Backspace on empty
        let action = widget.handle_key(make_key(KeyCode::Backspace));
        assert_eq!(action, FormAction::None);
        assert_eq!(widget.value, "");

        // Type "cat"
        widget.handle_key(make_key(KeyCode::Char('c')));
        widget.handle_key(make_key(KeyCode::Char('a')));
        widget.handle_key(make_key(KeyCode::Char('t')));
        assert_eq!(widget.value, "cat");
        assert_eq!(widget.cursor_pos, 3);

        // Backspace 't'
        let action = widget.handle_key(make_key(KeyCode::Backspace));
        assert_eq!(action, FormAction::TextModified { ref_id: 10, char_inserted: None });
        assert_eq!(widget.value, "ca");
        assert_eq!(widget.cursor_pos, 2);

        // Backspace 'a' and 'c'
        widget.handle_key(make_key(KeyCode::Backspace));
        assert_eq!(widget.value, "c");
        widget.handle_key(make_key(KeyCode::Backspace));
        assert_eq!(widget.value, "");
        assert_eq!(widget.cursor_pos, 0);

        // Extra backspace on empty
        let action = widget.handle_key(make_key(KeyCode::Backspace));
        assert_eq!(action, FormAction::None);
    }

    #[test]
    fn test_cursor_navigation_and_mid_insertion() {
        let mut widget = TextInputWidget::new(10, 100, None);
        for c in "hllo".chars() {
            widget.handle_key(make_key(KeyCode::Char(c)));
        }
        assert_eq!(widget.value, "hllo");
        assert_eq!(widget.cursor_pos, 4);

        // Move cursor left 3 times to between 'h' and 'l'
        widget.handle_key(make_key(KeyCode::Left));
        widget.handle_key(make_key(KeyCode::Left));
        widget.handle_key(make_key(KeyCode::Left));
        assert_eq!(widget.cursor_pos, 1);

        // Insert 'e'
        let action = widget.handle_key(make_key(KeyCode::Char('e')));
        assert_eq!(action, FormAction::TextModified { ref_id: 10, char_inserted: Some('e') });
        assert_eq!(widget.value, "hello");
        assert_eq!(widget.cursor_pos, 2);

        // Home and End
        widget.handle_key(make_key(KeyCode::Home));
        assert_eq!(widget.cursor_pos, 0);
        widget.handle_key(make_key(KeyCode::End));
        assert_eq!(widget.cursor_pos, 5);

        // Right arrow at end does not overflow
        widget.handle_key(make_key(KeyCode::Right));
        assert_eq!(widget.cursor_pos, 5);

        // Left arrow at beginning does not underflow
        widget.handle_key(make_key(KeyCode::Home));
        widget.handle_key(make_key(KeyCode::Left));
        assert_eq!(widget.cursor_pos, 0);
    }

    #[test]
    fn test_enter_submit_single_line() {
        let mut widget = TextInputWidget::new(5, 50, None);
        widget.is_multiline = false;

        for c in "search term".chars() {
            widget.handle_key(make_key(KeyCode::Char(c)));
        }

        let action = widget.handle_key(make_key(KeyCode::Enter));
        assert_eq!(action, FormAction::Submit { ref_id: 5 });
        assert_eq!(widget.value, "search term");
    }

    #[test]
    fn test_enter_multiline() {
        let mut widget = TextInputWidget::new(6, 60, None);
        widget.is_multiline = true;

        for c in "line1".chars() {
            widget.handle_key(make_key(KeyCode::Char(c)));
        }

        let action = widget.handle_key(make_key(KeyCode::Enter));
        assert_eq!(action, FormAction::TextModified { ref_id: 6, char_inserted: Some('\n') });
        assert_eq!(widget.value, "line1\n");
        assert_eq!(widget.cursor_pos, 6);

        for c in "line2".chars() {
            widget.handle_key(make_key(KeyCode::Char(c)));
        }
        assert_eq!(widget.value, "line1\nline2");
    }

    #[test]
    fn test_utf8_multibyte_safety() {
        let mut widget = TextInputWidget::new(7, 70, None);
        // Insert a 4-byte emoji 🦀
        widget.handle_key(make_key(KeyCode::Char('🦀')));
        assert_eq!(widget.value, "🦀");
        assert_eq!(widget.cursor_pos, 4);

        // Insert a 2-byte char é
        widget.handle_key(make_key(KeyCode::Char('é')));
        assert_eq!(widget.value, "🦀é");
        assert_eq!(widget.cursor_pos, 6);

        // Cursor left past 'é'
        widget.handle_key(make_key(KeyCode::Left));
        assert_eq!(widget.cursor_pos, 4);

        // Cursor left past '🦀'
        widget.handle_key(make_key(KeyCode::Left));
        assert_eq!(widget.cursor_pos, 0);

        // Cursor right past '🦀'
        widget.handle_key(make_key(KeyCode::Right));
        assert_eq!(widget.cursor_pos, 4);

        // Backspace '🦀'
        let action = widget.handle_key(make_key(KeyCode::Backspace));
        assert_eq!(action, FormAction::TextModified { ref_id: 7, char_inserted: None });
        assert_eq!(widget.value, "é");
        assert_eq!(widget.cursor_pos, 0);
    }
}
