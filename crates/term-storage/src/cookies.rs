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

    pub fn get_for_domain(conn: &Connection, request_domain: &str) -> anyhow::Result<Vec<StoredCookie>> {
        let clean_req_domain = request_domain.trim_start_matches('.').to_ascii_lowercase();
        let mut stmt = conn.prepare(
            "SELECT domain, name, value, path, expires_at, is_secure, is_httponly, same_site
             FROM cookies",
        )?;
        let rows = stmt.query_map([], |row| {
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

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut cookies = Vec::new();
        for cookie in rows {
            let c = cookie?;
            // Filter expired cookies
            if let Some(exp) = c.expires_at {
                if exp > 0 && exp < now {
                    continue;
                }
            }
            let cookie_domain = c.domain.trim_start_matches('.').to_ascii_lowercase();
            // Match exact domain or subdomains (e.g. sub.example.com matches example.com)
            if clean_req_domain == cookie_domain
                || clean_req_domain.ends_with(&format!(".{}", cookie_domain))
            {
                cookies.push(c);
            }
        }
        Ok(cookies)
    }

    pub fn get_all(conn: &Connection) -> anyhow::Result<Vec<StoredCookie>> {
        let mut stmt = conn.prepare(
            "SELECT domain, name, value, path, expires_at, is_secure, is_httponly, same_site
             FROM cookies ORDER BY domain, name",
        )?;
        let rows = stmt.query_map([], |row| {
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

    pub fn delete_for_domain(conn: &Connection, domain: &str) -> anyhow::Result<usize> {
        let clean = domain.trim_start_matches('.').to_ascii_lowercase();
        let count = conn.execute(
            "DELETE FROM cookies WHERE domain = ?1 OR domain = ?2",
            params![clean, format!(".{}", clean)],
        )?;
        Ok(count)
    }

    pub fn clear_all(conn: &Connection) -> anyhow::Result<usize> {
        let count = conn.execute("DELETE FROM cookies", [])?;
        Ok(count)
    }

    /// Parses an HTTP `Set-Cookie` header into a `StoredCookie`
    pub fn parse_set_cookie(default_domain: &str, raw: &str) -> Option<StoredCookie> {
        let mut parts = raw.split(';').map(|s| s.trim()).filter(|s| !s.is_empty());
        let first = parts.next()?;
        let (name, value) = first.split_once('=')?;
        let name = name.trim();
        if name.is_empty() {
            return None;
        }

        let mut domain = default_domain.trim_start_matches('.').to_string();
        let mut path = "/".to_string();
        let mut expires_at = None;
        let mut is_secure = false;
        let mut is_httponly = false;
        let mut same_site = "Lax".to_string();

        for part in parts {
            if let Some((attr_key, attr_val)) = part.split_once('=') {
                let key_lower = attr_key.trim().to_ascii_lowercase();
                let val = attr_val.trim();
                match key_lower.as_str() {
                    "domain" => {
                        domain = val.trim_start_matches('.').to_string();
                    }
                    "path" => {
                        path = val.to_string();
                    }
                    "samesite" => {
                        same_site = val.to_string();
                    }
                    "max-age" => {
                        if let Ok(secs) = val.parse::<i64>() {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64;
                            expires_at = Some(now + secs);
                        }
                    }
                    _ => {}
                }
            } else {
                let flag = part.to_ascii_lowercase();
                if flag == "secure" {
                    is_secure = true;
                } else if flag == "httponly" {
                    is_httponly = true;
                }
            }
        }

        Some(StoredCookie {
            domain,
            name: name.to_string(),
            value: value.trim().to_string(),
            path,
            expires_at,
            is_secure,
            is_httponly,
            same_site,
        })
    }

    /// Formats a list of cookies into an HTTP `Cookie` header string
    pub fn build_cookie_header(cookies: &[StoredCookie]) -> Option<String> {
        if cookies.is_empty() {
            return None;
        }
        let pairs: Vec<String> = cookies
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect();
        Some(pairs.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_store_cookie() {
        let conn = Connection::open_in_memory().unwrap();
        CookieVault::init_table(&conn).unwrap();

        let raw_header = "rack.session=ABC123xyz; path=/; HttpOnly; Secure; SameSite=Strict";
        let cookie = CookieVault::parse_set_cookie("the-internet.herokuapp.com", raw_header)
            .expect("Should parse");

        assert_eq!(cookie.name, "rack.session");
        assert_eq!(cookie.value, "ABC123xyz");
        assert_eq!(cookie.domain, "the-internet.herokuapp.com");
        assert_eq!(cookie.path, "/");
        assert!(cookie.is_secure);
        assert!(cookie.is_httponly);
        assert_eq!(cookie.same_site, "Strict");

        CookieVault::upsert(&conn, &cookie).unwrap();

        let retrieved = CookieVault::get_for_domain(&conn, "the-internet.herokuapp.com").unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].name, "rack.session");

        let header = CookieVault::build_cookie_header(&retrieved).unwrap();
        assert_eq!(header, "rack.session=ABC123xyz");
    }

    #[test]
    fn test_domain_subdomain_matching() {
        let conn = Connection::open_in_memory().unwrap();
        CookieVault::init_table(&conn).unwrap();

        let parent_cookie = StoredCookie {
            domain: "example.com".to_string(),
            name: "auth_token".to_string(),
            value: "tok_parent".to_string(),
            path: "/".to_string(),
            expires_at: None,
            is_secure: true,
            is_httponly: true,
            same_site: "Lax".to_string(),
        };
        CookieVault::upsert(&conn, &parent_cookie).unwrap();

        // sub.example.com should inherit example.com cookie
        let sub_matches = CookieVault::get_for_domain(&conn, "sub.example.com").unwrap();
        assert_eq!(sub_matches.len(), 1);
        assert_eq!(sub_matches[0].name, "auth_token");

        // other.com should not match
        let other = CookieVault::get_for_domain(&conn, "other.com").unwrap();
        assert_eq!(other.len(), 0);
    }

    #[test]
    fn test_expired_cookies_filtered() {
        let conn = Connection::open_in_memory().unwrap();
        CookieVault::init_table(&conn).unwrap();

        let past_time = 1_000_000; // year 1970
        let expired = StoredCookie {
            domain: "test.com".to_string(),
            name: "old_cookie".to_string(),
            value: "expired_val".to_string(),
            path: "/".to_string(),
            expires_at: Some(past_time),
            is_secure: false,
            is_httponly: false,
            same_site: "Lax".to_string(),
        };
        CookieVault::upsert(&conn, &expired).unwrap();

        let cookies = CookieVault::get_for_domain(&conn, "test.com").unwrap();
        assert_eq!(cookies.len(), 0);
    }

    #[test]
    fn test_clear_all_cookies() {
        let conn = Connection::open_in_memory().unwrap();
        CookieVault::init_table(&conn).unwrap();

        let c1 = StoredCookie {
            domain: "site1.com".to_string(),
            name: "c1".to_string(),
            value: "v1".to_string(),
            path: "/".to_string(),
            expires_at: None,
            is_secure: false,
            is_httponly: false,
            same_site: "Lax".to_string(),
        };
        let c2 = StoredCookie {
            domain: "site2.com".to_string(),
            name: "c2".to_string(),
            value: "v2".to_string(),
            path: "/".to_string(),
            expires_at: None,
            is_secure: false,
            is_httponly: false,
            same_site: "Lax".to_string(),
        };
        CookieVault::upsert(&conn, &c1).unwrap();
        CookieVault::upsert(&conn, &c2).unwrap();

        let all = CookieVault::get_all(&conn).unwrap();
        assert_eq!(all.len(), 2);

        let deleted = CookieVault::clear_all(&conn).unwrap();
        assert_eq!(deleted, 2);

        let all_after = CookieVault::get_all(&conn).unwrap();
        assert_eq!(all_after.len(), 0);
    }
}
