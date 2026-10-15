# Terminal Web Browser (Next-Gen Semantic & Agentic TUI Browser)

A next-generation CLI terminal web browser engineered for human keyboard ergonomics and seamless AI agent web browsing.

## Core Vision & Architecture

Unlike legacy text-only browsers (Lynx, w3m) or downsampled pixel streamers (Browsh, Carbonyl), this project implements a **decoupled, semantic-first architecture**:

- **Engine Subsystem**: Sandboxed Chromium / Servo runtime managing live DOM and JavaScript execution via local binary IPC.
- **Semantic Translation**: Accessibility Tree (`AXTree`) mapping directly into clean TUI primitives and compact LLM tokens instead of raw HTML or lossy block graphics.
- **Ergonomic TUI**: Vimium-style link hinting (`f` -> two-letter tag jumps), native reader mode extraction, and multi-protocol graphics (Sixel, Kitty).
- **Agent Bridge (MCP)**: Native Model Context Protocol (MCP) server providing stable element Reference IDs (`#1`, `#2`, `#3`), zero-token-waste snapshots, and shared browser session vaults.

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) / [terminal_browser_architecture.md](terminal_browser_architecture.md): Full system architecture specification, gap analysis of open-source CLI browsers, and agent integration protocols.
