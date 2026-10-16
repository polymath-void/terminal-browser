use term_core::{AXNode, PageSnapshot};

/// Converts a PageSnapshot into a compact token-efficient markdown representation with stable Ref IDs
pub fn format_snapshot_markdown(snapshot: &PageSnapshot) -> String {
    let mut buf = String::new();
    let scroll_pct = (snapshot.scroll_percent * 100.0) as u8;
    buf.push_str(&format!(
        "<!-- URL: {} | Status: {} | Scroll: {}% -->\n",
        snapshot.url, snapshot.http_status, scroll_pct
    ));
    for node in &snapshot.nodes {
        format_node(&mut buf, node, 0);
    }
    buf
}

fn format_node(buf: &mut String, node: &AXNode, depth: usize) {
    let indent = "  ".repeat(depth);
    let ref_str = node.ref_id.map(|id| format!(" #{}", id)).unwrap_or_default();
    let name_str = node
        .name
        .as_deref()
        .map(|n| format!(" \"{}\"", n))
        .unwrap_or_default();
    let val_str = node
        .value
        .as_deref()
        .filter(|v| !v.is_empty())
        .map(|v| format!(" value=\"{}\"", v))
        .unwrap_or_default();

    match node.role.as_str() {
        "link" => buf.push_str(&format!(
            "{}[link{}{} href=\"{}\"]\n",
            indent,
            ref_str,
            name_str,
            node.href.as_deref().unwrap_or("#")
        )),
        "button" => buf.push_str(&format!("{}[button{}{}]\n", indent, ref_str, name_str)),
        "textbox" => buf.push_str(&format!("{}[textbox{}{}{}]\n", indent, ref_str, name_str, val_str)),
        role => buf.push_str(&format!("{}[{}{}]\n", indent, role, name_str)),
    }

    for child in &node.children {
        format_node(buf, child, depth + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use term_core::{SnapshotMode, SnapshotSource};

    #[test]
    fn test_format_snapshot_markdown_token_efficiency_and_ref_ids() {
        let snapshot = PageSnapshot {
            url: "https://example.com/login".to_string(),
            title: Some("Login Page".to_string()),
            http_status: 200,
            mode: SnapshotMode::Semantic,
            scroll_percent: 0.25,
            source: SnapshotSource::DirectNegotiation,
            nodes: vec![
                AXNode {
                    ref_id: None,
                    backend_node_id: Some(1),
                    role: "banner".to_string(),
                    name: Some("Header".to_string()),
                    value: None,
                    placeholder: None,
                    checked: None,
                    expanded: None,
                    level: None,
                    href: None,
                    children: vec![
                        AXNode {
                            ref_id: Some(1),
                            backend_node_id: Some(2),
                            role: "link".to_string(),
                            name: Some("Home".to_string()),
                            value: None,
                            placeholder: None,
                            checked: None,
                            expanded: None,
                            level: None,
                            href: Some("https://example.com".to_string()),
                            children: vec![],
                        },
                    ],
                },
                AXNode {
                    ref_id: None,
                    backend_node_id: Some(3),
                    role: "main".to_string(),
                    name: None,
                    value: None,
                    placeholder: None,
                    checked: None,
                    expanded: None,
                    level: None,
                    href: None,
                    children: vec![
                        AXNode {
                            ref_id: Some(2),
                            backend_node_id: Some(4),
                            role: "textbox".to_string(),
                            name: Some("Username".to_string()),
                            value: Some("alice".to_string()),
                            placeholder: Some("Enter username".to_string()),
                            checked: None,
                            expanded: None,
                            level: None,
                            href: None,
                            children: vec![],
                        },
                        AXNode {
                            ref_id: Some(3),
                            backend_node_id: Some(5),
                            role: "button".to_string(),
                            name: Some("Sign In".to_string()),
                            value: None,
                            placeholder: None,
                            checked: None,
                            expanded: None,
                            level: None,
                            href: None,
                            children: vec![],
                        },
                    ],
                },
            ],
        };

        let markdown = format_snapshot_markdown(&snapshot);
        
        // Check header with URL, HTTP status and Scroll percent
        assert!(markdown.starts_with("<!-- URL: https://example.com/login | Status: 200 | Scroll: 25% -->\n"));

        // Check indentation and structure
        assert!(markdown.contains("[banner \"Header\"]\n"));
        assert!(markdown.contains("  [link #1 \"Home\" href=\"https://example.com\"]\n"));
        assert!(markdown.contains("[main]\n"));
        assert!(markdown.contains("  [textbox #2 \"Username\" value=\"alice\"]\n"));
        assert!(markdown.contains("  [button #3 \"Sign In\"]\n"));

        // Verify token compactness: output is concise with no HTML or JSON noise
        assert!(!markdown.contains("<div>"));
        assert!(!markdown.contains("\"backend_node_id\""));
        assert!(markdown.lines().count() <= 6);
    }
}
