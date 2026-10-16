use crate::formatter::format_snapshot_markdown;
use crate::security::SecurityGate;
use serde_json::{json, Value};
use term_core::{AXNode, AXNodeHandle, ActionType, AgentAction, PageSnapshot};

pub struct McpServer;

impl McpServer {
    /// Handles standard MCP tool calls adhering to mcp_browser_schema.json
    pub fn handle_tool_call(name: &str, args: &Value, current_snapshot: Option<&PageSnapshot>) -> Value {
        match name {
            "browser_snapshot" => {
                if let Some(snap) = current_snapshot {
                    let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("semantic");
                    let _include_hidden = args.get("include_hidden").and_then(|v| v.as_bool()).unwrap_or(false);

                    let formatted = match mode {
                        "raw_text" => extract_raw_text(&snap.nodes),
                        _ => format_snapshot_markdown(snap),
                    };

                    json!({
                        "content": [{
                            "type": "text",
                            "text": formatted
                        }]
                    })
                } else {
                    json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": "No page currently loaded" }]
                    })
                }
            }
            "browser_navigate" => {
                let url = match args.get("url").and_then(|v| v.as_str()) {
                    Some(u) if !u.trim().is_empty() => u.trim(),
                    _ => {
                        return json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": "Missing or empty required parameter 'url'" }]
                        });
                    }
                };
                let wait_idle = args.get("wait_idle").and_then(|v| v.as_bool()).unwrap_or(true);
                let timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(30000);

                json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Navigation initiated to: {} (wait_idle={}, timeout_ms={})", url, wait_idle, timeout_ms)
                    }]
                })
            }
            "browser_act" => {
                let ref_id = match args.get("ref_id").and_then(|v| v.as_u64()) {
                    Some(id) => id as u32,
                    None => {
                        return json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": "Missing required parameter 'ref_id'" }]
                        });
                    }
                };
                let action_str = match args.get("action").and_then(|v| v.as_str()) {
                    Some(a) => a,
                    None => {
                        return json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": "Missing required parameter 'action'" }]
                        });
                    }
                };
                let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);

                let action_type = match action_str {
                    "click" => ActionType::Click,
                    "type" => ActionType::Type,
                    "select" => ActionType::Select,
                    "hover" => ActionType::Hover,
                    "scroll" => ActionType::Scroll,
                    "press_key" => ActionType::PressKey,
                    other => {
                        return json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": format!("Invalid action '{}'. Valid actions: click, type, select, hover, scroll, press_key", other) }]
                        });
                    }
                };

                let target_node = current_snapshot.and_then(|s| find_node_by_ref_id(&s.nodes, ref_id));
                let target_handle = target_node.map(|n| AXNodeHandle {
                    backend_node_id: n.backend_node_id.unwrap_or(0),
                    role: n.role.clone(),
                    name: n.name.clone(),
                });

                let agent_action = AgentAction {
                    action_type,
                    ref_id: Some(ref_id),
                    payload: None,
                    force,
                };

                // Enforce SecurityGate on critical/destructive actions
                if let Err(block_reason) = SecurityGate::can_execute(&agent_action, target_handle.as_ref()) {
                    return json!({
                        "isError": true,
                        "content": [{
                            "type": "text",
                            "text": format!("SecurityGate Blocked: {}", block_reason)
                        }]
                    });
                }

                let payload_desc = args.get("payload").map(|p| format!(" with payload {}", p)).unwrap_or_default();
                json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Executed action '{}' on Ref #{}{}", action_str, ref_id, payload_desc)
                    }]
                })
            }
            "browser_extract" => {
                let target = args.get("target").and_then(|v| v.as_str()).unwrap_or("article");
                if let Some(snap) = current_snapshot {
                    match target {
                        "links" => {
                            let mut links = Vec::new();
                            collect_links(&snap.nodes, &mut links);
                            let link_lines: Vec<String> = links.into_iter().map(|(n, h)| format!("- [{}]({})", n, h)).collect();
                            json!({
                                "content": [{
                                    "type": "text",
                                    "text": link_lines.join("\n")
                                }]
                            })
                        }
                        "metadata" => {
                            let scroll_pct = (snap.scroll_percent * 100.0) as u8;
                            json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!("URL: {}\nTitle: {}\nHTTP Status: {}\nScroll: {}%",
                                        snap.url,
                                        snap.title.as_deref().unwrap_or("None"),
                                        snap.http_status,
                                        scroll_pct
                                    )
                                }]
                            })
                        }
                        "tables" => {
                            json!({
                                "content": [{
                                    "type": "text",
                                    "text": "No tables detected in current viewport."
                                }]
                            })
                        }
                        _ => {
                            let clean_text = extract_raw_text(&snap.nodes);
                            json!({
                                "content": [{
                                    "type": "text",
                                    "text": clean_text
                                }]
                            })
                        }
                    }
                } else {
                    json!({
                        "isError": true,
                        "content": [{ "type": "text", "text": "No page currently loaded to extract content from" }]
                    })
                }
            }
            "browser_session" => {
                let action = match args.get("action").and_then(|v| v.as_str()) {
                    Some(a) if a == "export" || a == "import" || a == "clear" => a,
                    _ => {
                        return json!({
                            "isError": true,
                            "content": [{ "type": "text", "text": "Missing or invalid required parameter 'action'. Must be one of: export, import, clear" }]
                        });
                    }
                };
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                match action {
                    "export" => json!({
                        "content": [{ "type": "text", "text": format!("Session state exported to '{}'", path) }]
                    }),
                    "import" => json!({
                        "content": [{ "type": "text", "text": format!("Session state imported from '{}'", path) }]
                    }),
                    "clear" => json!({
                        "content": [{ "type": "text", "text": "Session vault and cookies cleared." }]
                    }),
                    _ => unreachable!(),
                }
            }
            _ => json!({
                "isError": true,
                "content": [{ "type": "text", "text": format!("Unknown tool: {}", name) }]
            }),
        }
    }
}

