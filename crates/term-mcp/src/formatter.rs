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
