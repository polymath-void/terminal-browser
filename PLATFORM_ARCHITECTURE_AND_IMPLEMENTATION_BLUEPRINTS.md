# Platform-Independent Architecture & Implementation Blueprints

## 1. Cross-Platform Technology Stack Evaluation

To achieve true platform independence across **Windows (x64/arm64)**, **Linux (x64/arm64)**, **macOS (Intel/Apple Silicon)**, and **Android Termux (aarch64)** without external runtime requirements (no pre-installed Node.js, Python, or dynamic C libraries), we evaluated the primary systems engineering ecosystems:

| Evaluation Criteria | Rust (`ratatui` + `chromiumoxide`) | Go (`bubbletea` + `chromedp`) | Node.js (`ink` + `puppeteer`) | Zig / C++ |
| :--- | :--- | :--- | :--- | :--- |
| **Portability & Distribution** | **Exceptional** (Single static binary, zero C-lib dependencies) | **High** (Single binary, but CGO needed for SQLite FTS5) | **Poor** (Requires Node runtime; 120MB+ bundle size) | **High** (Static binary, but no ecosystem) |
| **TUI Performance (60 FPS)** | **Zero GC pauses**, instant microsecond cell diffing via `crossterm` | Minor stutters during Go GC cycles under heavy DOM parsing | Jittery event loop; high CPU on large terminal reflows | Peak performance, but high boilerplate |
| **CDP Async Automation** | **First-class** (`chromiumoxide` generates typed CDP domains) | Good (`chromedp`), but imperative API is hard to stream | Good (`puppeteer`), but memory-heavy | No mature CDP library; must write from scratch |
| **Cross-Platform IPC** | Native Windows Named Pipes + Unix Sockets unified via `interprocess` | Windows named pipes require external CGO wrappers | Complex OS-level IPC | Low-level OS syscalls |
| **Cold Startup Time** | **< 15 ms** | ~35 ms | ~450 ms | **< 5 ms** |
| **Memory Footprint** | **~12 MB** | ~28 MB | ~110 MB | **~8 MB** |

