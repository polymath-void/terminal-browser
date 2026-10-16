use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryRecord {
    pub url: String,
    pub title: Option<String>,
    pub visit_count: u32,
    pub last_visited_at: i64,
}

pub struct HistoryVault;

impl HistoryVault {
    pub fn init_table(conn: &Connection) -> anyhow::Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT UNIQUE NOT NULL,
                title TEXT,
                visit_count INTEGER NOT NULL DEFAULT 1,
                last_visited_at INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    pub fn record_visit(conn: &Connection, url: &str, title: Option<&str>) -> anyhow::Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO history (url, title, visit_count, last_visited_at)
             VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(url) DO UPDATE SET
                visit_count = visit_count + 1,
                title = COALESCE(?2, title),
                last_visited_at = ?3",
            params![url, title, now],
        )?;
        Ok(())
    }

    pub fn query_recent(conn: &Connection, limit: usize) -> anyhow::Result<Vec<HistoryRecord>> {
        let mut stmt = conn.prepare(
            "SELECT url, title, visit_count, last_visited_at 
             FROM history ORDER BY last_visited_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(HistoryRecord {
                url: row.get(0)?,
                title: row.get(1)?,
                visit_count: row.get(2)?,
                last_visited_at: row.get(3)?,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }
}
