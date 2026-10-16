use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use term_tui::{FormAction, SyntheticDispatcher, TextInputWidget};

fn make_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn test_typing_workflow_generates_correct_actions_and_scripts() {
    let mut input = TextInputWidget::new(15, 302, Some("Search docs...".to_string()));
    assert_eq!(input.value, "");
    assert_eq!(input.cursor_pos, 0);

    // 1. Type "Rust"
    let mut actions = Vec::new();
    for ch in "Rust".chars() {
        let action = input.handle_key(make_key(KeyCode::Char(ch)));
        actions.push(action);
    }

    assert_eq!(
        actions,
        vec![
            FormAction::TextModified { ref_id: 15, char_inserted: Some('R') },
            FormAction::TextModified { ref_id: 15, char_inserted: Some('u') },
            FormAction::TextModified { ref_id: 15, char_inserted: Some('s') },
            FormAction::TextModified { ref_id: 15, char_inserted: Some('t') },
        ]
    );
    assert_eq!(input.value, "Rust");

    // 2. Dispatch synthetic change script for React
    let script = SyntheticDispatcher::build_react_change_script(input.backend_node_id, &input.value);
    assert!(script.contains("window.__term_resolve_node(302)"));
    assert!(script.contains(r#"nativeSetter.call(el, "Rust");"#));
    assert!(script.contains("dispatchEvent(new Event('input'"));
    assert!(script.contains("dispatchEvent(new Event('change'"));

    // 3. Type Backspace twice -> "Ru"
    let bs1 = input.handle_key(make_key(KeyCode::Backspace));
    let bs2 = input.handle_key(make_key(KeyCode::Backspace));
    assert_eq!(bs1, FormAction::TextModified { ref_id: 15, char_inserted: None });
    assert_eq!(bs2, FormAction::TextModified { ref_id: 15, char_inserted: None });
    assert_eq!(input.value, "Ru");

    // 4. Update React script with revised value
    let script2 = SyntheticDispatcher::build_react_change_script(input.backend_node_id, &input.value);
    assert!(script2.contains(r#"nativeSetter.call(el, "Ru");"#));

    // 5. Press Enter on single-line input -> FormAction::Submit
    let enter_action = input.handle_key(make_key(KeyCode::Enter));
    assert_eq!(enter_action, FormAction::Submit { ref_id: 15 });

    // 6. Generate submission script
    let submit_script = SyntheticDispatcher::build_react_submit_script(input.backend_node_id);
    assert!(submit_script.contains("window.__term_resolve_node(302)"));
    assert!(submit_script.contains("form.requestSubmit"));
}

#[test]
fn test_multiline_form_dispatcher_flow() {
    let mut textarea = TextInputWidget::new(20, 405, None);
    textarea.is_multiline = true;

    // Type line 1
    for ch in "hello".chars() {
        textarea.handle_key(make_key(KeyCode::Char(ch)));
    }

    // Press Enter in multiline textarea
    let enter_action = textarea.handle_key(make_key(KeyCode::Enter));
    assert_eq!(enter_action, FormAction::TextModified { ref_id: 20, char_inserted: Some('\n') });
    assert_eq!(textarea.value, "hello\n");

    // Type line 2
    for ch in "world".chars() {
        textarea.handle_key(make_key(KeyCode::Char(ch)));
    }
    assert_eq!(textarea.value, "hello\nworld");

    // React change script must escape newlines properly
    let script = SyntheticDispatcher::build_react_change_script(textarea.backend_node_id, &textarea.value);
    assert!(script.contains(r#"nativeSetter.call(el, "hello\nworld");"#));
    assert!(script.contains("HTMLTextAreaElement.prototype"));
}

#[test]
fn test_script_injection_and_quote_safety() {
    let mut input = TextInputWidget::new(99, 999, None);
    let malicious = r#"<script>alert('xss');</script> "test" `eval()` \n"#;
    for ch in malicious.chars() {
        input.handle_key(make_key(KeyCode::Char(ch)));
    }

    let script = SyntheticDispatcher::build_react_change_script(input.backend_node_id, &input.value);
    // Ensure properly escaped in JS literal
    assert!(script.contains(r#"nativeSetter.call(el, "<script>alert('xss');</script> \"test\" `eval()` \\n");"#));
}
