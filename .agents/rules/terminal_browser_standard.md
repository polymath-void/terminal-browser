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
   - **Direct Fetch**: `tb fetch "<url>"` (automatically attaches SQLite vault session cookies for authenticated domains).
   - **Interactive TUI**: `tb tui "<url>"` (launches 60 FPS Ratatui virtual viewport with Vimium keyboard shortcuts).
   - **Authenticated Login**: `tb login "<url>" -u "<username>" -p "<password>"` (submits form, extracts `Set-Cookie`, follows redirects to authenticated dashboard).
   - **Session Vault Management**:
     - `tb session list` (inspect active domain cookies stored in SQLite vault).
     - `tb session clear` (purge all session cookies from SQLite vault).
     - `tb session test` (run automated live end-to-end login & session verification).
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

4. **Environment-Aware Headed Detach (Human-in-the-Loop):**
   - If an authentication challenge (Cloudflare Turnstile, CAPTCHA, bank 2FA) or visual verification is encountered, invoke `tb open "<url>"` or `tb act --action detach_headed "<url>"`.
   - The launcher dispatches strictly based on the active runtime environment:
     - **Android Termux Environment**: ONLY use `termux-open-url "<url>"` (fallback `termux-open "<url>"`). Never attempt desktop browser commands or `xdg-open` in Termux.
     - **PC Windows Environment**: Use `cmd.exe /c start "" "<url>"` or `powershell Start-Process "<url>"`.
     - **Desktop Linux Environment**: Use `xdg-open "<url>"`.
     - **macOS Environment**: Use `open "<url>"`.
   - Once the human completes the interactive challenge in their native browser, session cookies auto-sync back to the SQLite vault and headless execution resumes.

5. **Persistent Cookie Vault & Authenticated Access:**
   - All session and authentication cookies are stored in SQLite `CookieVault` at `~/.terminal-browser/vault.db`.
   - RFC 6265 domain boundaries and parent domains are strictly respected and automatically matched.
   - Subsequent agent requests to authenticated domains automatically present valid cookies without manual token passing.
