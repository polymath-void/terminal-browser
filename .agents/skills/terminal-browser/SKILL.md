---
name: terminal-browser
description: Operate the native Terminal Browser (tb) for fast-path web content negotiation, 60 FPS TUI browsing, accessibility tree (AXTree) reduction with Ref IDs, embedded Model Context Protocol (MCP) server, SQLite cookie session persistence, and automated form dispatching.
---

# Terminal Browser (`tb`): Agent Operating Standard & Skill

The **Terminal Browser** is a production-grade, platform-independent, terminal-native web browser engineered specifically for AI agents and human power users. Built in Rust with Ratatui and Chromium DevTools Protocol (CDP), it eliminates rendering bloat, strips web noise, and enables instant, CAPTCHA-resistant web interaction.

---

## 1. Quick CLI Reference

The binary is installed as `tb`, `term-browser`, and `terminal-browser` in system `PATH` (`~/.cargo/bin` and Scoop shims):

| Command | Action | Latency / Behavior |
| :--- | :--- | :--- |
| `tb fetch <url>` | Fast-Path `/llms.txt` and markdown negotiation | **<500ms** direct HTTP fetch; injects SQLite vault cookies |
| `tb login <url> -u <user> -p <pass>` | Authenticated form submission & cookie storage | Posts credentials, follows 301/302/303, saves cookies in vault |
| `tb session [list\|clear\|test]` | SQLite CookieVault management & live verification | Inspects, purges, or runs automated live session tests |
| `tb tui [url]` | Interactive 60 FPS Ratatui browser | Full keyboard navigation, Vimium jump hints, Reader Mode |
| `tb open <url>` / `tb detach <url>` | Environment-aware headed browser detach | Projects page into Termux/Windows/Linux/Mac browser for 2FA |
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

## 3. SQLite CookieVault & Authenticated Login Sessions

The Terminal Browser includes a built-in persistent **Cookie Vault** powered by SQLite (`~/.terminal-browser/vault.db`):

- **Automatic Cookie Injection**: `tb fetch` and `tb tui` automatically query the vault for domain-matching cookies (RFC 6265 compliant) and attach the `Cookie` header.
- **Form Login**: `tb login <url> -u <username> -p <password>` submits form data via POST, follows redirect hops (301, 302, 303), captures `Set-Cookie` headers into the vault, and renders the authenticated destination page.
- **Session Verification**: `tb session test` executes a live end-to-end verification against live endpoints (e.g. `the-internet.herokuapp.com`).
- **Inspection & Cleanup**:
  ```powershell
  # List stored cookies and expiry
  tb session list

  # Clear all cookies from SQLite vault
  tb session clear
  ```

---

## 4. Model Context Protocol (MCP) Agent Directives

When integrated into agent tool runners, `tb mcp` provides structured tools:

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
Destructive or financial actions (e.g., clicking `[button #9 "Pay $50"]` or `[link #10 "Delete Account"]`) are classified as `Critical` risk by `SecurityGate`. They are blocked unless `force: true` is explicitly provided.

---

## 5. Interactive Keyboard Shortcuts (TUI Mode)

When running `tb tui <url>`:

- `f`: Activate **Vimium Link Hints** (overlays mnemonic 2-letter labels like `[aa]`, `[as]` on interactive elements).
- `j` / `k` or `Down` / `Up`: Scroll line-by-line.
- `d` / `u` or `PageDown` / `PageUp`: Half-page scroll.
- `gg` / `G`: Jump to document top / bottom.
- `/`: Open search prompt (searches document using SQLite FTS5 index).
- `r`: Toggle Reader Mode (pure markdown layout).
- `q` / `Esc`: Exit hint mode or quit browser.

---

## 6. Environment-Aware Headed Detach (Human-in-the-Loop)

If a target website issues a Cloudflare Turnstile, CAPTCHA challenge, or bank login prompt:
1. The agent or user triggers `tb open <url>`, `tb detach <url>`, or MCP tool `detach_headed`.
2. The launcher automatically detects the active host environment and dispatches the native command:
   - **Android Termux**: ONLY invokes `termux-open-url <url>` (or fallback `termux-open <url>`) to project the page directly into the default Android browser.
   - **Windows PC**: Invokes `cmd /c start "" <url>`.
   - **Desktop Linux**: Invokes `xdg-open <url>`.
   - **macOS**: Invokes `open <url>`.
3. The human completes the interactive challenge.
4. Updated cookies are automatically synced back into the SQLite `cookies` table, and the headless session resumes.

---

## 7. Binary Locations & Health Check

```powershell
# Verify binary is found in PATH
Get-Command tb, term-browser, terminal-browser

# Run live login and session test
tb session test

# Fetch authenticated or public web content
tb fetch https://docs.anthropic.com/llms.txt
```
