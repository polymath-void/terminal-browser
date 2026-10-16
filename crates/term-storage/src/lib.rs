pub mod cookies;
pub mod history;
pub mod profiles;

pub use cookies::{CookieVault, StoredCookie};
pub use history::{HistoryRecord, HistoryVault};
pub use profiles::ProfileVault;

use rusqlite::Connection;
use std::path::PathBuf;

pub struct DatabaseManager {
    conn: Connection,
}

impl DatabaseManager {
    /// Returns the standard path to `vault.db` in `~/.terminal-browser/vault.db`
    pub fn default_vault_path() -> PathBuf {
        if let Ok(custom) = std::env::var("TB_VAULT_PATH") {
            return PathBuf::from(custom);
        }
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let dir = PathBuf::from(home).join(".terminal-browser");
        let _ = std::fs::create_dir_all(&dir);
        dir.join("vault.db")
    }

    /// Opens the default SQLite vault at `~/.terminal-browser/vault.db`
    pub fn open_default() -> anyhow::Result<Self> {
        let path = Self::default_vault_path();
        Self::open_or_create(path.to_str().unwrap_or("vault.db"))
    }

    /// Opens or creates an SQLite vault at a specified path
    pub fn open_or_create(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        CookieVault::init_table(&conn)?;
        HistoryVault::init_table(&conn)?;
        ProfileVault::init_table(&conn)?;
        Ok(Self { conn })
    }

    /// Opens an in-memory SQLite database for ephemeral sessions / tests
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        CookieVault::init_table(&conn)?;
        HistoryVault::init_table(&conn)?;
        ProfileVault::init_table(&conn)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}
