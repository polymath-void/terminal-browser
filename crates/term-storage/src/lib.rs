pub mod bookmarks;
pub mod cookies;
pub mod history;
pub mod profiles;

pub use bookmarks::{BookmarkRecord, BookmarkVault};
pub use cookies::{CookieVault, StoredCookie};
pub use history::{HistoryRecord, HistoryVault};
pub use profiles::ProfileVault;

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
            let _ = termux_prefix;
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

        CookieVault::init_table(&conn)?;
        HistoryVault::init_table(&conn)?;
        ProfileVault::init_table(&conn)?;
        BookmarkVault::init_table(&conn)?;

        Ok(Self {
            conn,
            db_path: PathBuf::from(path_str),
        })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_manager_in_memory() {
        let db = DatabaseManager::open_or_create(":memory:").expect("Failed to open in-memory db");
        BookmarkVault::add(db.conn(), "https://example.com", "Example").expect("Failed to add bookmark");
        let bookmarks = BookmarkVault::list(db.conn()).expect("Failed to list bookmarks");
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].0, "Example");
        assert_eq!(bookmarks[0].1, "https://example.com");

        HistoryVault::record_visit(db.conn(), "https://example.com", Some("Example")).expect("Failed to record visit");
        HistoryVault::record_visit(db.conn(), "https://example.com", Some("Example")).expect("Failed to record visit");
        let history = HistoryVault::query_recent(db.conn(), 10).expect("Failed to query history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].visit_count, 2);
    }
}
