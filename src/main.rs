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
            let target_url = second.unwrap_or("https://news.ycombinator.com");
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
        "tui" => {
            let target_url = second.unwrap_or("https://news.ycombinator.com");
            let norm_url = normalize_url(target_url);
            let mut app = TuiApp::new();
            let _ = app.load_url(&norm_url).await;
            app.run_interactive().await?;
        }
        "" => {
            // Default: no args -> launch TUI with Hacker News
            let target_url = "https://news.ycombinator.com";
            let mut app = TuiApp::new();
            let _ = app.load_url(target_url).await;
            app.run_interactive().await?;
        }
        url_candidate => {
            // Check if second is "tui" or if first is a direct URL
            let target_url = if url_candidate == "tui" {
                second.unwrap_or("https://news.ycombinator.com")
            } else {
                url_candidate
            };
            let norm_url = normalize_url(target_url);
            let mut app = TuiApp::new();
            let _ = app.load_url(&norm_url).await;
            app.run_interactive().await?;
        }
    }

    Ok(())
}
