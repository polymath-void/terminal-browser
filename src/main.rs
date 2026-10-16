use clap::Parser;
use term_engine::FastPathClient;
use term_mcp::McpServer;
use term_storage::DatabaseManager;
use term_tui::TuiApp;

fn normalize_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{}", trimmed)
    }
}

#[derive(Parser, Debug)]
#[command(name = "tb", version = "0.1.0", author = "Polymath & Antigravity Swarm")]
#[command(about = "Next-generation semantic & agentic CLI terminal web browser")]
struct Cli {
    /// URL to browse, command (tui, fetch, login, session, mcp, daemon), or mode
    #[arg(value_name = "URL_OR_COMMAND")]
    first: Option<String>,

    /// Secondary argument (e.g. URL for fetch / login, or action for session)
    #[arg(value_name = "PARAM")]
    second: Option<String>,

    /// Tertiary argument (optional)
    #[arg(value_name = "EXTRA")]
    third: Option<String>,

    /// Username for login form submission
    #[arg(long, short = 'u')]
    user: Option<String>,

    /// Password for login form submission
    #[arg(long, short = 'p')]
    pass: Option<String>,

    /// Launch interactive 60 FPS TUI browser
    #[arg(long, short = 't')]
    tui: bool,

    /// Probe /llms.txt first during fetch
    #[arg(long, default_value_t = true)]
    probe_llms: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    let first = cli.first.as_deref().unwrap_or("");
    let second = cli.second.as_deref();

