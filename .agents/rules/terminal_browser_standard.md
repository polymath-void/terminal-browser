---
trigger: always_on
---
# Terminal Browser Autonomous Operation Standard

Whenever the USER or AGENT needs to browse the web, retrieve web page contents, read documentation, inspect live websites, interact with web forms, or automate browser actions:

1. **Native Tool Priority & Fast-Path Protocol:**
   - ALWAYS prefer the native `tb` / `terminal-browser` CLI over heavy headless browsers or fragile raw scraping.
   - **Fast-Path Negotiation (<500ms)**:
     Before launching full CDP rendering, probe for `/llms.txt` or negotiate markdown:
     ```powershell
     tb fetch "<url>"
     ```
     This automatically dispatches transparent agent headers (`User-Agent: TerminalBrowserAgent/1.0`, `Accept: text/markdown, text/plain`) and skips HTML/JS/CSS bloat, eliminating CAPTCHA risk.

2. **Standard CLI Execution Commands:**
   - **Direct Fetch**: `tb fetch "<url>"` (falls back to CDP if markdown is not directly returned).
   - **Interactive TUI**: `tb tui "<url>"` (launches 60 FPS Ratatui virtual viewport with Vimium keyboard shortcuts).
   - **Model Context Protocol (MCP)**: `tb mcp` (runs embedded MCP server over stdio for agent tool-use).
   - **Background Engine Daemon**: `tb daemon` (listens on IPC named pipe `\\.\pipe\term_browser_ipc` on Windows or Unix socket `/tmp/term_browser.sock` on Linux/Termux).

3. **Autonomous Agent MCP Directives:**
   When driving the browser programmatically via MCP or IPC:
   - Query page state with compact AXTree reduction: `browser_snapshot`.
   - Interact using stable numeric Ref IDs:
     - `browser_act(ref_id, action="click")`
     - `browser_act(ref_id, action="type", payload="text")`
     - `browser_act(ref_id, action="submit")`
   - **Security Gate Compliance**: High-stakes destructive actions (e.g., delete account, submit payment) require explicit `force=true` or user confirmation in TUI.

4. **Human-in-the-Loop Headed Detach:**
   - If an authentication challenge (Cloudflare Turnstile, CAPTCHA, bank 2FA) is encountered, invoke:
     `tb act --action detach_headed`
   - This seamlessly projects the current Chromium target into a visible window without losing session cookies or restarting the tab. Once solved, cookies auto-sync back to the SQLite vault.
