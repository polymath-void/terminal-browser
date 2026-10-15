---
name: semantic-web-browsing
description: >-
  Actionable workflow and implementation guide for agentic web browsing, terminal web browsing,
  semantic AXTree extraction, deterministic Ref ID interactions, and anti-noise content extraction
  derived from Google DeepMind research and modern autonomous web agent architectures.
---

# Semantic Web Browsing: Agentic & Terminal Architecture

This skill provides the operational standards and code templates for executing and developing
semantic-first, token-efficient web browsing systems for humans and AI agents.

## Core Protocols

### 1. Viewport Ingestion: The AXTree over Raw DOM
Agents must NEVER ingest raw, unpruned HTML. All page representations must follow the
Accessibility Tree (AXTree) reduction protocol:

1. **Query AXTree Snapshot**: Obtain `Accessibility.getFullAXTree` via Chrome DevTools Protocol (CDP) or native engine bindings.
2. **Prune Passive Nodes**: Strip non-semantic containers (`<div>`, `<span>` without labels or handlers). Retain elements with:
   - Interactive roles: `button`, `link`, `textbox`, `combobox`, `checkbox`, `radio`, `tab`, `menuitem`.
   - Structural landmarks: `banner`, `navigation`, `main`, `contentinfo`, `heading`, `table`.
   - Text content and ARIA accessible names / descriptions.
3. **Sequential Ref Mapping**: Assign stable numeric handles (`#1`, `#2`, `#3`) sequentially from top-left to bottom-right across the active viewport.

#### Canonical Compact Representation
```markdown
[main]
  [heading:1 "Antigravity Research Portal"]
  [textbox #1 "Search papers or topics..." value=""]
  [button #2 "Search"]
  [table 2 cols, 2 rows]
    | Title | Author |
    | [link #3 "WebAgent Architecture"] | DeepMind |
    | [link #4 "Terminal AXTree Pipelines"] | Polymath |
```

---

### 2. Action Normalization & Execution
Actions must be dispatched deterministically using assigned `Ref IDs`:

- **Click**: `act(ref=2, action="click")` -> Maps internally to element center coordinate or CDP `DOM.focus` + synthetic mouse/pointer event.
- **Type**: `act(ref=1, action="type", text="AXTree Pruning", submit=true)` -> Sets input value, dispatches `input` and `change` events, and triggers Enter if requested.
- **Select**: `act(ref=5, action="select", value="option_val")`.
- **Scroll**: `act(action="scroll", direction="down", amount="viewport")`.

#### Stability Gating
Never return immediate results following an action. Enforce three deterministic settling criteria:
1. `DOM Mutation Settled`: MutationObserver reports zero mutations for at least 150ms.
2. `Network Idle`: CDP `Network.loadingFinished` reports 0 active in-flight requests (excluding persistent SSE/WebSockets) for 300ms.
3. `URL Transition`: If a navigation event was triggered, block until `Page.loadEventFired` or `Page.frameNavigated`.

---

### 3. Noise Neutralization & Anti-Overlay Engine
Before taking any semantic snapshot or TUI render:
1. **Modal & Backdrop Removal**:
   - Query elements with `z-index > 1000` or CSS classes matching `/cookie|consent|banner|overlay|paywall|newsletter|modal/i`.
   - Inject style override: `[aria-modal="true"], .cookie-banner { display: none !important; }` and remove `overflow: hidden` on `<body>` and `<html>`.
2. **Reader Mode Bypass**:
   - When extracting article or documentation text, run Mozilla Readability / Trafilatura to extract markdown directly, bypassing layout DOM entirely.

---

### 4. Shared Cookie & Session Vault (Zero-CAPTCHA Protocol)
1. **Profile Inheritance**: When initiating the browser engine, mount the human's existing Chrome/Chromium profile directory in read-only / temporary clone mode (`--user-data-dir`).
2. **Human-in-the-Loop Hand-off**:
   - If a Cloudflare Turnstile, hCaptcha, or 2FA challenge is detected (`[role="dialog"]` with captcha indicators):
   - Browser emits an MCP event: `{"status": "challenge_required", "type": "turnstile"}`.
   - CLI prints an alert: `CAPTCHA detected. Press Enter once solved in the interactive window.`
   - Once resolved, the engine saves the minted session cookies to the local vault and resumes autonomous agent loop.

---

### 5. Standard MCP Browser Server Tools

Expose the following tool signatures to any calling agent:

| Tool Name | Parameters | Description |
| :--- | :--- | :--- |
| `browser_navigate` | `url: string, wait_idle: bool` | Navigates to a target URL, runs anti-overlay filters, and waits for stability. |
| `browser_snapshot` | `mode: "semantic" \| "markdown" \| "raw_text"` | Returns the viewport snapshot with numbered Ref IDs. |
| `browser_act` | `ref_id: int, action: string, payload: dict` | Dispatches normalized actions against target Ref IDs. |
| `browser_extract` | `target: "article" \| "table" \| "links"` | Extracts structured data without interactive markup. |
| `browser_session` | `action: "export" \| "import", path: string` | Manages authenticated cookies and storage state. |

---

## 6. Directory Structure & References

- **Tool Schema**: [references/mcp_browser_schema.json](./references/mcp_browser_schema.json)
- **Snapshot Sample**: [examples/axtree_example.md](./examples/axtree_example.md)
- **Extraction Helper**: [scripts/extract_axtree.py](./scripts/extract_axtree.py)
