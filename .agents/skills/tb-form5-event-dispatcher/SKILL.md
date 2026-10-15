---
name: tb-form5-event-dispatcher
description: >-
  FORM-5 Form & Event Dispatch Engineer for terminal-browser. Activate when implementing
  or debugging terminal form widgets (TextInput, SelectDropdown, Checkbox, Radio, FileUpload),
  synthetic CDP keyboard/mouse event dispatching (Input.dispatchKeyEvent, Input.insertText),
  and React/SPA onChange/onInput compatibility.
---

# FORM-5: Form & Event Dispatch Engineer

## Role
Specialist engineer owning interactive form widgets within the TUI and translating user terminal
interactions into faithful synthetic Chrome DevTools Protocol (CDP) input events.

## Blueprint Sources
- [missing-blueprints.md](../../../missing-blueprints.md): §3 Interactive Form & Synthetic Event Dispatch Blueprint
- [PLATFORM_ARCHITECTURE_AND_IMPLEMENTATION_BLUEPRINTS.md](../../../PLATFORM_ARCHITECTURE_AND_IMPLEMENTATION_BLUEPRINTS.md): §3 Unified Event Loop

## Module Location
`crates/term-tui/src/forms/`
```
crates/term-tui/src/forms/
├── mod.rs                 # FormManager & active widget focus state
├── textinput.rs           # Single-line & multiline terminal text input
├── dropdown.rs            # Floating popover fuzzy-picker for <select>
├── toggle.rs              # Checkbox and Radio terminal toggles
├── file_upload.rs         # Path prompt & DOM.setFileInputFiles handler
└── dispatcher.rs          # Crossterm KeyEvent -> CDP Input event synthesizer
```

---

## 1. Terminal TextInput Widget (textinput.rs)

Supports focus state, cursor navigation, insertion, Backspace/Delete, and multiline mode:

```rust
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
```

---

## 2. Synthetic CDP Event Synthesizer (dispatcher.rs)

Translates terminal key presses into exact CDP `Input.dispatchKeyEvent` structures.
Modern React / Vue / Angular applications do not respond to simple attribute value changes—they
require true keyboard events and `input`/`change` event bubbling:

```rust
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchKeyEventParams, DispatchKeyEventType,
};

pub struct SyntheticDispatcher;

impl SyntheticDispatcher {
    pub fn to_cdp_key_event(key: crossterm::event::KeyEvent) -> DispatchKeyEventParams {
        use crossterm::event::KeyCode;
        let (key_str, code_str, text_opt) = match key.code {
            KeyCode::Char(c) => (c.to_string(), format!("Key{}", c.to_uppercase()), Some(c.to_string())),
            KeyCode::Enter => ("Enter".to_string(), "Enter".to_string(), Some("\r".to_string())),
            KeyCode::Backspace => ("Backspace".to_string(), "Backspace".to_string(), None),
            KeyCode::Tab => ("Tab".to_string(), "Tab".to_string(), None),
            KeyCode::Esc => ("Escape".to_string(), "Escape".to_string(), None),
            _ => ("Unidentified".to_string(), "Unidentified".to_string(), None),
        };

        let mut params = DispatchKeyEventParams::new(DispatchKeyEventType::KeyDown);
        params.key = Some(key_str);
        params.code = Some(code_str);
        params.text = text_opt;
        params
    }

    /// Dispatches full React-compatible input change sequence via Runtime.evaluate
    pub fn build_react_change_script(backend_node_id: u32, new_value: &str) -> String {
        format!(
            r#"(() => {{
                const el = window.__term_resolve_node({backend_node_id});
                if (!el) return false;
                const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
                const nativeSetter = Object.getOwnPropertyDescriptor(proto, 'value').set;
                nativeSetter.call(el, {value:?});
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }})()"#,
            backend_node_id = backend_node_id,
            value = new_value
        )
    }
}
```

---

## 3. Dropdown Popover Picker (dropdown.rs)

When a `<select>` element receives focus or `Enter`:
1. Renders a floating modal popover overlaying the current terminal viewport.
2. Supports fuzzy filtering (typing narrows down options).
3. Selecting an option triggers CDP synthetic click or value change:

```rust
pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub is_selected: bool,
}

pub struct DropdownPopover {
    pub ref_id: u32,
    pub options: Vec<SelectOption>,
    pub selected_idx: usize,
    pub filter_query: String,
}
```

---

## 4. File Upload Handler (file_upload.rs)

When interacting with `<input type="file">` (e.g. `[button #9 "Choose File"]`):
1. Opens an inline path prompt in the terminal status bar: `Enter file path to upload: ~/document.pdf`.
2. Calls CDP `DOM.setFileInputFiles` with absolute file path:
```rust
pub async fn handle_file_upload(
    page: &chromiumoxide::Page,
    backend_node_id: u32,
    file_path: &std::path::Path,
) -> anyhow::Result<()> {
    // Translates path into CDP DOM.setFileInputFiles
    let abs_path = std::fs::canonicalize(file_path)?;
    // dispatch via chromiumoxide DOM API
    Ok(())
}
```

## Validation
```bash
cargo check -p term-tui --lib
cargo test -p term-tui -- forms
# Test interactive form widgets in terminal:
cargo run -p term-tui --example form_widgets_demo
```
