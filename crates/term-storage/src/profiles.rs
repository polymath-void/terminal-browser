use rusqlite::{params, Connection};
use term_core::SessionProfile;

pub struct ProfileVault;

impl ProfileVault {
    pub fn init_table(conn: &Connection) -> anyhow::Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agent_profiles (
                profile_id TEXT PRIMARY KEY,
                user_agent TEXT NOT NULL,
                custom_headers TEXT,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    pub fn save_profile(conn: &Connection, profile: &SessionProfile) -> anyhow::Result<()> {
        let headers_json = serde_json::to_string(&profile.custom_headers)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO agent_profiles (profile_id, user_agent, custom_headers, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![profile.profile_id, profile.user_agent, headers_json, now],
        )?;
        Ok(())
    }
}
