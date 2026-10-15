use term_core::{AXNode, RefMap};
use term_engine::axtree::prune_axtree;
use term_mcp::formatter::format_snapshot_markdown;
use term_mcp::security::{ActionRisk, SecurityGate};

#[test]
fn test_axtree_pruning_and_ref_assignment() {
    let mut ref_map = RefMap::new();

    let raw_tree = AXNode {
        ref_id: None,
        backend_node_id: Some(101),
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
                ref_id: None,
                backend_node_id: Some(102),
                role: "button".to_string(),
                name: Some("Search Button".to_string()),
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
                backend_node_id: Some(103),
                role: "none".to_string(), // non-semantic node
                name: None,
                value: None,
                placeholder: None,
                checked: None,
                expanded: None,
                level: None,
                href: None,
                children: vec![],
            },
        ],
    };

    let pruned = prune_axtree(raw_tree, &mut ref_map).expect("Should retain banner");
    assert_eq!(pruned.role, "banner");
    assert_eq!(pruned.children.len(), 1); // "none" was pruned
    assert_eq!(pruned.children[0].role, "button");
    assert_eq!(pruned.children[0].ref_id, Some(1));
    assert_eq!(ref_map.len(), 1);
}

#[test]
fn test_security_gate_critical_action() {
    use term_core::{AXNodeHandle, ActionType, AgentAction};

    let critical_action = AgentAction {
        action_type: ActionType::Click,
        ref_id: Some(1),
        payload: None,
        force: false,
    };

    let pay_button = AXNodeHandle {
        backend_node_id: 12,
        role: "button".to_string(),
        name: Some("Pay $50.00 Now".to_string()),
    };

    let risk = SecurityGate::classify_action(&critical_action, Some(&pay_button));
    assert_eq!(risk, ActionRisk::Critical);

    let can_exec = SecurityGate::can_execute(&critical_action, Some(&pay_button));
    assert!(can_exec.is_err());
}
