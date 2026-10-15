---
name: tb-store6-storage-vault
description: >-
  STORE-6 Storage & Vault Engineer for terminal-browser. Activate when implementing or
  modifying crates/term-storage: SQLite vault initialization, DDL migration, cookie CRUD,
  FTS5 full-text history search, agent profile management, and OS-native encrypted
  secret storage (Windows DPAPI, Linux Secret Service, macOS Keychain, Android AES-256-GCM).
---

# STORE-6: Storage & Vault Engineer

## Role
Owns `crates/term-storage`. All persistence: cookies, history, FTS5 search, session profiles,
and encrypted secret storage across all four target platforms.

## Blueprint Sources
- [missing-blueprints.md](../../../missing-blueprints.md): §4 Persistence & Session Vault Schema
- [PLATFORM_ARCHITECTURE_AND_IMPLEMENTATION_BLUEPRINTS.md](../../../PLATFORM_ARCHITECTURE_AND_IMPLEMENTATION_BLUEPRINTS.md): §6 SQLite Schema

## Crate Structure
```
crates/term-storage/
├── src/
│   ├── lib.rs
│   ├── db.rs              # Database init, migrations, connection pool
│   ├── cookies.rs         # Cookie CRUD, domain-scoped header builder
│   ├── history.rs         # History insert, FTS5 search, dedup
│   ├── profiles.rs        # Agent profile CRUD and import/export
│   ├── cache.rs           # ETag / Last-Modified cache metadata
│   └── encryption/
│       ├── mod.rs         # Platform dispatch
│       ├── dpapi.rs       # Windows DPAPI (CryptProtectData)
│       ├── secret_service.rs  # Linux DBus Secret Service
│       ├── keychain.rs    # macOS security-framework
│       └── aes_fallback.rs    # Termux / Android AES-256-GCM
```

## SQLite Schema (Full DDL)
```sql
-- Cookie Vault
CREATE TABLE IF NOT EXISTS cookies (
    domain      TEXT    NOT NULL,
    name        TEXT    NOT NULL,
    value       TEXT    NOT NULL,
    path        TEXT    NOT NULL DEFAULT '/',
    expires_at  INTEGER,
    is_secure   BOOLEAN NOT NULL DEFAULT 1,
    is_httponly BOOLEAN NOT NULL DEFAULT 0,
    same_site   TEXT    NOT NULL DEFAULT 'Lax',
    PRIMARY KEY (domain, name, path)
);

-- Browsing History
CREATE TABLE IF NOT EXISTS history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    url             TEXT    UNIQUE NOT NULL,
    title           TEXT,
    visit_count     INTEGER NOT NULL DEFAULT 1,
    last_visited_at INTEGER NOT NULL
);

-- FTS5 Full-Text Search (page content indexing)
CREATE VIRTUAL TABLE IF NOT EXISTS page_content_fts USING fts5(
    url     UNINDEXED,
    title,
    body_markdown,
    tokenize = "trigram"
);

-- Named Agent Profiles
CREATE TABLE IF NOT EXISTS agent_profiles (
    profile_id      TEXT    PRIMARY KEY,
    user_agent      TEXT    NOT NULL,
    custom_headers  TEXT,   -- JSON blob {"Key": "Value"}
    created_at      INTEGER NOT NULL
);

-- HTTP Cache Metadata
CREATE TABLE IF NOT EXISTS cache_metadata (
    url             TEXT    PRIMARY KEY,
    etag            TEXT,
    last_modified   TEXT,
    cached_at       INTEGER NOT NULL,
    cache_file_path TEXT
);
```

## Cookie API
```rust
pub struct CookieStore {
    conn: rusqlite::Connection,
}

impl CookieStore {
    pub fn upsert(&self, cookie: &Cookie) -> anyhow::Result<()>;
    pub fn for_url(&self, url: &url::Url) -> anyhow::Result<Vec<Cookie>>;
    pub fn cookie_header(&self, url: &url::Url) -> anyhow::Result<String>;
    pub fn import_from_json(&self, path: &std::path::Path) -> anyhow::Result<usize>;
    pub fn export_to_json(&self, path: &std::path::Path) -> anyhow::Result<usize>;
}
```

## FTS5 History Search API
```rust
pub fn search_history(conn: &rusqlite::Connection, query: &str) -> anyhow::Result<Vec<HistoryEntry>> {
    let sql = "
        SELECT h.url, h.title, h.visit_count, h.last_visited_at,
               snippet(page_content_fts, 2, '<b>', '</b>', '...', 10) AS excerpt
        FROM page_content_fts fts
        JOIN history h ON h.url = fts.url
        WHERE page_content_fts MATCH ?1
        ORDER BY rank
        LIMIT 25
    ";
    // ... execute and map rows
}
```

## Platform Encryption Dispatch
```rust
pub fn encrypt_secret(plaintext: &[u8]) -> anyhow::Result<Vec<u8>> {
    #[cfg(target_os = "windows")]
    return dpapi::encrypt(plaintext);

    #[cfg(target_os = "linux")]
    return secret_service::encrypt(plaintext);

    #[cfg(target_os = "macos")]
    return keychain::encrypt(plaintext);

    #[cfg(target_os = "android")]
    return aes_fallback::encrypt(plaintext);
}
```

## Cargo.toml for term-storage
```toml
[package]
name = "term-storage"
version = "0.1.0"
edition = "2021"

[dependencies]
term-core = { path = "../term-core" }
rusqlite = { version = "0.32", features = ["bundled-full"] }
serde_json = "1"
url = "2"
anyhow = "1"
aes-gcm = "0.10"
rand = "0.8"
tracing = "0.1"

[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { version = "0.59", features = ["Win32_Security_Cryptography"] }

[target.'cfg(target_os = "linux")'.dependencies]
secret-service = "4"

[target.'cfg(target_os = "macos")'.dependencies]
security-framework = "3"
```

## Validation
```bash
cargo check -p term-storage
cargo test -p term-storage
# Verify FTS5 is bundled:
cargo run -p term-storage --example fts5_search_test
```
