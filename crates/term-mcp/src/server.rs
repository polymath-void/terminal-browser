use crate::formatter::format_snapshot_markdown;
use serde_json::{json, Value};
use term_core::PageSnapshot;

pub struct McpServer;

impl McpServer {
    /// Handles standard MCP tool calls
    pub fn handle_tool_call(name: &str, args: &Value, current_snapshot: Option<&PageSnapshot>) -> Value {
        match name {
            "browser_snapshot" => {
                if let Some(snap) = current_snapshot {
                    let formatted = format_snapshot_markdown(snap);
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
                let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Navigation initiated to: {}", url)
                    }]
                })
            }
            "browser_act" => {
                let ref_id = args.get("ref_id").and_then(|v| v.as_u64()).unwrap_or(0);
                let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
                json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Executed action '{}' on Ref #{}", action, ref_id)
                    }]
                })
            }
            _ => json!({
                "isError": true,
                "content": [{ "type": "text", "text": format!("Unknown tool: {}", name) }]
            }),
        }
    }
}
