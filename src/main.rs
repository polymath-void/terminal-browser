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
    #[arg(long, default_value_t = false)]
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
            let server = term_ipc::DaemonServer::new()?;
            server.run_loop().await?;
        }
        "search" => {
            let query = match second {
                Some(q) => q,
                None => {
                    eprintln!("❌ Error: Search query required. Usage: tb search <query>");
                    return Ok(());
                }
            };
            println!("🔍 Searching web natively for: \"{}\"", query);
            let client = FastPathClient::new(None);
            let results = client.search_web_results(query).await?;
            println!("\nFound {} organic search results:\n", results.len());
            for r in &results {
                println!("  [{}] {} -> {}", r.rank, r.title, r.url);
            }
        }
        "research" => {
            let query = match second {
                Some(q) => q,
                None => {
                    eprintln!("❌ Error: Research query required. Usage: tb research <query>");
                    return Ok(());
                }
            };
            println!("================================================================================");
            println!("  TERMINAL BROWSER: NATIVE LIVE MULTI-TAB RESEARCH");
            println!("  Query: \"{}\"", query);
            println!("================================================================================");
            let client = FastPathClient::new(None);

            println!("\n[Tab 0] Search dispatched -> Gathering top 10 organic results...");
            let pages = client.research_workflow(query, 10).await?;

            println!("\n[Multi-Tab Execution Lifecycle Completed: {} Results Inspected]", pages.len());
            for p in &pages {
                if !p.is_blocked {
                    println!("\n▶ [Tab {}] {} (HTTP {})", p.rank, p.title, p.status);
                    println!("  URL: {}", p.url);
                    let snippet: String = p.content_preview.lines().filter(|l| !l.trim().is_empty()).take(3).collect::<Vec<_>>().join(" ");
                    let short_snip = if snippet.len() > 220 { format!("{}...", &snippet[..220]) } else { snippet };
                    println!("  Content: {}", short_snip);
                    println!("  ◀ Closed Tab {} -> Returned to Tab 0", p.rank);
                } else {
                    println!("\n▶ [Tab {}] {} (HTTP 403 / Cloudflare WAF)", p.rank, p.title);
                    println!("  URL: {}", p.url);
                    println!("  ⚠️ Block detected -> Headed Detach protocol available via 'tb open <url>'");
                    println!("  ◀ Closed Tab {} -> Returned to Tab 0", p.rank);
                }
            }
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
        "dump" => {
            let target_url = match second {
                Some(url) => url,
                None => {
                    eprintln!("❌ Error: URL required for 'dump'. Usage: tb dump <URL>");
                    return Ok(());
                }
            };
            let norm_url = normalize_url(target_url);
            let client = FastPathClient::new(None);
            let (content, _) = client.fetch_page_content(&norm_url).await?;
            println!("{}", content);
        }
        "bookmarks" => {
            let mut app = TuiApp::new();
            app.show_bookmarks();
            app.run_interactive().await?;
        }
        "history" => {
            let mut app = TuiApp::new();
            app.show_history();
            app.run_interactive().await?;
        }
        "tui" => {
            let mut app = TuiApp::new();
            if let Some(target) = second {
                if target.starts_with("http://") || target.starts_with("https://") || (!target.contains(' ') && target.contains('.')) {
                    let norm_url = normalize_url(target);
                    let _ = app.load_url(&norm_url).await;
                } else {
                    let _ = app.search_and_display(target).await;
                }
            }
            app.run_interactive().await?;
        }
        "" => {
            let mut app = TuiApp::new();
            app.run_interactive().await?;
        }
        url_candidate => {
            let mut app = TuiApp::new();
            if url_candidate.starts_with("http://") || url_candidate.starts_with("https://") || (!url_candidate.contains(' ') && url_candidate.contains('.')) {
                let norm_url = normalize_url(url_candidate);
                let _ = app.load_url(&norm_url).await;
            } else {
                let _ = app.search_and_display(url_candidate).await;
            }
            app.run_interactive().await?;
        }
    }

    Ok(())
}
