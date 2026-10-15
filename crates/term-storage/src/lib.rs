pub mod cookies;
pub mod history;
pub mod profiles;

pub use cookies::{CookieVault, StoredCookie};
pub use history::{HistoryRecord, HistoryVault};
pub use profiles::ProfileVault;

use rusqlite::Connection;

pub struct DatabaseManager {
    conn: Connection,
}

impl DatabaseManager {
    pub fn open_or_create(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        CookieVault::init_table(&conn)?;
        HistoryVault::init_table(&conn)?;
        ProfileVault::init_table(&conn)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}
