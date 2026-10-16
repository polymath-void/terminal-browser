use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredCookie {
    pub domain: String,
    pub name: String,
    pub value: String,
    pub path: String,
    pub expires_at: Option<i64>,
    pub is_secure: bool,
    pub is_httponly: bool,
    pub same_site: String,
}

pub struct CookieVault;

impl CookieVault {
    pub fn init_table(conn: &Connection) -> anyhow::Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cookies (
                domain TEXT NOT NULL,
                name TEXT NOT NULL,
                value TEXT NOT NULL,
                path TEXT NOT NULL DEFAULT '/',
                expires_at INTEGER,
                is_secure BOOLEAN NOT NULL DEFAULT 1,
                is_httponly BOOLEAN NOT NULL DEFAULT 0,
                same_site TEXT NOT NULL DEFAULT 'Lax',
                PRIMARY KEY (domain, name, path)
            )",
            [],
        )?;
        Ok(())
    }

    pub fn upsert(conn: &Connection, cookie: &StoredCookie) -> anyhow::Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO cookies 
            (domain, name, value, path, expires_at, is_secure, is_httponly, same_site)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                cookie.domain,
                cookie.name,
                cookie.value,
                cookie.path,
                cookie.expires_at,
                cookie.is_secure,
                cookie.is_httponly,
                cookie.same_site
            ],
        )?;
        Ok(())
    }

    pub fn get_for_domain(conn: &Connection, domain: &str) -> anyhow::Result<Vec<StoredCookie>> {
        let mut stmt = conn.prepare(
            "SELECT domain, name, value, path, expires_at, is_secure, is_httponly, same_site
             FROM cookies WHERE ?1 LIKE '%' || domain",
        )?;
        let rows = stmt.query_map(params![domain], |row| {
            Ok(StoredCookie {
                domain: row.get(0)?,
                name: row.get(1)?,
                value: row.get(2)?,
                path: row.get(3)?,
                expires_at: row.get(4)?,
                is_secure: row.get(5)?,
                is_httponly: row.get(6)?,
                same_site: row.get(7)?,
            })
        })?;

        let mut cookies = Vec::new();
        for cookie in rows {
            cookies.push(cookie?);
        }
        Ok(cookies)
    }
}
