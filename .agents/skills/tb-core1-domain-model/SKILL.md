---
name: tb-core1-domain-model
description: >-
  CORE-1 Domain Model Architect for terminal-browser. Activate when defining or modifying
  shared Rust types in crates/term-core: AXNode, RefMap, AppEvent, AgentAction, PageSnapshot,
  SessionProfile, BrowserUrl. This agent owns all cross-crate data structures and serialization contracts.
---

# CORE-1: Domain Model Architect

## Role
Owns all shared Rust domain types in `crates/term-core`. Zero business logic — pure type library.

## Crate: `crates/term-core/`

## Key Types to Implement

### AXNode — Accessibility Tree Node
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AXNode {
    pub ref_id: Option<u32>,
    pub role: String,
    pub name: Option<String>,
    pub value: Option<String>,
    pub placeholder: Option<String>,
    pub checked: Option<bool>,
    pub expanded: Option<bool>,
    pub level: Option<u32>,
    pub href: Option<String>,
    pub children: Vec<AXNode>,
}
```

### RefMap — Stable Reference Registry
```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct RefMap {
    inner: HashMap<u32, AXNodeHandle>,
    next_id: u32,
}

#[derive(Debug, Clone)]
pub struct AXNodeHandle {
    pub backend_node_id: u32,  // CDP BackendNodeId
    pub role: String,
    pub name: Option<String>,
}

impl RefMap {
    pub fn assign(&mut self, backend_node_id: u32, role: String, name: Option<String>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.inner.insert(id, AXNodeHandle { backend_node_id, role, name });
        id
    }

    pub fn resolve(&self, ref_id: u32) -> Option<&AXNodeHandle> {
        self.inner.get(&ref_id)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.next_id = 0;
    }
}
```

### AppEvent — Unified Async Event Enum
```rust
#[derive(Debug)]
pub enum AppEvent {
    TermInput(crossterm::event::Event),
    PageNavigated { url: String, status: u16 },
    SnapshotReady(PageSnapshot),
    AgentAction(AgentAction),
    ChallengeRequired { challenge_type: ChallengeType },
    EngineHealthCheck { memory_mb: u64, tab_count: u32 },
    Tick,
    Quit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeType {
    Turnstile,
    HCaptcha,
    ReCaptcha,
    TwoFactor,
    Unknown,
}
```

### AgentAction — MCP → Engine Commands
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    pub action_type: ActionType,
    pub ref_id: Option<u32>,
    pub payload: Option<ActionPayload>,
    pub force: bool,  // bypass confirmation gate
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Click,
    Type,
    Select,
    Hover,
    Scroll,
    PressKey,
    Navigate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPayload {
    pub text: Option<String>,
    pub key: Option<String>,
    pub value: Option<String>,
    pub submit: Option<bool>,
    pub url: Option<String>,
    pub direction: Option<ScrollDirection>,
}
```

### PageSnapshot — Serializable Viewport State
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub url: String,
    pub title: Option<String>,
    pub http_status: u16,
    pub mode: SnapshotMode,
    pub nodes: Vec<AXNode>,
    pub scroll_percent: f32,
    pub source: SnapshotSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotMode { Semantic, Markdown, RawText }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SnapshotSource { DirectNegotiation, LlmsTxt, CdpHeadless }
```

## Cargo.toml for term-core
```toml
[package]
name = "term-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
url = "2"
uuid = { version = "1", features = ["v4"] }
crossterm = "0.28"
```

## Validation
After implementing, run:
```bash
cargo check -p term-core
cargo test -p term-core
```
