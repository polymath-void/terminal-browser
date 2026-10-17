pub mod axtree;
pub mod blocklist;
pub mod cdp;
pub mod detach;
pub mod fast_path;
pub mod sanitizer;

pub use axtree::prune_axtree;
pub use blocklist::{BLOCKED_PATTERNS, CHROMIUM_AGENT_FLAGS};
pub use cdp::{find_browser_binary, is_cdp_available, is_js_required, CdpEngine, RenderedPage};
pub use detach::detach_headed;
pub use fast_path::{FastPathClient, ResearchedPage, SearchResult};
pub use sanitizer::sanitize_web_text;

