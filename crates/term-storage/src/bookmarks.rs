use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarkRecord {
    pub url: String,
    pub title: String,
    pub created_at: i64,
}

pub struct BookmarkVault;

impl BookmarkVault {
    pub fn init_table(conn: &Connection) -> anyhow::Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS bookmarks (
                url TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    pub fn add(conn: &Connection, url: &str, title: &str) -> anyhow::Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO bookmarks (url, title, created_at)
             VALUES (?1, ?2, ?3)",
            params![url, title, now],
        )?;
        Ok(())
    }

    pub fn remove(conn: &Connection, url: &str) -> anyhow::Result<()> {
        conn.execute("DELETE FROM bookmarks WHERE url = ?1", params![url])?;
        Ok(())
    }

    pub fn list(conn: &Connection) -> anyhow::Result<Vec<(String, String)>> {
        let mut stmt = conn.prepare("SELECT title, url FROM bookmarks ORDER BY created_at ASC")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }
}
