---
name: tb-mcp7-agent-protocol
description: >-
  MCP-7 Agent Protocol Engineer for terminal-browser. Activate when implementing crates/term-mcp:
  the embedded Model Context Protocol server (stdio + SSE), browser_navigate / browser_snapshot /
  browser_act / browser_extract / browser_session tool handlers, compact AXTree markdown formatter,
  Ref ID resolution, and multi-agent session multiplexing.
---

# MCP-7: Agent Protocol Engineer

## Role
Owns `crates/term-mcp`. The agent-facing interface — embedded MCP server that translates
structured agent tool calls into CDP operations via `term-engine`, formats compact AXTree
snapshots for LLM consumption, and manages multi-agent session state.

## Blueprint Sources
- [ARCHITECTURE.md](../../../ARCHITECTURE.md): §4 Agentic Integrations
- [Semantic Web Browsing Skill](../semantic-web-browsing/SKILL.md): §5 MCP Browser Server Tools
- [mcp_browser_schema.json](../semantic-web-browsing/references/mcp_browser_schema.json)
- [axtree_example.md](../semantic-web-browsing/examples/axtree_example.md)

## Crate Structure
```
crates/term-mcp/
├── src/
│   ├── lib.rs
│   ├── server.rs          # MCP server runner (stdio + SSE on 127.0.0.1:6789)
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── navigate.rs    # browser_navigate handler
│   │   ├── snapshot.rs    # browser_snapshot → AXTree formatter
│   │   ├── act.rs         # browser_act → RefMap lookup → engine dispatch
│   │   ├── extract.rs     # browser_extract → readability markdown
│   │   └── session.rs     # browser_session import/export
│   ├── formatter.rs       # AXNode tree → compact markdown serializer
│   └── security.rs        # ActionInterceptor middleware (defers to SEC-8)
```

## AXTree → Compact Markdown Formatter (formatter.rs)

This is the most critical function — converts `Vec<AXNode>` into a compact, token-efficient
markdown representation that an LLM can consume with minimal hallucination risk.

```rust
pub fn format_snapshot(snapshot: &PageSnapshot) -> String {
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
    let name_str = node.name.as_deref().map(|n| format!(" \"{}\"", n)).unwrap_or_default();
    let val_str = node.value.as_deref()
        .filter(|v| !v.is_empty())
        .map(|v| format!(" value=\"{}\"", v))
        .unwrap_or_default();

    match node.role.as_str() {
        "link" => buf.push_str(&format!("{}[link{}{} href=\"{}\"]\n",
            indent, ref_str, name_str,
            node.href.as_deref().unwrap_or("#"))),
        "button"  => buf.push_str(&format!("{}[button{}{}]\n", indent, ref_str, name_str)),
        "textbox" => buf.push_str(&format!("{}[textbox{}{}{}]\n", indent, ref_str, name_str, val_str)),
        role      => buf.push_str(&format!("{}[{}{}]\n", indent, role, name_str)),
    }
    for child in &node.children {
        format_node(buf, child, depth + 1);
    }
}
```

**Output example:**
```
<!-- URL: https://arxiv.org/abs/2307.03172 | Status: 200 | Scroll: 0% -->
[banner]
  [heading:1 "ArXiv Explorer"]
  [textbox #1 "Search papers..." value=""]
  [button #2 "Search"]
[main]
  [heading:2 "WebAgent Architecture"]
  [button #3 "Download PDF" href="/pdf/2307.03172"]
  [table 2 cols, 2 rows]
    | Title | Year |
    | [link #4 "Mind2Web"] | 2023 |
```

## MCP Tool Handlers

### browser_navigate
```rust
pub async fn handle_navigate(params: NavigateParams, engine: &EngineHandle) -> ToolResult {
    engine.navigate(&params.url, params.wait_idle.unwrap_or(true)).await?;
    let snapshot = engine.get_snapshot(SnapshotMode::Semantic).await?;
    ToolResult::ok(format_snapshot(&snapshot))
}
```

### browser_act
```rust
pub async fn handle_act(params: ActParams, engine: &EngineHandle, security: &SecurityGate) -> ToolResult {
    // 1. Resolve Ref ID to CDP BackendNodeId
    let handle = engine.ref_map().resolve(params.ref_id)
        .ok_or_else(|| anyhow!("Ref ID {} not found in current snapshot", params.ref_id))?;

    // 2. Pass through security gate (SEC-8)
    security.check_action(&params, handle).await?;

    // 3. Dispatch action
    engine.dispatch_action(params).await?;

    // 4. Wait for stability, return delta snapshot
    engine.wait_stable().await?;
    let snapshot = engine.get_snapshot(SnapshotMode::Semantic).await?;
    ToolResult::ok(format_snapshot(&snapshot))
}
```

## MCP Server Runner (server.rs)
- **stdio mode** (default): reads newline-delimited JSON-RPC from stdin, writes to stdout.
- **SSE mode** (`--mcp-port 6789`): HTTP server on `127.0.0.1:6789`, sends Server-Sent Events.
- Supports multiple concurrent agent connections via `tokio::sync::broadcast` channels.

## Cargo.toml for term-mcp
```toml
[package]
name = "term-mcp"
version = "0.1.0"
edition = "2021"

[dependencies]
term-core    = { path = "../term-core" }
term-engine  = { path = "../term-engine" }
term-storage = { path = "../term-storage" }
tokio        = { version = "1", features = ["full"] }
serde_json   = "1"
anyhow       = "1"
tracing      = "0.1"
axum         = "0.7"   # For SSE HTTP server
```

## Validation
```bash
cargo check -p term-mcp
cargo test -p term-mcp
# Run MCP server in stdio test mode:
echo '{"method":"browser_navigate","params":{"url":"https://example.com"}}' | cargo run -p term-mcp
```
