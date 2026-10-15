# Terminal Web Browser (Next-Gen Semantic & Agentic TUI Browser)

A next-generation CLI terminal web browser engineered for human keyboard ergonomics and seamless AI agent web browsing.

## Core Vision & Architecture

Unlike legacy text-only browsers (Lynx, w3m) or downsampled pixel streamers (Browsh, Carbonyl), this project implements a **decoupled, semantic-first architecture**:

- **Engine Subsystem**: Sandboxed Chromium / Servo runtime managing live DOM and JavaScript execution via local binary IPC.
- **Semantic Translation**: Accessibility Tree (`AXTree`) mapping directly into clean TUI primitives and compact LLM tokens instead of raw HTML or lossy block graphics.
- **Ergonomic TUI**: Vimium-style link hinting (`f` -> two-letter tag jumps), native reader mode extraction, and multi-protocol graphics (Sixel, Kitty).
- **Agent Bridge (MCP)**: Native Model Context Protocol (MCP) server providing stable element Reference IDs (`#1`, `#2`, `#3`), zero-token-waste snapshots, and shared browser session vaults.
- **Transparent Identity & Fast-Path**: Open agent request profiles (`Accept: text/markdown`, `Sec-Purpose: ai-agent-reading`) and `/llms.txt` probing to bypass CAPTCHAs and load content in <50ms.

## Architecture Specifications

- [ARCHITECTURE.md](ARCHITECTURE.md) / [terminal_browser_architecture.md](terminal_browser_architecture.md): Core decoupled system architecture, open-source CLI browser analysis, and UX design.
- [FAST_BROWSING_AND_IDENTITY.md](FAST_BROWSING_AND_IDENTITY.md): Fast browsing engine, transparent agent identity profile, anti-CAPTCHA strategies, and CDP resource filtering benchmarks.

## Integrated Agent Skills & Scripts

- [Semantic Web Browsing Skill](.agents/skills/semantic-web-browsing/SKILL.md): Complete agent runbook for token-efficient AXTree extraction, noise filtering, and MCP browser control.
  - [MCP Browser Schema](.agents/skills/semantic-web-browsing/references/mcp_browser_schema.json)
  - [AXTree Snapshot Example](.agents/skills/semantic-web-browsing/examples/axtree_example.md)
  - [AXTree Extraction Script](.agents/skills/semantic-web-browsing/scripts/extract_axtree.py)
  - [Fast Agent Browser & Content Negotiator](scripts/fast_agent_browser.py)