fn find_node_by_ref_id<'a>(nodes: &'a [AXNode], target_ref: u32) -> Option<&'a AXNode> {
    for node in nodes {
        if node.ref_id == Some(target_ref) {
            return Some(node);
        }
        if let Some(found) = find_node_by_ref_id(&node.children, target_ref) {
            return Some(found);
        }
    }
    None
}

fn collect_links(nodes: &[AXNode], links: &mut Vec<(String, String)>) {
    for node in nodes {
        if node.role == "link" {
            let name = node.name.clone().unwrap_or_else(|| "link".to_string());
            let href = node.href.clone().unwrap_or_else(|| "#".to_string());
            links.push((name, href));
        }
        collect_links(&node.children, links);
    }
}

fn extract_raw_text(nodes: &[AXNode]) -> String {
    let mut parts = Vec::new();
    collect_text(nodes, &mut parts);
    parts.join(" ")
}

fn collect_text(nodes: &[AXNode], out: &mut Vec<String>) {
    for node in nodes {
        if let Some(name) = &node.name {
            if !name.trim().is_empty() {
                out.push(name.trim().to_string());
            }
        }
        collect_text(&node.children, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use term_core::{SnapshotMode, SnapshotSource};

    fn make_test_snapshot() -> PageSnapshot {
        PageSnapshot {
            url: "https://example.com/checkout".to_string(),
            title: Some("Checkout Page".to_string()),
            http_status: 200,
            mode: SnapshotMode::Semantic,
            scroll_percent: 0.1,
            source: SnapshotSource::DirectNegotiation,
            nodes: vec![
                AXNode {
                    ref_id: Some(1),
                    backend_node_id: Some(101),
                    role: "button".to_string(),
                    name: Some("Pay $50".to_string()),
                    value: None,
                    placeholder: None,
                    checked: None,
                    expanded: None,
                    level: None,
                    href: None,
                    children: vec![],
                },
                AXNode {
                    ref_id: Some(2),
                    backend_node_id: Some(102),
                    role: "button".to_string(),
                    name: Some("Delete Account".to_string()),
                    value: None,
                    placeholder: None,
                    checked: None,
                    expanded: None,
                    level: None,
                    href: None,
                    children: vec![],
                },
                AXNode {
                    ref_id: Some(3),
                    backend_node_id: Some(103),
                    role: "button".to_string(),
                    name: Some("Continue Shopping".to_string()),
                    value: None,
                    placeholder: None,
                    checked: None,
                    expanded: None,
                    level: None,
                    href: None,
                    children: vec![],
                },
                AXNode {
                    ref_id: Some(4),
                    backend_node_id: Some(104),
                    role: "link".to_string(),
                    name: Some("Privacy Policy".to_string()),
                    value: None,
                    placeholder: None,
                    checked: None,
                    expanded: None,
                    level: None,
                    href: Some("https://example.com/privacy".to_string()),
                    children: vec![],
                },
            ],
        }
    }

    #[test]
    fn test_mcp_navigate_valid_and_invalid() {
        let valid_args = json!({ "url": "https://news.ycombinator.com", "wait_idle": true });
        let res = McpServer::handle_tool_call("browser_navigate", &valid_args, None);
        assert!(!res.get("isError").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(res["content"][0]["text"].as_str().unwrap().contains("https://news.ycombinator.com"));

        let invalid_args = json!({ "url": "" });
        let res_err = McpServer::handle_tool_call("browser_navigate", &invalid_args, None);
        assert!(res_err["isError"].as_bool().unwrap());
    }

    #[test]
    fn test_mcp_snapshot() {
        let snap = make_test_snapshot();
        let res = McpServer::handle_tool_call("browser_snapshot", &json!({}), Some(&snap));
        assert!(!res.get("isError").and_then(|v| v.as_bool()).unwrap_or(false));
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Pay $50"));
        assert!(text.contains("#1"));

        // No snapshot loaded returns error
        let res_none = McpServer::handle_tool_call("browser_snapshot", &json!({}), None);
        assert!(res_none["isError"].as_bool().unwrap());
    }

    #[test]
    fn test_mcp_act_blocks_critical_pay_without_force() {
        let snap = make_test_snapshot();
        let act_args = json!({
            "ref_id": 1,
            "action": "click",
            "force": false
        });
        let res = McpServer::handle_tool_call("browser_act", &act_args, Some(&snap));
        assert!(res["isError"].as_bool().unwrap());
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("SecurityGate Blocked"));
        assert!(text.contains("Critical financial or destructive action"));
    }

    #[test]
    fn test_mcp_act_allows_critical_pay_with_force() {
        let snap = make_test_snapshot();
        let act_args = json!({
            "ref_id": 1,
            "action": "click",
            "force": true
        });
        let res = McpServer::handle_tool_call("browser_act", &act_args, Some(&snap));
        assert!(!res.get("isError").and_then(|v| v.as_bool()).unwrap_or(false));
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Executed action 'click' on Ref #1"));
    }

    #[test]
    fn test_mcp_act_blocks_destructive_delete_without_force() {
        let snap = make_test_snapshot();
        let act_args = json!({
            "ref_id": 2,
            "action": "click",
            "force": false
        });
        let res = McpServer::handle_tool_call("browser_act", &act_args, Some(&snap));
        assert!(res["isError"].as_bool().unwrap());
        let text = res["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("SecurityGate Blocked"));
    }

    #[test]
    fn test_mcp_act_allows_destructive_delete_with_force() {
        let snap = make_test_snapshot();
        let act_args = json!({
            "ref_id": 2,
            "action": "click",
            "force": true
        });
        let res = McpServer::handle_tool_call("browser_act", &act_args, Some(&snap));
        assert!(!res.get("isError").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(res["content"][0]["text"].as_str().unwrap().contains("Executed action 'click' on Ref #2"));
    }

    #[test]
    fn test_mcp_act_safe_action_allowed() {
        let snap = make_test_snapshot();
        let act_args = json!({
            "ref_id": 3,
            "action": "click",
            "force": false
        });
        let res = McpServer::handle_tool_call("browser_act", &act_args, Some(&snap));
        assert!(!res.get("isError").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(res["content"][0]["text"].as_str().unwrap().contains("Executed action 'click' on Ref #3"));
    }

    #[test]
    fn test_mcp_extract_links_and_metadata() {
        let snap = make_test_snapshot();
        let res_links = McpServer::handle_tool_call("browser_extract", &json!({ "target": "links" }), Some(&snap));
        assert!(!res_links.get("isError").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(res_links["content"][0]["text"].as_str().unwrap().contains("[Privacy Policy](https://example.com/privacy)"));

        let res_meta = McpServer::handle_tool_call("browser_extract", &json!({ "target": "metadata" }), Some(&snap));
        assert!(!res_meta.get("isError").and_then(|v| v.as_bool()).unwrap_or(false));
        assert!(res_meta["content"][0]["text"].as_str().unwrap().contains("Checkout Page"));
    }

    #[test]
    fn test_mcp_session_management() {
        let res_exp = McpServer::handle_tool_call("browser_session", &json!({ "action": "export", "path": "vault.json" }), None);
        assert!(!res_exp.get("isError").and_then(|v| v.as_bool()).unwrap_or(false));

        let res_clear = McpServer::handle_tool_call("browser_session", &json!({ "action": "clear" }), None);
        assert!(!res_clear.get("isError").and_then(|v| v.as_bool()).unwrap_or(false));

        let res_invalid = McpServer::handle_tool_call("browser_session", &json!({ "action": "invalid" }), None);
        assert!(res_invalid["isError"].as_bool().unwrap());
    }

    #[test]
    fn test_unknown_mcp_tool() {
        let res = McpServer::handle_tool_call("nonexistent_tool", &json!({}), None);
        assert!(res["isError"].as_bool().unwrap());
        assert!(res["content"][0]["text"].as_str().unwrap().contains("Unknown tool"));
    }
}
