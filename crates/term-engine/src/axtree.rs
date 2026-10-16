use term_core::{AXNode, RefMap};

pub const INTERACTIVE_ROLES: &[&str] = &[
    "button", "link", "textbox", "combobox", "checkbox", "radio",
    "switch", "slider", "menuitem", "tab", "searchbox",
];

pub const STRUCTURAL_ROLES: &[&str] = &[
    "banner", "navigation", "main", "contentinfo", "heading",
    "table", "row", "cell", "list", "listitem", "dialog", "alert",
];

/// Prunes an Accessibility Node tree, filtering noise and registering interactive Ref IDs
pub fn prune_axtree(mut root: AXNode, ref_map: &mut RefMap) -> Option<AXNode> {
    let is_interactive = INTERACTIVE_ROLES.contains(&root.role.as_str());
    let is_structural = STRUCTURAL_ROLES.contains(&root.role.as_str());

    if is_interactive {
        let backend_id = root.backend_node_id.unwrap_or(0);
        let assigned_id = ref_map.assign(backend_id, root.role.clone(), root.name.clone());
        root.ref_id = Some(assigned_id);
    }

    // Recursively prune children
    let mut pruned_children = Vec::new();
    for child in root.children {
        if let Some(pruned_child) = prune_axtree(child, ref_map) {
            pruned_children.push(pruned_child);
        }
    }
    root.children = pruned_children;

    if is_interactive || is_structural || !root.children.is_empty() || root.name.is_some() {
        Some(root)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prune_filters_empty_passive_nodes() {
        let mut ref_map = RefMap::new();
        let passive_node = AXNode {
            ref_id: None,
            backend_node_id: Some(999),
            role: "none".to_string(),
            name: None,
            value: None,
            placeholder: None,
            checked: None,
            expanded: None,
            level: None,
            href: None,
            children: vec![],
        };
        let result = prune_axtree(passive_node, &mut ref_map);
        assert!(result.is_none());
        assert_eq!(ref_map.len(), 0);
    }

    #[test]
    fn test_prune_assigns_sequential_ref_ids() {
        let mut ref_map = RefMap::new();
        let tree = AXNode {
            ref_id: None,
            backend_node_id: Some(1),
            role: "main".to_string(),
            name: Some("Main Area".to_string()),
            value: None,
            placeholder: None,
            checked: None,
            expanded: None,
            level: None,
            href: None,
            children: vec![
                AXNode {
                    ref_id: None,
                    backend_node_id: Some(10),
                    role: "textbox".to_string(),
                    name: Some("Search Box".to_string()),
                    value: None,
                    placeholder: Some("Search...".to_string()),
                    checked: None,
                    expanded: None,
                    level: None,
                    href: None,
                    children: vec![],
                },
                AXNode {
                    ref_id: None,
                    backend_node_id: Some(20),
                    role: "button".to_string(),
                    name: Some("Submit".to_string()),
                    value: None,
                    placeholder: None,
                    checked: None,
                    expanded: None,
                    level: None,
                    href: None,
                    children: vec![],
                },
                AXNode {
                    ref_id: None,
                    backend_node_id: Some(30),
                    role: "link".to_string(),
                    name: Some("Read More".to_string()),
                    value: None,
                    placeholder: None,
                    checked: None,
                    expanded: None,
                    level: None,
                    href: Some("https://news.ycombinator.com".to_string()),
                    children: vec![],
                },
            ],
        };

        let pruned = prune_axtree(tree, &mut ref_map).expect("Should retain main");
        assert_eq!(pruned.role, "main");
        assert_eq!(pruned.children.len(), 3);
        assert_eq!(pruned.children[0].ref_id, Some(1));
        assert_eq!(pruned.children[1].ref_id, Some(2));
        assert_eq!(pruned.children[2].ref_id, Some(3));
        assert_eq!(ref_map.len(), 3);

        let handle1 = ref_map.resolve(1).unwrap();
        assert_eq!(handle1.role, "textbox");
        assert_eq!(handle1.name.as_deref(), Some("Search Box"));

        let handle2 = ref_map.resolve(2).unwrap();
        assert_eq!(handle2.role, "button");

        let handle3 = ref_map.resolve(3).unwrap();
        assert_eq!(handle3.role, "link");
    }
}
