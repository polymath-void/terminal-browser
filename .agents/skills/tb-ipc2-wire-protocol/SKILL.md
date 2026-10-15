---
name: tb-ipc2-wire-protocol
description: >-
  IPC-2 Wire Protocol Engineer for terminal-browser. Activate when implementing or debugging
  crates/term-ipc: cross-platform length-prefixed JSON-RPC 2.0 framing over Windows Named Pipes
  and Unix Domain Sockets, Chromium subprocess supervision, health heartbeats, memory leak
  watchdogs, and crash recovery with exponential backoff.
---

# IPC-2: Wire Protocol Engineer

## Role
Owns `crates/term-ipc`. Responsible for cross-platform inter-process communication between the TUI
frontend and background browser daemon, as well as supervising the headless Chromium subprocess.

## Blueprint Sources
- [missing-blueprints.md](../../../missing-blueprints.md): §1 IPC Wire Protocol & Daemon Lifecycle
- [PLATFORM_ARCHITECTURE_AND_IMPLEMENTATION_BLUEPRINTS.md](../../../PLATFORM_ARCHITECTURE_AND_IMPLEMENTATION_BLUEPRINTS.md): §4 Cross-Platform IPC Wire Protocol

## Crate Structure
```
crates/term-ipc/
├── src/
│   ├── lib.rs
│   ├── transport/
│   │   ├── mod.rs         # Unified IpcStream & IpcListener traits
│   │   ├── windows.rs     # tokio::net::windows::named_pipe
│   │   └── unix.rs        # tokio::net::UnixStream / UnixListener
│   ├── codec.rs           # Length-prefixed framing codec (Tokio Framed)
│   ├── protocol.rs        # JSON-RPC 2.0 request, response, and notification types
│   ├── client.rs          # Async IPC client with request-response matching
│   ├── server.rs          # Async IPC server handling multiple connections
│   └── supervisor.rs      # Subprocess supervisor, memory watchdog, health pings
```

## 1. Framing Codec (codec.rs)
Uses Tokio `Decoder` and `Encoder` with 4-byte big-endian length prefixing:
```
+------------------------------------+------------------------------------------+
| Payload Length: u32 (4 bytes, BE)  | JSON-RPC 2.0 UTF-8 Payload Bytes         |
+------------------------------------+------------------------------------------+
```

```rust
use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

pub struct LengthPrefixedJsonCodec;

impl Decoder for LengthPrefixedJsonCodec {
    type Item = serde_json::Value;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }
        let length = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;
        if src.len() < 4 + length {
            src.reserve(4 + length - src.len());
            return Ok(None);
        }
        src.advance(4);
        let payload = src.split_to(length);
        let value = serde_json::from_slice(&payload)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(Some(value))
    }
}

impl Encoder<serde_json::Value> for LengthPrefixedJsonCodec {
    type Error = std::io::Error;

    fn encode(&mut self, item: serde_json::Value, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let json_bytes = serde_json::to_vec(&item)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        dst.reserve(4 + json_bytes.len());
        dst.put_u32(json_bytes.len() as u32);
        dst.put_slice(&json_bytes);
        Ok(())
    }
}
```

---

## 2. Platform-Agnostic Transport (transport/mod.rs)

```rust
pub const WINDOWS_PIPE_NAME: &str = r"\\.\pipe\term_browser_ipc";
pub const UNIX_SOCKET_DEFAULT: &str = "/tmp/term_browser.sock";

pub fn get_socket_path() -> String {
    #[cfg(target_os = "windows")]
    {
        WINDOWS_PIPE_NAME.to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("XDG_RUNTIME_DIR")
            .map(|dir| format!("{}/term_browser.sock", dir))
            .unwrap_or_else(|_| UNIX_SOCKET_DEFAULT.to_string())
    }
}
```

---

## 3. Subprocess Supervisor & Watchdog (supervisor.rs)

Supervises the browser engine process, monitors RSS memory, runs health heartbeats, and restarts with exponential backoff on crash:

```rust
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};

pub struct ProcessSupervisor {
    child: Option<Child>,
    restart_attempts: u32,
    max_memory_mb: u64,
    navigations_count: u32,
    max_navigations_before_recycle: u32,
}

impl ProcessSupervisor {
    pub fn new(max_memory_mb: u64, max_navs: u32) -> Self {
        Self {
            child: None,
            restart_attempts: 0,
            max_memory_mb,
            navigations_count: 0,
            max_navigations_before_recycle: max_navs,
        }
    }

    pub async fn spawn_engine(&mut self, binary_path: &str, args: &[&str]) -> anyhow::Result<()> {
        let child = Command::new(binary_path)
            .args(args)
            .kill_on_drop(true)
            .spawn()?;
        self.child = Some(child);
        self.restart_attempts = 0;
        tracing::info!("Browser subprocess spawned successfully");
        Ok(())
    }

    pub async fn check_and_recycle_if_needed(&mut self) -> anyhow::Result<bool> {
        self.navigations_count += 1;
        if self.navigations_count >= self.max_navigations_before_recycle {
            tracing::warn!("Recycling browser process after {} navigations", self.navigations_count);
            self.recycle().await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn recycle(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
        }
        self.navigations_count = 0;
        Ok(())
    }
}
```

---

## Cargo.toml for term-ipc
```toml
[package]
name = "term-ipc"
version = "0.1.0"
edition = "2021"

[dependencies]
term-core = { path = "../term-core" }
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["codec"] }
bytes = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
tracing = "0.1"
```

## Validation
```bash
cargo check -p term-ipc
cargo test -p term-ipc
# Verify IPC pipe roundtrip:
cargo run -p term-ipc --example echo_pipe_test
```
