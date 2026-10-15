---
name: tb-tui4-terminal-ui
description: >-
  TUI-4 Terminal UI Engineer for terminal-browser. Activate when implementing or debugging
  crates/term-tui: Ratatui multi-pane layout, 60 FPS virtual viewport with dirty cell diffing,
  Vimium-style link hint engine (f-key jump labels), Reader Mode toggle, Sixel/Kitty inline
  graphics, status bar, and MCP activity drawer. Owns the entire human-facing terminal surface.
---

# TUI-4: Terminal UI Engineer

## Role
Owns `crates/term-tui`. The visual presentation layer — 60 FPS Ratatui-powered terminal UI
with multi-pane layout, virtual viewport, and ergonomic Vimium-style keyboard navigation.

## Blueprint Sources
- [missing-blueprints.md](../../../missing-blueprints.md): §2 TUI Windowing & Virtual Scrolling
- [PLATFORM_ARCHITECTURE_AND_IMPLEMENTATION_BLUEPRINTS.md](../../../PLATFORM_ARCHITECTURE_AND_IMPLEMENTATION_BLUEPRINTS.md): §3 Event Loop, §5 Virtual Viewport

## Crate Structure
```
crates/term-tui/
├── src/
│   ├── lib.rs
│   ├── app.rs             # AppState, run_event_loop (tokio::select!)
│   ├── layout.rs          # Multi-pane split layout definitions
│   ├── viewport.rs        # VirtualViewport, RenderNode, index-sliced rendering
│   ├── hints.rs           # Vimium-style link hint engine
│   ├── reader.rs          # Reader Mode markdown renderer
│   ├── graphics.rs        # Sixel/Kitty/Braille/ASCII image tier selection
│   ├── statusbar.rs       # Status bar widget
│   ├── drawer.rs          # MCP Agent Activity drawer
│   └── forms/             # (FORM-5 module)
│       ├── mod.rs
│       ├── textinput.rs
│       ├── dropdown.rs
│       └── event_dispatcher.rs
```

## Layout Architecture
```
┌──────────────────────────────────────────────────────────────────────────┐
│ Tab Bar: [1: GitHub] [2: ArXiv*] [3: Docs]          [Mem:42MB | ⚡OK]   │
├────────────────────┬─────────────────────────────────────────────────────┤
│   TOC Sidebar      │           Main Viewport (AXTree / Markdown)          │
│                    │                                                       │
│  ▸ Introduction    │  [heading:1 "System Architecture"]                   │
│  ▸ Architecture    │  Distributed nodes require robust protocols...        │
│    ▸ IPC Layer     │                                                       │
│    ▸ TUI Engine    │  [link #14 "Source Repository"]                      │
│  ▸ Security        │  [button #15 "Download PDF"]                         │
│                    │                                                       │
├────────────────────┴─────────────────────────────────────────────────────┤
│ 200 OK | Semantic | https://arxiv.org/abs/123 | [f] Hints  [:] Command   │
│ [MCP] browser_act(ref=14, click) ✓ 32ms                                  │
└──────────────────────────────────────────────────────────────────────────┘
```

## Virtual Viewport (viewport.rs)
```rust
pub struct RenderNode {
    pub ref_id: Option<u32>,
    pub height: u16,          // measured lines this node occupies
    pub hint_label: Option<[char; 2]>,  // Vimium hint e.g. ['a','k']
    pub content: ratatui::text::Line<'static>,
}

pub struct VirtualViewport {
    pub nodes: Vec<RenderNode>,
    pub scroll_y: usize,
}

impl VirtualViewport {
    pub fn render(&self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        let mut row = area.top();
        for node in self.nodes.iter().skip(self.scroll_y) {
            if row >= area.bottom() { break; }
            // render node content + hint badge if present
            buf.set_line(row, area.left(), &node.content, area.width);
            row += node.height;
        }
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_y = (self.scroll_y + lines).min(self.nodes.len().saturating_sub(1));
    }
}
```

## Vimium Hint Engine (hints.rs)
```rust
const HINT_CHARS: &[char] = &['a','s','d','f','g','h','j','k','l','e','r','t','y','u','i','o','p'];

pub fn generate_hints(ref_ids: &[u32]) -> HashMap<u32, [char; 2]> {
    let mut hints = HashMap::new();
    for (i, &ref_id) in ref_ids.iter().enumerate() {
        let a = HINT_CHARS[(i / HINT_CHARS.len()) % HINT_CHARS.len()];
        let b = HINT_CHARS[i % HINT_CHARS.len()];
        hints.insert(ref_id, [a, b]);
    }
    hints
}
```

**Activation Flow:**
1. User presses `f` → `HintMode` activates, overlay renders `[ab]` badges in yellow over all interactive nodes.
2. User types first char → filter hints, dim non-matching badges.
3. User types second char → if unique match, immediately fire `AgentAction::Click(ref_id)`.

## Async Event Loop (app.rs)
```rust
pub async fn run_event_loop(
    mut app: AppState,
    mut terminal: ratatui::Terminal<CrosstermBackend<Stdout>>,
    mut event_rx: tokio::sync::mpsc::Receiver<AppEvent>,
) -> anyhow::Result<()> {
    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                match event {
                    AppEvent::TermInput(e)         => app.handle_key(e).await?,
                    AppEvent::SnapshotReady(snap)  => app.update_viewport(snap),
                    AppEvent::AgentAction(action)  => app.dispatch_action(action).await?,
                    AppEvent::ChallengeRequired{..}=> app.show_challenge_prompt(),
                    AppEvent::Tick                 => { terminal.draw(|f| app.render(f))?; },
                    AppEvent::Quit                 => break,
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
```

## Cargo.toml for term-tui
```toml
[package]
name = "term-tui"
version = "0.1.0"
edition = "2021"

[dependencies]
term-core = { path = "../term-core" }
ratatui = "0.29"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
unicode-width = "0.2"
anyhow = "1"
tracing = "0.1"
```

## Validation
```bash
cargo check -p term-tui
cargo test -p term-tui
# Render snapshot test at 80x24:
cargo run -p term-tui --example viewport_test
```
