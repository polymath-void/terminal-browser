use clap::Parser;
use term_engine::FastPathClient;
use term_mcp::McpServer;
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
    /// URL to browse, command (tui, fetch, mcp, daemon), or mode
    #[arg(value_name = "URL_OR_COMMAND")]
    first: Option<String>,

    /// Secondary argument (e.g. URL for fetch / tui)
    #[arg(value_name = "PARAM")]
    second: Option<String>,

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
                    let (content, _) = client.fetch_page_content(&norm_url).await?;
                    println!("{}", content);
                }
                Err(e) => {
                    eprintln!("❌ Fetch error: {}", e);
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
