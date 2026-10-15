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
