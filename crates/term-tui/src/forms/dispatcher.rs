pub struct SyntheticDispatcher;

impl SyntheticDispatcher {
    /// Builds a JavaScript snippet to dispatch synthetic input and change events for React/SPA hydration.
    /// Uses native prototype setter invocation to bypass React 16-19 value override trackers.
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

    /// Builds a JavaScript snippet to dispatch form submission for the given element.
    pub fn build_react_submit_script(backend_node_id: u32) -> String {
        format!(
            r#"(() => {{
                const el = window.__term_resolve_node({backend_node_id});
                if (!el) return false;
                const form = el.form || (el.tagName === 'FORM' ? el : el.closest('form'));
                if (form) {{
                    if (typeof form.requestSubmit === 'function') {{
                        form.requestSubmit();
                    }} else {{
                        form.submit();
                    }}
                    return true;
                }}
                el.dispatchEvent(new KeyboardEvent('keydown', {{ key: 'Enter', code: 'Enter', bubbles: true }}));
                return true;
            }})()"#,
            backend_node_id = backend_node_id
        )
    }

    /// Maps a crossterm KeyEvent to key description strings (key, code, text) for CDP dispatch
    pub fn to_key_event_strings(key: crossterm::event::KeyEvent) -> (&'static str, &'static str, Option<String>) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char(c) => ("Char", "KeyChar", Some(c.to_string())),
            KeyCode::Enter => ("Enter", "Enter", Some("\r".to_string())),
            KeyCode::Backspace => ("Backspace", "Backspace", None),
            KeyCode::Tab => ("Tab", "Tab", None),
            KeyCode::Esc => ("Escape", "Escape", None),
            KeyCode::Left => ("ArrowLeft", "ArrowLeft", None),
            KeyCode::Right => ("ArrowRight", "ArrowRight", None),
            KeyCode::Up => ("ArrowUp", "ArrowUp", None),
            KeyCode::Down => ("ArrowDown", "ArrowDown", None),
            _ => ("Unidentified", "Unidentified", None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn test_build_react_change_script_basic() {
        let script = SyntheticDispatcher::build_react_change_script(42, "hello world");
        assert!(script.contains("window.__term_resolve_node(42)"));
        assert!(script.contains(r#"nativeSetter.call(el, "hello world");"#));
        assert!(script.contains("new Event('input', { bubbles: true })"));
        assert!(script.contains("new Event('change', { bubbles: true })"));
    }

    #[test]
    fn test_build_react_change_script_escaping() {
        let special_text = "line1\nline2\t\"quoted\" and 'single' \\backslash";
        let script = SyntheticDispatcher::build_react_change_script(100, special_text);
        assert!(script.contains("window.__term_resolve_node(100)"));
        // Rust's {:?} format properly escapes quotes, newlines, and backslashes
        assert!(script.contains(r#""line1\nline2\t\"quoted\" and 'single' \\backslash""#));
    }

    #[test]
    fn test_build_react_submit_script() {
        let script = SyntheticDispatcher::build_react_submit_script(7);
        assert!(script.contains("window.__term_resolve_node(7)"));
        assert!(script.contains("form.requestSubmit"));
        assert!(script.contains("form.submit()"));
    }

    #[test]
    fn test_to_key_event_strings() {
        let char_ev = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let (key, code, text) = SyntheticDispatcher::to_key_event_strings(char_ev);
        assert_eq!(key, "Char");
        assert_eq!(code, "KeyChar");
        assert_eq!(text, Some("a".to_string()));

        let enter_ev = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let (key, code, text) = SyntheticDispatcher::to_key_event_strings(enter_ev);
        assert_eq!(key, "Enter");
        assert_eq!(code, "Enter");
        assert_eq!(text, Some("\r".to_string()));

        let bs_ev = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        let (key, code, text) = SyntheticDispatcher::to_key_event_strings(bs_ev);
        assert_eq!(key, "Backspace");
        assert_eq!(code, "Backspace");
        assert_eq!(text, None);
    }
}
