#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormAction {
    None,
    TextModified { ref_id: u32, char_inserted: Option<char> },
    Submit { ref_id: u32 },
}

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
                self.cursor_pos += 1;
                FormAction::TextModified {
                    ref_id: self.ref_id,
                    char_inserted: Some(c),
                }
            }
            KeyCode::Backspace if self.cursor_pos > 0 => {
                self.cursor_pos -= 1;
                self.value.remove(self.cursor_pos);
                FormAction::TextModified {
                    ref_id: self.ref_id,
                    char_inserted: None,
                }
            }
            KeyCode::Left if self.cursor_pos > 0 => {
                self.cursor_pos -= 1;
                FormAction::None
            }
            KeyCode::Right if self.cursor_pos < self.value.len() => {
                self.cursor_pos += 1;
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
