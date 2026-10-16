use crate::codec::LengthPrefixedJsonCodec;
use crate::transport::get_socket_path;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tokio_util::codec::Framed;

pub struct DaemonServer {
    socket_path: String,
    db_manager: Arc<Mutex<term_storage::DatabaseManager>>,
    pid_path: PathBuf,
}

impl DaemonServer {
    pub fn new() -> anyhow::Result<Self> {
        let db = Arc::new(Mutex::new(term_storage::DatabaseManager::open_default()?));
        let base_dir = term_storage::DatabaseManager::get_default_db_path()
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let pid_path = base_dir.join("daemon.pid");

        Ok(Self {
            socket_path: get_socket_path(),
            db_manager: db,
            pid_path,
        })
    }

    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    pub async fn run_loop(&self) -> anyhow::Result<()> {
        let p = Path::new(&self.socket_path);
        if p.exists() {
            let _ = std::fs::remove_file(p);
        }

        // Write PID file
        let pid = std::process::id();
        let _ = std::fs::write(&self.pid_path, pid.to_string());
        println!("🚀 Terminal Browser Daemon booted (PID: {})", pid);
        println!("🌐 IPC Socket listening at: {}", self.socket_path);

        let listener = UnixListener::bind(&self.socket_path)?;

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let db = Arc::clone(&self.db_manager);
                    tokio::spawn(async move {
                        let mut framed = Framed::new(stream, LengthPrefixedJsonCodec);
                        while let Some(res) = framed.next().await {
                            match res {
                                Ok(request) => {
                                    let response = Self::handle_rpc_request(request, &db).await;
                                    if let Err(e) = framed.send(response).await {
                                        tracing::error!("Failed to send IPC response: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("IPC frame decode error: {}", e);
                                    break;
                                }
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
        db: &Arc<Mutex<term_storage::DatabaseManager>>,
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
            "STATUS" => {
                let pid = std::process::id();
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "status": "healthy",
                        "pid": pid,
                        "uptime_sec": 0
                    }
                })
            }
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
            "HISTORY" => {
                let guard = db.lock().await;
                match term_storage::HistoryVault::query_recent(guard.conn(), 20) {
                    Ok(records) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": records
                    }),
                    Err(e) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32002, "message": e.to_string() }
                    }),
                }
            }
            "BOOKMARKS" => {
                let guard = db.lock().await;
                match term_storage::BookmarkVault::list(guard.conn()) {
                    Ok(records) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": records
                    }),
                    Err(e) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32003, "message": e.to_string() }
                    }),
                }
            }
            "SHUTDOWN" => {
                tokio::spawn(async {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    std::process::exit(0);
                });
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": "Shutting down daemon"
                })
            }
            unknown => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {}", unknown) }
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_daemon_rpc_ping_and_status() {
        let db = Arc::new(Mutex::new(term_storage::DatabaseManager::open_or_create(":memory:").unwrap()));
        let ping_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "PING"
        });
        let res = DaemonServer::handle_rpc_request(ping_req, &db).await;
        assert_eq!(res["result"], "PONG");
        assert_eq!(res["id"], 1);

        let status_req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "STATUS"
        });
        let status_res = DaemonServer::handle_rpc_request(status_req, &db).await;
        assert_eq!(status_res["result"]["status"], "healthy");
    }
}
