# Terminal Web Browser (Next-Gen Semantic & Agentic TUI Browser)

A next-generation, platform-independent CLI terminal web browser engineered in **Rust** for human keyboard ergonomics and seamless AI agent web browsing across Windows, Linux, macOS, and Android Termux.

## Core Vision & Architecture

Unlike legacy text-only browsers (Lynx, w3m) or downsampled pixel streamers (Browsh, Carbonyl), this project implements a **decoupled, semantic-first architecture**:

- **Engine Subsystem**: Sandboxed Chromium / Servo runtime managing live DOM and JavaScript execution via local binary IPC.
- **Semantic Translation**: Accessibility Tree (`AXTree`) mapping directly into clean TUI primitives and compact LLM tokens instead of raw HTML or lossy block graphics.
- **Ergonomic TUI**: Ratatui-powered 60 FPS virtual viewport, Vimium-style link hinting (`f` -> two-letter tag jumps), and native reader mode extraction.
- **Agent Bridge (MCP)**: Native Model Context Protocol (MCP) server providing stable element Reference IDs (`#1`, `#2`, `#3`), zero-token-waste snapshots, and shared browser session vaults.
- **Transparent Identity & Fast-Path**: Open agent request profiles (`Accept: text/markdown`, `Sec-Purpose: ai-agent-reading`) and `/llms.txt` probing to bypass CAPTCHAs and load content in <50ms.
- **Cross-Platform Foundation**: Zero external runtime dependencies. Compiles to a single static binary for Windows (`x64`/`arm64`), Linux, macOS, and Termux.

## Architecture & Engineering Blueprints

| Blueprint | Description |
| :--- | :--- |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Core system architecture, open-source browser gap analysis, UX design |
| [FAST_BROWSING_AND_IDENTITY.md](FAST_BROWSING_AND_IDENTITY.md) | Fast browsing engine, transparent agent identity, anti-CAPTCHA, CDP resource filtering |
| [missing-blueprints.md](missing-blueprints.md) | Catalog of 6 critical missing engineering subsystems |
| [PLATFORM_ARCHITECTURE_AND_IMPLEMENTATION_BLUEPRINTS.md](PLATFORM_ARCHITECTURE_AND_IMPLEMENTATION_BLUEPRINTS.md) | Cross-platform stack analysis, Cargo workspace, async event loop, IPC, SQLite schema |
| [AGENT_TEAM_ARCHITECTURE.md](AGENT_TEAM_ARCHITECTURE.md) | Full 9-agent swarm team design with crate ownership, communication protocol, and phased roadmap |

## Development Agent Team (10 Specialist Roles)

| Agent ID | Role | Crate Ownership | Dedicated Skill |
| :--- | :--- | :--- | :--- |
| **ARCH-0** | Swarm Orchestrator | `Cargo.toml`, `src/main.rs`, CI | [tb-arch0-orchestrator](.agents/skills/tb-arch0-orchestrator/SKILL.md) |
| **CORE-1** | Domain Model Architect | `crates/term-core` | [tb-core1-domain-model](.agents/skills/tb-core1-domain-model/SKILL.md) |
| **IPC-2** | Wire Protocol Engineer | `crates/term-ipc` | [tb-ipc2-wire-protocol](.agents/skills/tb-ipc2-wire-protocol/SKILL.md) |
| **ENG-3** | Browser Engine Specialist | `crates/term-engine` | [tb-eng3-browser-engine](.agents/skills/tb-eng3-browser-engine/SKILL.md) |
| **TUI-4** | Terminal UI Engineer | `crates/term-tui` | [tb-tui4-terminal-ui](.agents/skills/tb-tui4-terminal-ui/SKILL.md) |
| **FORM-5** | Form & Event Dispatch Engineer | `crates/term-tui::forms` | [tb-form5-event-dispatcher](.agents/skills/tb-form5-event-dispatcher/SKILL.md) |
| **STORE-6** | Storage & Vault Engineer | `crates/term-storage` | [tb-store6-storage-vault](.agents/skills/tb-store6-storage-vault/SKILL.md) |
| **MCP-7** | Agent Protocol Engineer | `crates/term-mcp` | [tb-mcp7-agent-protocol](.agents/skills/tb-mcp7-agent-protocol/SKILL.md) |
| **SEC-8** | Security & Safety Engineer | `term-mcp::security`, `term-engine::sanitizer` | [tb-sec8-safety-engineer](.agents/skills/tb-sec8-safety-engineer/SKILL.md) |
| **QA-9** | Test & Integration Engineer | `tests/`, `.github/workflows/` | [tb-qa9-test-engineer](.agents/skills/tb-qa9-test-engineer/SKILL.md) |

## Quick Installation

Install the native binary and automatically provision the Antigravity agent skills & rules:

### Windows (PowerShell)
```powershell
.\install.ps1
```

### Linux / macOS / Android Termux (Bash)
```bash
./install.sh
```

The installer compiles the release binary and links `tb`, `term-browser`, and `terminal-browser` directly into your system `PATH` (`~/.cargo/bin` and Scoop shims), while installing the Antigravity skill and rule.

---

## CLI Usage

Run directly from any terminal without `cargo`:

```powershell
# 1. Fast-Path /llms.txt and Markdown Retrieval (<500ms)
tb fetch https://docs.anthropic.com/llms.txt

# 2. Interactive 60 FPS Terminal Browser
tb tui https://news.ycombinator.com

# 3. Embedded Model Context Protocol (MCP) Server for Agents
tb mcp

# 4. Background Browser Daemon
tb daemon
```

---

## Agent Skills & System Rules

- **Terminal Browser Skill**: [terminal-browser](.agents/skills/terminal-browser/SKILL.md) (provisioned to `~/.gemini/config/skills/terminal-browser/SKILL.md`)
- **Autonomous Operation Rule**: [terminal_browser_standard.md](.agents/rules/terminal_browser_standard.md) (provisioned to `~/.gemini/config/rules/terminal_browser_standard.md`)
- [Semantic Web Browsing Skill](.agents/skills/semantic-web-browsing/SKILL.md): Token-efficient AXTree extraction, noise filtering, and MCP browser control.
  - [MCP Browser Schema](.agents/skills/semantic-web-browsing/references/mcp_browser_schema.json)
  - [AXTree Snapshot Example](.agents/skills/semantic-web-browsing/examples/axtree_example.md)
  - [AXTree Extraction Script](.agents/skills/semantic-web-browsing/scripts/extract_axtree.py)
  - [Fast Agent Browser & Content Negotiator](scripts/fast_agent_browser.py)

