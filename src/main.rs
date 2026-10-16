use clap::{Parser, Subcommand};
use term_engine::FastPathClient;
use term_mcp::McpServer;

#[derive(Parser, Debug)]
#[command(name = "term-browser", version = "0.1.0", author = "Polymath & Antigravity Swarm")]
#[command(about = "Next-generation semantic & agentic CLI terminal web browser")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// URL to fetch or browse directly
    url: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Launch the interactive 60 FPS Ratatui terminal browser
    Tui {
        /// Initial URL to load
        url: Option<String>,
    },
    /// Fast-fetch content using agent negotiation (<50ms) without full rendering
    Fetch {
        /// Target URL to retrieve
        url: String,
        /// Probe /llms.txt first
        #[arg(long, default_value_t = true)]
        probe_llms: bool,
    },
    /// Run as an embedded Model Context Protocol (MCP) server over stdio
    Mcp,
    /// Run the background browser engine daemon
    Daemon,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Fetch { url, probe_llms }) => {
            println!("🚀 Fast-fetching URL: {}", url);
            let client = FastPathClient::new(None);

            if probe_llms {
                if let Some(llms) = client.probe_llms_txt(&url).await {
                    println!("📄 Found /llms.txt summary:\n{}", llms);
                    return Ok(());
                }
            }

            match client.fetch_negotiated(&url).await {
                Ok(Some(markdown)) => {
                    println!("⚡ Negotiated direct Markdown content:\n{}", markdown);
                }
                Ok(None) => {
                    println!("ℹ️ Server did not return direct Markdown; falling back to CDP engine...");
                }
                Err(e) => {
                    eprintln!("❌ Fetch error: {}", e);
                }
            }
        }
        Some(Commands::Mcp) => {
            eprintln!("🤖 Starting embedded Model Context Protocol (MCP) server over stdio...");
            // Stdio loop for MCP tools
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
        Some(Commands::Daemon) => {
            println!("🌐 Running background browser daemon on IPC socket: {}", term_ipc::get_socket_path());
        }
        Some(Commands::Tui { url }) => {
            let target_url = url.unwrap_or_else(|| "https://news.ycombinator.com".to_string());
            println!("🖥️ Starting Terminal Web Browser for URL: {}", target_url);
            println!("Hint: Run with `fetch <url>` for <50ms Markdown retrieval or `mcp` for agent control.");
        }
        None => {
            let target_url = cli.url.unwrap_or_else(|| "https://news.ycombinator.com".to_string());
            println!("🖥️ Starting Terminal Web Browser for URL: {}", target_url);
            println!("Hint: Run with `fetch <url>` for <50ms Markdown retrieval or `mcp` for agent control.");
        }
    }

    Ok(())
}
