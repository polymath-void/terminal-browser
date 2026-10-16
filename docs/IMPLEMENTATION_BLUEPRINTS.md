# Terminal Browser (`tb`) — Core Systems Implementation Blueprints

**Document Version:** 1.0.0  
**Target Architecture:** Rust 2021 Edition / POSIX & Android Termux (Bionic libc `aarch64`) / Windows  
**Status:** Approved Architectural Specification  

---

## 1. Executive Architecture Overview

This document provides exhaustive, production-ready implementation blueprints for the three remaining partial/unwired subsystems identified in the [CLI Browser Checklist](file:///data/data/com.termux/files/home/.gemini/antigravity-cli/brain/84525177-1466-4153-8881-4fd5902eb5f9/cli_browser_checklist.md):

```
+-----------------------------------------------------------------------------------+
|                            Terminal Browser Core Engine                           |
+-----------------------------------------------------------------------------------+
|                                                                                   |
|  [ Blueprint 1: SQLite Storage ]  <--->  [ Blueprint 2: IPC Daemon ]               |
|  - Cookie Vault (RFC 6265)               - Tokio Unix Domain Socket Server        |
|  - Bidirectional Detach Sync             - LengthPrefixedJsonCodec                |
|  - Persistent History & Bookmarks        - JSON-RPC 2.0 Method Routing            |
|                                                                                   |
|                                     |                                             |
|                                     v                                             |
|                   [ Blueprint 3: Interactive Form Engine ]                        |
|                   - HTML AST / AXNode Form Field Extraction                       |
|                   - Viewport FocusRing State Machine                              |
|                   - Native POST & SyntheticDispatcher Submission                  |
|                                                                                   |
+-----------------------------------------------------------------------------------+
```

---

## 2. Blueprint 1: SQLite Storage, Cookie Vault & Session Persistence

### 2.1 Storage Directory & File Hierarchy

The browser runtime resolves the storage directory dynamically with Termux and XDG compliance:

```
~/.config/terminal-browser/
├── vault.db               <-- SQLite database (WAL mode enabled)
├── vault.db-wal
├── vault.db-shm
├── daemon.pid             <-- Daemon process lock
└── profiles/
    └── default.json
```

### 2.2 Database Initialization & Path Resolution

```rust
// Location: crates/term-storage/src/lib.rs

use rusqlite::Connection;
use std::path::PathBuf;

pub struct DatabaseManager {
    conn: Connection,
    db_path: PathBuf,
}

impl DatabaseManager {
    /// Resolves canonical configuration directory respecting XDG and Termux environment
    pub fn get_default_db_path() -> PathBuf {
        let base_dir = if let Ok(termux_prefix) = std::env::var("PREFIX") {
            // Android Termux environment: $HOME/.config/terminal-browser
            let home = std::env::var("HOME").unwrap_or_else(|_| "/data/data/com.termux/files/home".to_string());
            PathBuf::from(home).join(".config").join("terminal-browser")
        } else if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config).join("terminal-browser")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config").join("terminal-browser")
        } else {
            PathBuf::from(".").join(".terminal-browser")
        };

        let _ = std::fs::create_dir_all(&base_dir);
        base_dir.join("vault.db")
    }

    pub fn open_default() -> anyhow::Result<Self> {
        let path = Self::get_default_db_path();
        Self::open_or_create(&path.to_string_lossy())
    }

    pub fn open_or_create(path_str: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path_str)?;
        // Enable WAL mode and synchronous=NORMAL for high throughput and battery efficiency
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA busy_timeout = 5000;
        ")?;

        crate::cookies::CookieVault::init_table(&conn)?;
        crate::history::HistoryVault::init_table(&conn)?;
        crate::profiles::ProfileVault::init_table(&conn)?;

        Ok(Self {
            conn,
            db_path: PathBuf::from(path_str),
        })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}
```

### 2.3 SQLite-Backed Reqwest Cookie Store

In `crates/term-engine/src/fast_path.rs`, `reqwest::Client` integrates with SQLite using a custom cookie provider:

```rust
// Location: crates/term-engine/src/cookie_jar.rs

use reqwest::cookie::CookieStore;
use term_storage::{CookieVault, DatabaseManager, StoredCookie};
use std::sync::{Arc, Mutex};
use url::Url;

pub struct SqliteCookieJar {
    db: Arc<Mutex<DatabaseManager>>,
}

impl SqliteCookieJar {
    pub fn new(db: Arc<Mutex<DatabaseManager>>) -> Self {
        Self { db }
    }
}

impl CookieStore for SqliteCookieJar {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &reqwest::header::HeaderValue>, url: &Url) {
        let domain = match url.host_str() {
            Some(d) => d,
            None => return,
        };

        let db = match self.db.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        for header in cookie_headers {
            if let Ok(cookie_str) = header.to_str() {
                if let Some((name_val, _)) = cookie_str.split_once(';') {
                    if let Some((name, val)) = name_val.split_once('=') {
                        let cookie = StoredCookie {
                            domain: domain.to_string(),
                            name: name.trim().to_string(),
                            value: val.trim().to_string(),
                            path: url.path().to_string(),
                            expires_at: None,
                            is_secure: url.scheme() == "https",
                            is_httponly: false,
                            same_site: "Lax".to_string(),
                        };
                        let _ = CookieVault::upsert(db.conn(), &cookie);
                    }
                }
            }
        }
    }

    fn cookies(&self, url: &Url) -> Option<reqwest::header::HeaderValue> {
        let domain = url.host_str()?;
        let db = self.db.lock().ok()?;
        let cookies = CookieVault::get_for_domain(db.conn(), domain).ok()?;

        if cookies.is_empty() {
            return None;
        }

        let cookie_pairs: Vec<String> = cookies
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect();
        let cookie_header_str = cookie_pairs.join("; ");
        reqwest::header::HeaderValue::from_str(&cookie_header_str).ok()
    }
}
```

### 2.4 Bidirectional Cookie Synchronization (Headed Detach Protocol)

When a website blocks automated access with Cloudflare WAF, the browser triggers `tb open <url>`. Once the user solves the challenge in their external browser, cookies are synchronized back into SQLite:

```rust
// Location: crates/term-engine/src/sync.rs

use term_storage::{CookieVault, DatabaseManager, StoredCookie};
use std::sync::{Arc, Mutex};

pub struct CookieSyncEngine;

impl CookieSyncEngine {
    /// Ingests cookies returned from a completed headed browser session
    pub fn import_session_cookies(
        db: Arc<Mutex<DatabaseManager>>,
        domain: &str,
        raw_cookie_pairs: &[(&str, &str)],
    ) -> anyhow::Result<usize> {
        let guard = db.lock().unwrap();
        let mut count = 0;

        for (name, value) in raw_cookie_pairs {
            let record = StoredCookie {
                domain: domain.to_string(),
                name: name.to_string(),
                value: value.to_string(),
                path: "/".to_string(),
                expires_at: Some(chrono::Utc::now().timestamp() + 86400 * 30),
                is_secure: true,
                is_httponly: true,
                same_site: "Lax".to_string(),
            };
            CookieVault::upsert(guard.conn(), &record)?;
            count += 1;
        }

        Ok(count)
    }
}
```

---

## 3. Blueprint 2: Background IPC Engine & Daemon (`tb daemon`)

### 3.1 IPC Architecture & Socket Framing

The background daemon operates as a persistent async engine over Unix Domain Sockets (Linux/Termux) and Named Pipes (Windows):

```
+------------------+         Unix Domain Socket         +-----------------------+
|  tb CLI Client   |  <==============================>  |  tb Daemon Process    |
|  (e.g., tb URL)  |    Length-Prefixed JSON-RPC 2.0    |  (PID: daemon.pid)    |
+------------------+                                    +-----------------------+
                                                                   |
                                                        +----------+----------+
                                                        |  Engine State Pool  |
                                                        |  - Sqlite CookieJar |
                                                        |  - Shared Cache     |
                                                        |  - Active Workers   |
                                                        +---------------------+
```

### 3.2 Protocol RPC Message Catalog

Frames use a 4-byte big-endian length prefix followed by JSON payload:

```json
// Client Request
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "NAVIGATE",
  "params": {
    "url": "https://lite.duckduckgo.com/lite/",
    "timeout_ms": 10000
  }
}

// Daemon Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "status": 200,
    "url": "https://lite.duckduckgo.com/lite/",
    "content_type": "text/html",
    "markdown": "# DuckDuckGo Lite\n\nSearch page loaded.",
    "links_count": 14
  }
}
```

### 3.3 Server Daemon Implementation Blueprint

```rust
// Location: crates/term-ipc/src/daemon.rs

use crate::codec::LengthPrefixedJsonCodec;
use crate::transport::get_socket_path;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tokio_util::codec::Framed;

pub struct DaemonServer {
    socket_path: String,
    db_manager: Arc<Mutex<term_storage::DatabaseManager>>,
}

impl DaemonServer {
    pub fn new() -> anyhow::Result<Self> {
        let db = Arc::new(Mutex::new(term_storage::DatabaseManager::open_default()?));
        Ok(Self {
            socket_path: get_socket_path(),
            db_manager: db,
        })
    }

    pub async fn run_loop(&self) -> anyhow::Result<()> {
        let p = Path::new(&self.socket_path);
        if p.exists() {
            let _ = std::fs::remove_file(p);
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        tracing::info!("Daemon listening on: {}", self.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let db = Arc::clone(&self.db_manager);
                    tokio::spawn(async move {
                        let mut framed = Framed::new(stream, LengthPrefixedJsonCodec);
                        while let Some(Ok(request)) = framed.next().await {
                            let response = Self::handle_rpc_request(request, &db).await;
                            if let Err(e) = framed.send(response).await {
                                tracing::error!("Failed to send IPC response: {}", e);
                                break;
                            }
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Error accepting IPC connection: {}", e);
                }
            }
        }
    }

    async fn handle_rpc_request(
        req: Value,
        _db: &Arc<Mutex<term_storage::DatabaseManager>>,
    ) -> Value {
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = match req.get("method").and_then(|m| m.as_str()) {
            Some(m) => m,
            None => {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32600, "message": "Invalid Request: missing method" }
                });
            }
        };

        match method {
            "PING" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": "PONG"
            }),
            "FETCH" => {
                let url = req["params"]["url"].as_str().unwrap_or("about:home");
                let client = term_engine::FastPathClient::new(None);
                match client.fetch_page_content(url).await {
                    Ok((content, status)) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": { "status": status, "url": url, "content": content }
                    }),
                    Err(e) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32000, "message": e.to_string() }
                    }),
                }
            }
            "SEARCH" => {
                let query = req["params"]["query"].as_str().unwrap_or("");
                let client = term_engine::FastPathClient::new(None);
                match client.search_web_results(query).await {
                    Ok(results) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": results
                    }),
                    Err(e) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32001, "message": e.to_string() }
                    }),
                }
            }
            "SHUTDOWN" => {
                std::process::exit(0);
            }
            unknown => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {}", unknown) }
            }),
        }
    }
}
```

---

## 4. Blueprint 3: Interactive Form Engine & DOM Input Dispatch

### 4.1 HTML Form AST Node Schema

When HTML contains `<form>` structures, the parser extracts form controls into interactive AST nodes:

```rust
// Location: crates/term-core/src/forms.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormElementType {
    Text,
    Password,
    Checkbox { checked: bool },
    Radio { group: String, selected: bool },
    Select { options: Vec<String>, selected_index: usize },
    SubmitButton,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormFieldDescriptor {
    pub form_action: String,
    pub form_method: String, // "GET" or "POST"
    pub field_name: String,
    pub element_type: FormElementType,
    pub ref_id: u32,
    pub backend_node_id: u32,
    pub initial_value: String,
}
```

### 4.2 FocusRing State Machine in TUI Viewport

The TUI maintains a `FocusRing` tracking all active inputs in the loaded document:

```rust
// Location: crates/term-tui/src/forms/focus_ring.rs

use crate::forms::TextInputWidget;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct FocusRing {
    pub active_index: Option<usize>,
    pub fields: Vec<u32>, // ref_ids of form fields in document order
    pub widgets: HashMap<u32, TextInputWidget>,
}

impl FocusRing {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_field(&mut self, widget: TextInputWidget) {
        let ref_id = widget.ref_id;
        self.fields.push(ref_id);
        self.widgets.insert(ref_id, widget);
    }

    pub fn focus_next(&mut self) -> Option<u32> {
        if self.fields.is_empty() {
            return None;
        }

        let next_idx = match self.active_index {
            Some(i) => (i + 1) % self.fields.len(),
            None => 0,
        };
        self.set_focus(next_idx)
    }

    pub fn focus_prev(&mut self) -> Option<u32> {
        if self.fields.is_empty() {
            return None;
        }

        let prev_idx = match self.active_index {
            Some(0) | None => self.fields.len() - 1,
            Some(i) => i - 1,
        };
        self.set_focus(prev_idx)
    }

    fn set_focus(&mut self, index: usize) -> Option<u32> {
        for w in self.widgets.values_mut() {
            w.is_focused = false;
        }

        self.active_index = Some(index);
        let ref_id = self.fields[index];
        if let Some(widget) = self.widgets.get_mut(&ref_id) {
            widget.is_focused = true;
        }
        Some(ref_id)
    }

    pub fn current_focused_widget_mut(&mut self) -> Option<&mut TextInputWidget> {
        let ref_id = self.fields.get(self.active_index?)?;
        self.widgets.get_mut(ref_id)
    }
}
```

### 4.3 Form Submission Flow

```rust
// Location: crates/term-tui/src/forms/submission.rs

use std::collections::HashMap;
use term_engine::FastPathClient;

pub struct FormSubmitter;

impl FormSubmitter {
    pub async fn submit_form(
        action_url: &str,
        method: &str,
        form_data: &HashMap<String, String>,
    ) -> anyhow::Result<(String, u16)> {
        let client = FastPathClient::new(None);

        if method.eq_ignore_ascii_case("POST") {
            // Native POST dispatch via FastPathClient
            let res = client
                .post_form(action_url, form_data)
                .await?;
            Ok(res)
        } else {
            // GET dispatch: append URL query params
            let mut url = url::Url::parse(action_url)?;
            for (key, val) in form_data {
                url.query_pairs_mut().append_pair(key, val);
            }
            client.fetch_page_content(url.as_str()).await
        }
    }
}
```

---

## 5. Verification & Rollout Matrix

| Milestone | Deliverable | Verification Gate |
| :--- | :--- | :--- |
| **M1: Storage Integration** | Wire `DatabaseManager` into `FastPathClient` and `TuiApp` | `cargo test -p term-storage` & session persistence validation |
| **M2: Daemon Activation** | Implement `DaemonServer::run_loop` in `src/main.rs` under `tb daemon` | Connect via `nc -U /tmp/term_browser.sock` and execute JSON-RPC `PING` |
| **M3: Interactive Forms** | Connect `FocusRing` to `VirtualViewport` with `Tab` navigation | Test form input in DuckDuckGo search field within TUI |
