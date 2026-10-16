---
name: terminal-browser
description: Operate the native Terminal Browser (tb) for fast-path web content negotiation, 60 FPS TUI browsing, accessibility tree (AXTree) reduction with Ref IDs, embedded Model Context Protocol (MCP) server, and automated form dispatching.
---

# Terminal Browser (`tb`): Agent Operating Standard & Skill

The **Terminal Browser** is a production-grade, platform-independent, terminal-native web browser engineered specifically for AI agents and human power users. Built in Rust with Ratatui and Chromium DevTools Protocol (CDP), it eliminates rendering bloat, strips web noise, and enables instant, CAPTCHA-resistant web interaction.

---

## 1. Quick CLI Reference

The binary is installed as `tb`, `term-browser`, and `terminal-browser` in system `PATH` (`~/.cargo/bin` and Scoop shims):

| Command | Action | Latency / Behavior |
| :--- | :--- | :--- |
| `tb fetch <url>` | Fast-Path `/llms.txt` and markdown negotiation | **<500ms** direct HTTP fetch; skips CDP unless needed |
| `tb tui [url]` | Interactive 60 FPS Ratatui browser | Full keyboard navigation, Vimium jump hints, Reader Mode |
| `tb mcp` | Embedded Model Context Protocol server | Runs JSON-RPC 2.0 stdio server for LLM agents |
| `tb daemon` | Headless browser background engine | Cross-platform IPC (Named Pipe on Win, Unix Socket on Linux) |

---

## 2. Fast-Path Content Negotiation Protocol

Web servers increasingly support agentic formats (`/llms.txt` and `Accept: text/markdown`). The Terminal Browser uses transparent agent identity profiles to avoid bot detection while fetching clean data:

```powershell
# Probe /llms.txt and retrieve documentation summary
tb fetch https://docs.anthropic.com/llms.txt

# Disable /llms.txt probe and retrieve negotiated markdown
tb fetch https://news.ycombinator.com --probe-llms=false
```

### Transparent Agent Request Headers
```http
User-Agent: TerminalBrowserAgent/1.0 (+https://github.com/polymath-void/terminal-browser; ai-agent)
Accept: text/markdown, text/plain;q=0.9, text/html;q=0.5
Accept-Language: en-US,en;q=0.9
```

---

## 3. Model Context Protocol (MCP) Agent Directives

When integrated into agent tool runners, `tb mcp` provides four structured tools:

### A. `browser_navigate`
```json
{
  "name": "browser_navigate",
  "arguments": {
    "url": "https://news.ycombinator.com",
    "wait_until": "domcontentloaded"
  }
}
```

### B. `browser_snapshot`
Generates a token-compact, noise-reduced semantic representation with stable numeric **Ref IDs**:
```json
{
  "name": "browser_snapshot",
  "arguments": {
    "mode": "semantic"
  }
}
```
*Output Snapshot Example:*
```markdown
<!-- URL: https://news.ycombinator.com | Status: 200 | Scroll: 0% -->
[banner "Hacker News"]
  [link #1 "Hacker News" href="https://news.ycombinator.com"]
  [link #2 "new" href="newest"]
  [link #3 "past" href="front"]
  [link #4 "comments" href="newcomments"]
[main]
  [link #5 "Show HN: Terminal Browser in Rust" href="item?id=40001"]
  [button #6 "upvote"]
```

### C. `browser_act`
Executes targeted actions using Ref IDs without needing raw CSS selectors or coordinates:
```json
{
  "name": "browser_act",
  "arguments": {
    "ref_id": 5,
    "action": "click"
  }
}
```

#### High-Stakes Action Security Gate
Destructive or financial actions (e.g., clicking `[button #9 "Pay $50"]` or `[link #10 "Delete Account"]`) are classified as `Critical` risk by `SecurityGate`. They are blocked unless `force: true` is explicitly provided:
```json
{
  "name": "browser_act",
  "arguments": {
    "ref_id": 9,
    "action": "click",
    "force": true
  }
}
```

---

## 4. Interactive Keyboard Shortcuts (TUI Mode)

When running `tb tui <url>`:

- `f`: Activate **Vimium Link Hints** (overlays mnemonic 2-letter labels like `[aa]`, `[as]` on interactive elements).
- `j` / `k` or `Down` / `Up`: Scroll line-by-line.
- `d` / `u` or `PageDown` / `PageUp`: Half-page scroll.
- `gg` / `G`: Jump to document top / bottom.
- `/`: Open search prompt (searches document using SQLite FTS5 index).
- `r`: Toggle Reader Mode (pure markdown layout).
- `q` / `Esc`: Exit hint mode or quit browser.

---

## 5. Human-in-the-Loop Detach (CAPTCHA / 2FA Handling)

If a target website issues a Cloudflare Turnstile, CAPTCHA challenge, or bank login prompt:
1. The agent or user triggers `detach_headed`.
2. Chromium immediately opens a visible platform window (`msedge` on Windows, `google-chrome` on Linux/macOS) bound to the current session target ID.
3. The human completes the interactive challenge.
4. Updated cookies are automatically synced back into the SQLite `cookies` table, and the headless session resumes.

---

## 6. Binary Locations & Health Check

```powershell
# Verify binary is found in PATH
Get-Command tb, term-browser, terminal-browser

# Run internal test pass
tb --help
tb fetch https://docs.anthropic.com/llms.txt
```