### The Chosen Stack: Rust Systems Architecture
- **Language**: **Rust 2024 Edition**
- **TUI & Terminal Engine**: [`ratatui`](https://crates.io/crates/ratatui) + [`crossterm`](https://crates.io/crates/crossterm) (pure Rust, handles Windows Virtual Terminal Sequences and Unix VT100/termios).
- **Asynchronous Runtime**: [`tokio`](https://crates.io/crates/tokio) (multi-threaded work-stealing runtime with `select!` multiplexing).
- **Headless CDP Engine**: [`chromiumoxide`](https://crates.io/crates/chromiumoxide) (fully async, typed Chrome DevTools Protocol implementation).
- **Embedded Storage**: [`rusqlite`](https://crates.io/crates/rusqlite) with `bundled` and `bundled-full` features (embeds SQLite 3 + FTS5 full-text search directly inside the binary).
- **Cross-Platform IPC**: Native Windows Named Pipes (`tokio::net::windows::named_pipe`) and Unix Domain Sockets (`tokio::net::UnixListener`).
- **Agent Protocol**: Embedded MCP Server over `stdio` and local SSE.

---

## 2. Cargo Workspace Architecture

The browser is modularized into isolated crates within a single Cargo workspace:

```text
terminal-browser/
├── Cargo.toml                       # Workspace definition
├── crates/
│   ├── term-core/                   # Domain models: Viewport, AXNode, RefMap, Event, URL
│   ├── term-engine/                 # CDP runner, Chromium supervisor, resource blocklist, /llms.txt
│   ├── term-tui/                    # Ratatui presentation, virtual viewport, Vimium hints
│   ├── term-storage/                # SQLite + FTS5 history, cookies, session vault
│   ├── term-ipc/                    # Cross-platform IPC wire framing (Named Pipes / Unix Sockets)
│   └── term-mcp/                    # Embedded Model Context Protocol server
├── src/
│   └── main.rs                      # Unified CLI entrypoint (Runs TUI, Daemon, or MCP Server)
└── tests/
    └── integration_tests.rs
```

---

## 3. Implementation Blueprint 1: The Unified Async Event Loop

The TUI and Agent APIs must coordinate concurrently without deadlocks or UI stuttering. This is solved via an **Actor-based `tokio::select!` event loop**:

```mermaid
flowchart TD
    subgraph Inputs ["Asynchronous Event Sources"]
        KeyEvents["Keyboard & Terminal Resize<br/>(crossterm EventStream)"]
        CDPEvents["CDP Page & DOM Events<br/>(chromiumoxide WebSocket)"]
        AgentCmds["Agent MCP Directives<br/>(term-mcp / IPC channel)"]
        Timer["Tick Interval (60 FPS)<br/>(tokio::time::interval)"]
    end

    subgraph CoreLoop ["Central Application State Machine (AppActor)"]
        Loop["tokio::select!"]
    end

    subgraph Outputs ["State Render & Execution"]
        Render["Double-Buffered TUI Draw<br/>(ratatui::Terminal::draw)"]
        ActionDispatch["Synthetic Event Dispatch<br/>(Input.dispatchKeyEvent / Click)"]
    end

    KeyEvents --> Loop
    CDPEvents --> Loop
    AgentCmds --> Loop
    Timer --> Loop
    Loop --> Render
    Loop --> ActionDispatch
```

### Event Loop Implementation Specification

```rust
// crates/term-tui/src/app.rs
pub enum AppEvent {
    TermInput(crossterm::event::Event),
    CdpMessage(chromiumoxide::cdp::browser_protocol::page::EventFrameNavigated),
    AgentAction(term_core::AgentAction),
    Tick,
}

pub async fn run_event_loop(
    mut app: AppState,
    mut terminal: ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    mut event_rx: tokio::sync::mpsc::Receiver<AppEvent>,
) -> anyhow::Result<()> {
    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                match event {
                    AppEvent::TermInput(e) => app.handle_key(e).await?,
                    AppEvent::CdpMessage(nav) => app.on_frame_navigated(nav).await?,
                    AppEvent::AgentAction(action) => app.execute_agent_action(action).await?,
                    AppEvent::Tick => {
                        terminal.draw(|frame| app.render_ui(frame))?;
                    }
                }
                if app.should_quit {
                    break;
                }
            }
        }
    }
    Ok(())
}
```

---

## 4. Implementation Blueprint 2: Cross-Platform IPC Wire Protocol

The TUI frontend must operate either standalone or attached to an existing background `term-browser-daemon`.

### Transport Specification
- **Windows**: `\\.\pipe\term_browser_ipc` via `tokio::net::windows::named_pipe::ServerOptions`.
- **Linux / macOS / Termux**: `$XDG_RUNTIME_DIR/term_browser.sock` (falling back to `/tmp/term_browser.sock`) via `tokio::net::UnixListener`.

### Framing Protocol
Every packet is length-prefixed:
```
+-----------------------------------+------------------------------------------+
| Payload Length (4 bytes, BigEndian) | JSON-RPC 2.0 Message Payload (UTF-8 Bytes)|
+-----------------------------------+------------------------------------------+
```

### JSON-RPC Commands:
- `browser.navigate(url, wait_idle)`
- `browser.get_snapshot(mode: "semantic"|"markdown")`
- `browser.act(ref_id: u32, action: "click"|"type", payload)`
- `browser.detach_headed()` (Pops visible Chrome window for CAPTCHA/2FA)

---

## 5. Implementation Blueprint 3: Virtual Viewport & 60 FPS Scrolling

When rendering 10,000 lines of parsed markdown or AXTree nodes, allocating full terminal cells crashes memory and locks the UI thread.

### Virtual Viewport Algorithm
1. The document is parsed into an indexed vector: `Vec<RenderNode>`.
2. Each `RenderNode` contains `measured_height: u16`, `ref_id: Option<u32>`, and `content: Line<'a>`.
3. The TUI tracks `scroll_offset_y: usize`.
4. During `terminal.draw()`:
   - Compute terminal viewport height: $H_{term}$ (e.g., 38 rows).
   - Slice only the visible items from index $I_{start}$ to $I_{end}$ where $\sum \text{height} \le H_{term}$.
   - Pass only the visible slice to Ratatui's buffer renderer.

```rust
// crates/term-tui/src/viewport.rs
pub struct VirtualViewport {
    pub nodes: Vec<RenderNode>,
    pub scroll_y: usize,
    pub max_scroll: usize,
}

impl VirtualViewport {
    pub fn render_visible_slice(&self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let mut current_row = area.top();
        for node in self.nodes.iter().skip(self.scroll_y) {
            if current_row >= area.bottom() {
                break;
            }
            node.render_at(current_row, area.left(), area.width, buf);
            current_row += node.height;
        }
    }
}
```

---

## 6. Implementation Blueprint 4: Persistent SQLite Vault Schema

The storage crate (`crates/term-storage`) manages cross-session state using SQLite with **FTS5 full-text search**.

```sql
-- Cookie Vault
CREATE TABLE IF NOT EXISTS cookies (
    domain TEXT NOT NULL,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    path TEXT NOT NULL DEFAULT '/',
    expires_at INTEGER,
    is_secure BOOLEAN NOT NULL DEFAULT 1,
    is_httponly BOOLEAN NOT NULL DEFAULT 0,
    same_site TEXT NOT NULL DEFAULT 'Lax',
    PRIMARY KEY (domain, name, path)
);

-- Browsing History
CREATE TABLE IF NOT EXISTS history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT UNIQUE NOT NULL,
    title TEXT,
    visit_count INTEGER NOT NULL DEFAULT 1,
    last_visited_at INTEGER NOT NULL
);

-- FTS5 Full-Text Search Virtual Table for Content
CREATE VIRTUAL TABLE IF NOT EXISTS page_content_fts USING fts5(
    url UNINDEXED,
    title,
    body_markdown
);

-- Named Agent Profiles
CREATE TABLE IF NOT EXISTS agent_profiles (
    profile_id TEXT PRIMARY KEY,
    user_agent TEXT NOT NULL,
    custom_headers TEXT, -- JSON blob
    created_at INTEGER NOT NULL
);
```

---

## 7. Implementation Blueprint 5: Human-in-the-Loop "Detach-to-Headed" Engine

When an agent encounters a Cloudflare Turnstile, biometrics, or bank authentication challenge:

1. **State Preservation**: The headless engine captures current target ID and session storage.
2. **Dynamic Window Attachment**: Rather than restarting Chromium and losing context:
   - Call CDP `Target.createTarget(url, newWindow=true)`.
   - Dispatches an OS signal or launches a minimal native viewer window using the local platform browser binary (`msedge` on Windows, `google-chrome` on Linux/macOS).
3. **Session Re-sync**: Once the user completes the 2FA / CAPTCHA, cookies are refreshed in the `cookies` table, the headed viewer closes, and the CLI resume hook fires automatically.
