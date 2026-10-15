---
name: tb-eng3-browser-engine
description: >-
  ENG-3 Browser Engine Specialist for terminal-browser. Activate when implementing or
  debugging crates/term-engine: headless Chromium CDP lifecycle, AXTree pruning pipeline,
  fast-path /llms.txt and content-negotiation retrieval, resource blocklist, anti-overlay
  injection, stability gating (waitForNetworkIdle), and transparent agent HTTP headers.
---

# ENG-3: Browser Engine Specialist

## Role
Owns `crates/term-engine`. The core intelligence that drives headless Chromium and converts
live web pages into structured semantic data for both the TUI and agent consumers.

## Blueprint Sources
- [ARCHITECTURE.md](../../../ARCHITECTURE.md): §1 Decoupled Engine Architecture, §2 Semantic AXTree
- [FAST_BROWSING_AND_IDENTITY.md](../../../FAST_BROWSING_AND_IDENTITY.md): §2–4 All sections
- [Semantic Web Browsing Skill](../semantic-web-browsing/SKILL.md): §1–3 Full protocol

## Crate Structure
```
crates/term-engine/
├── src/
│   ├── lib.rs
│   ├── supervisor.rs      # Chromium process lifecycle, crash recovery, memory watchdog
│   ├── cdp_runner.rs      # chromiumoxide session, CDP method dispatchers
│   ├── fast_path.rs       # /llms.txt probe + Accept: text/markdown negotiation
│   ├── axtree.rs          # Full AXTree fetch, prune, and RefMap assignment
│   ├── sanitizer.rs       # Anti-overlay CSS injection + readability extraction
│   ├── stability.rs       # waitForNetworkIdle, waitForMutation, waitForUrl
│   └── blocklist.rs       # BLOCKED_RESOURCE_PATTERNS
```

## Implementation Details

### Chromium Supervisor (supervisor.rs)
```rust
pub const CHROMIUM_FLAGS: &[&str] = &[
    "--headless=new",
    "--remote-debugging-port=0",   // ephemeral port, returned via pipe
    "--disable-gpu",
    "--blink-settings=imagesEnabled=false",
    "--disable-remote-fonts",
    "--disable-background-networking",
    "--disable-background-timer-throttling",
    "--disable-client-side-phishing-detection",
    "--disable-default-apps",
    "--disable-hang-monitor",
    "--disable-sync",
    "--no-first-run",
    "--safebrowsing-disable-auto-update",
    "--password-store=basic",
    "--use-mock-keychain",
];
```

### AXTree Pruning Algorithm (axtree.rs)
Retain elements meeting any of these conditions:
1. Role in `INTERACTIVE_ROLES` (button, link, textbox, combobox, checkbox, radio, tab, menuitem, searchbox)
2. Role in `STRUCTURAL_ROLES` (banner, navigation, main, contentinfo, heading, table, row, cell, list, listitem, dialog)
3. Has a non-empty accessible name and is visible (not hidden)

After pruning, assign sequential u32 RefIDs via `term_core::RefMap`.

### Fast-Path Content Negotiation (fast_path.rs)
Priority order:
1. `GET https://domain.com/llms.txt` — check for site-wide agent-optimized index.
2. `GET <url>` with `Accept: text/markdown, text/plain;q=0.9` — returns pre-rendered markdown.
3. Fallback: CDP headless page load with AXTree extraction.

### Agent Identity Headers
```rust
pub fn agent_headers(agent_ua: &str) -> reqwest::header::HeaderMap {
    let mut map = reqwest::header::HeaderMap::new();
    map.insert("User-Agent", agent_ua.parse().unwrap());
    map.insert("Accept", "text/markdown, text/plain;q=0.9, text/html;q=0.5".parse().unwrap());
    map.insert("Sec-Purpose", "ai-agent-reading".parse().unwrap());
    map.insert("X-Agent-Protocol", "MCP/1.0".parse().unwrap());
    map
}
```

### Anti-Overlay Injection (sanitizer.rs)
Inject via `Runtime.evaluate` after page load, before AXTree snapshot:
```javascript
// Remove cookie banners, modals, popups
document.querySelectorAll('[aria-modal="true"], [role="dialog"]').forEach(el => {
  if (/cookie|consent|banner|newsletter|overlay/i.test(el.className + el.id)) {
    el.remove();
  }
});
document.body.style.overflow = 'auto';
document.documentElement.style.overflow = 'auto';
```

### On-Demand Headed Detach & Screencast (detach.rs)
Implements Blueprint §5 for solving visual CAPTCHAs and 2FA challenges:

1. **On-Demand Headed Detach**:
   When a challenge is detected (`AppEvent::ChallengeRequired`), rather than killing Chromium and losing session cookies, call:
   ```rust
   pub async fn detach_headed(&self, current_url: &str) -> anyhow::Result<()> {
       // Spawns a temporary visible OS browser window attached to the same user profile/port
       #[cfg(target_os = "windows")]
       tokio::process::Command::new("cmd")
           .args(["/c", "start", "msedge", current_url])
           .spawn()?;
       #[cfg(target_os = "macos")]
       tokio::process::Command::new("open").arg(current_url).spawn()?;
       #[cfg(target_os = "linux")]
       tokio::process::Command::new("xdg-open").arg(current_url).spawn()?;
       Ok(())
   }
   ```
2. **Terminal Screencasting Protocol (`Page.startScreencast`)**:
   Streams visual JPEG frames directly into the terminal for headless visual debugging:
   ```rust
   use chromiumoxide::cdp::browser_protocol::page::{
       StartScreencastFormat, StartScreencastParams,
   };

   pub async fn start_screencast(&self, page: &chromiumoxide::Page) -> anyhow::Result<()> {
       let params = StartScreencastParams::builder()
           .format(StartScreencastFormat::Jpeg)
           .quality(60)
           .max_width(800)
           .max_height(600)
           .build();
       page.execute(params).await?;
       Ok(())
   }
   ```

## Cargo.toml for term-engine
```toml
[package]
name = "term-engine"
version = "0.1.0"
edition = "2021"

[dependencies]
term-core = { path = "../term-core" }
chromiumoxide = { version = "0.7", features = ["tokio-runtime"] }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["gzip", "brotli", "http2"] }
anyhow = "1"
tracing = "0.1"
regex = "1"
```

## Validation
```bash
cargo check -p term-engine
cargo test -p term-engine
# Verify fast-path works on a known llms.txt site:
cargo run -p term-engine --example fast_path_test -- https://docs.anthropic.com
```
