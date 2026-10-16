use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Accessibility Tree Node
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AXNode {
    pub ref_id: Option<u32>,
    pub backend_node_id: Option<u32>,
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

/// Stable Reference Registry mapping sequential u32 IDs to DOM elements
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RefMap {
    inner: HashMap<u32, AXNodeHandle>,
    next_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AXNodeHandle {
    pub backend_node_id: u32,
    pub role: String,
    pub name: Option<String>,
}

impl RefMap {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn assign(&mut self, backend_node_id: u32, role: String, name: Option<String>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.inner.insert(
            id,
            AXNodeHandle {
                backend_node_id,
                role,
                name,
            },
        );
        id
    }

    pub fn resolve(&self, ref_id: u32) -> Option<&AXNodeHandle> {
        self.inner.get(&ref_id)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.next_id = 1;
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// Unified Async Event Enum for central application loop
#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChallengeType {
    Turnstile,
    HCaptcha,
    ReCaptcha,
    TwoFactor,
    Unknown,
}

/// Agent Action Directives (MCP -> Engine)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentAction {
    pub action_type: ActionType,
    pub ref_id: Option<u32>,
    pub payload: Option<ActionPayload>,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ActionPayload {
    pub text: Option<String>,
    pub key: Option<String>,
    pub value: Option<String>,
    pub submit: Option<bool>,
    pub url: Option<String>,
    pub direction: Option<ScrollDirection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Serializable Page Snapshot representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub url: String,
    pub title: Option<String>,
    pub http_status: u16,
    pub mode: SnapshotMode,
    pub nodes: Vec<AXNode>,
    pub scroll_percent: f32,
    pub source: SnapshotSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnapshotMode {
    Semantic,
    Markdown,
    RawText,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnapshotSource {
    DirectNegotiation,
    LlmsTxt,
    CdpHeadless,
}

/// Session Profile definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionProfile {
    pub profile_id: String,
    pub user_agent: String,
    pub custom_headers: HashMap<String, String>,
}