    match first {
        "fetch" => {
            let target_url = match second {
                Some(url) => url,
                None => {
                    eprintln!("❌ Error: URL required for 'fetch'. Usage: tb fetch <URL>");
                    return Ok(());
                }
            };
            let norm_url = normalize_url(target_url);
            println!("🚀 Fast-fetching URL: {}", norm_url);
            let client = FastPathClient::new(None);

            if let Some(cookie_hdr) = client.get_cookie_header_for_url(&norm_url) {
                println!("🍪 Attaching active vault cookie: {}", cookie_hdr);
            }

            if cli.probe_llms {
                if let Some(llms) = client.probe_llms_txt(&norm_url).await {
                    println!("📄 Found /llms.txt summary:\n{}", llms);
                    return Ok(());
                }
            }

            match client.fetch_negotiated(&norm_url).await {
                Ok(Some(markdown)) => {
                    println!("⚡ Negotiated direct Markdown content:\n{}", markdown);
                }
                Ok(None) => {
                    println!("ℹ️ Server did not return direct Markdown; fetching page text...");
                    let (content, status) = client.fetch_page_content(&norm_url).await?;
                    println!("HTTP Status: {}\n{}", status, content);
                }
                Err(e) => {
                    eprintln!("❌ Fetch error: {}", e);
                }
            }
        }
        "login" => {
            let target_url = second.unwrap_or("https://the-internet.herokuapp.com/authenticate");
            let username = cli.user.as_deref().unwrap_or("tomsmith");
            let password = cli.pass.as_deref().unwrap_or("SuperSecretPassword!");
            let norm_url = normalize_url(target_url);

            println!("🔐 Submitting authenticated login to: {}", norm_url);
            println!("👤 User: {}", username);

            let client = FastPathClient::new(None);
            let form_data = [("username", username), ("password", password)];

            let start = std::time::Instant::now();
            match client.post_form(&norm_url, &form_data).await {
                Ok((content, status, cookies)) => {
                    let elapsed = start.elapsed();
                    println!("✅ Login complete in {:.2?} (HTTP {})", elapsed, status);
                    println!("🍪 Captured & Stored {} Cookie(s) in SQLite Vault:", cookies.len());
                    for c in &cookies {
                        println!("   • {} = {} (Domain: {}, Secure: {}, HttpOnly: {})",
                            c.name, c.value, c.domain, c.is_secure, c.is_httponly);
                    }
                    println!("\n--- Authenticated Response Snippet ---\n");
                    let preview: String = content.lines().take(20).collect::<Vec<_>>().join("\n");
                    println!("{}", preview);
                }
                Err(e) => {
                    eprintln!("❌ Login failed: {}", e);
                }
            }
        }
        "session" => {
            let action = second.unwrap_or("list");
            let client = FastPathClient::new(None);

            match action {
                "clear" => {
                    let count = client.clear_all_cookies()?;
                    println!("🧹 Cleared {} cookie(s) from SQLite CookieVault.", count);
                }
                "list" => {
                    let cookies = client.get_all_stored_cookies();
                    let db_path = DatabaseManager::default_vault_path();
                    println!("📦 SQLite CookieVault: {}", db_path.display());
                    if cookies.is_empty() {
                        println!("ℹ️ No active cookies stored in vault.");
                    } else {
                        println!("🍪 Total Active Cookies: {}", cookies.len());
                        for (i, c) in cookies.iter().enumerate() {
                            let exp = c.expires_at.map(|e| e.to_string()).unwrap_or_else(|| "Session".to_string());
                            println!("   [{}] Domain: {} | Name: {} | Value: {} | Path: {} | Exp: {} | Secure: {} | HttpOnly: {}",
                                i + 1, c.domain, c.name, c.value, c.path, exp, c.is_secure, c.is_httponly);
                        }
                    }
                }
                "test" => {
                    println!("🧪 Running Live End-to-End Login & Session Vault Verification...");
                    let secure_url = "https://the-internet.herokuapp.com/secure";
                    let auth_url = "https://the-internet.herokuapp.com/authenticate";

                    // Step 1: Clear session to test unauthenticated baseline
                    println!("\n[1/5] Clearing existing session cookies...");
                    let _ = client.clear_all_cookies();

                    // Step 2: Request protected area without cookies
                    println!("[2/5] Testing unauthenticated access to: {}", secure_url);
                    let (content_unauth, status_unauth) = client.fetch_page_content(secure_url).await?;
                    let is_logged_in_unauth = content_unauth.contains("Secure Area") && !content_unauth.contains("login");
                    println!("      Status: {} | Secure Area visible: {}", status_unauth, is_logged_in_unauth);
                    assert!(!is_logged_in_unauth, "Unauthenticated access should be redirected to login!");
                    println!("      🔒 Access denied as expected (redirected to login).");

                    // Step 3: Post credentials to login endpoint
                    println!("\n[3/5] Posting credentials to: {}", auth_url);
                    println!("      Credentials: username='tomsmith', password='SuperSecretPassword!'");
                    let (content_login, status_login, cookies) = client.post_form(auth_url, &[
                        ("username", "tomsmith"),
                        ("password", "SuperSecretPassword!"),
                    ]).await?;

                    println!("      Login HTTP Status: {}", status_login);
                    println!("      Cookies Captured: {}", cookies.len());
                    for c in &cookies {
                        println!("      🍪 Stored Cookie: {} (Domain: {})", c.name, c.domain);
                    }
                    let login_success = content_login.contains("You logged into a secure area") || content_login.contains("Secure Area");
                    println!("      Session authenticated: {}", login_success);

                    // Step 4: Verify persistent cookie access on subsequent request
                    println!("\n[4/5] Testing subsequent request to {} using stored SQLite cookie...", secure_url);
                    let (content_auth, status_auth) = client.fetch_page_content(secure_url).await?;
                    let has_secure_badge = content_auth.contains("Secure Area") || content_auth.contains("You logged into a secure area");
                    println!("      Status: {} | Authenticated content rendered: {}", status_auth, has_secure_badge);

                    // Step 5: Verify SQLite vault state
                    println!("\n[5/5] Inspecting SQLite CookieVault state...");
                    let active_cookies = client.get_all_stored_cookies();
                    println!("      Active Cookies in Vault: {}", active_cookies.len());
                    for c in &active_cookies {
                        println!("      • {} (Value preview: {}...)", c.name, &c.value.chars().take(25).collect::<String>());
                    }

                    if has_secure_badge && !active_cookies.is_empty() {
                        println!("\n🎉 SUCCESS: Live Login Session verified end-to-end with SQLite vault persistence!");
                    } else {
                        eprintln!("\n⚠️ Verification finished with unexpected content.");
                    }
                }
                other => {
                    println!("Unknown session command: '{}'. Use 'list', 'clear', or 'test'.", other);
                }
            }
        }
        "mcp" => {
            eprintln!("🤖 Starting embedded Model Context Protocol (MCP) server over stdio...");
            let mut line = String::new();
            let stdin = std::io::stdin();
            while stdin.read_line(&mut line)? > 0 {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    let res = McpServer::handle_tool_call("browser_snapshot", &serde_json::json!({}), None);
                    println!("{}", res);
                }
                line.clear();
            }
        }
        "daemon" => {
            println!("🌐 Running background browser daemon on IPC socket: {}", term_ipc::get_socket_path());
        }
        "open" | "detach" => {
            let target_url = match second {
                Some(url) => url,
                None => {
                    eprintln!("❌ Error: URL required for 'open' / 'detach'. Usage: tb open <URL>");
                    return Ok(());
                }
            };
            let norm_url = normalize_url(target_url);
            println!("🌐 Launching external headed browser for: {}", norm_url);
            term_engine::detach_headed(&norm_url).await?;
        }
        "act" => {
            let target_url = match second {
                Some(url) => url,
                None => {
                    eprintln!("❌ Error: URL required for 'act'. Usage: tb act <URL>");
                    return Ok(());
                }
            };
            let norm_url = normalize_url(target_url);
            println!("🌐 Executing headed browser detach for: {}", norm_url);
            term_engine::detach_headed(&norm_url).await?;
        }
        "tui" => {
            let mut app = TuiApp::new();
            if let Some(target_url) = second {
                let norm_url = normalize_url(target_url);
                let _ = app.load_url(&norm_url).await;
            } else {
                app.load_markdown_content(
                    "about:home",
                    "# Terminal Web Browser (tb)\n\nWelcome to **Terminal Browser**.\n\n### Navigation:\n- Press `/` to enter URL or search\n- Press `f` for link hints\n- Press `q` to quit\n\n### Quick Start:\nRun `tb <url>` to browse a website directly.",
                );
            }
            app.run_interactive().await?;
        }
        "" => {
            let mut app = TuiApp::new();
            app.load_markdown_content(
                "about:home",
                "# Terminal Web Browser (tb)\n\nWelcome to **Terminal Browser**.\n\n### Navigation:\n- Press `/` to enter URL or search\n- Press `f` for link hints\n- Press `q` to quit\n\n### Quick Start:\nRun `tb <url>` to browse a website directly.",
            );
            app.run_interactive().await?;
        }
        url_candidate => {
            let norm_url = normalize_url(url_candidate);
            let mut app = TuiApp::new();
            let _ = app.load_url(&norm_url).await;
            app.run_interactive().await?;
        }
    }

    Ok(())
}
