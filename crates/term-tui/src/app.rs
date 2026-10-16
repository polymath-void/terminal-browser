use crate::viewport::{RenderNode, VirtualViewport};
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::collections::HashMap;
use term_core::PageSnapshot;
use term_engine::FastPathClient;

struct TerminalGuard;
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen, Show);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    UrlBar,
    InPageSearch,
    VimiumHints,
    HelpModal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrowserTab {
    pub title: String,
    pub url: String,
    pub status_code: u16,
    pub viewport: VirtualViewport,
    pub link_targets: HashMap<u32, String>,
    pub history_back: Vec<String>,
    pub history_forward: Vec<String>,
    pub raw_content: String,
    pub is_reader_mode: bool,
}

impl Default for BrowserTab {
    fn default() -> Self {
        Self {
            title: "Home".to_string(),
            url: "about:home".to_string(),
            status_code: 200,
            viewport: VirtualViewport::new(),
            link_targets: HashMap::new(),
            history_back: Vec::new(),
            history_forward: Vec::new(),
            raw_content: String::new(),
            is_reader_mode: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TuiApp {
    pub tabs: Vec<BrowserTab>,
    pub active_tab: usize,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub input_cursor: usize,
    pub search_query: String,
    pub search_matches: Vec<usize>,
    pub current_match_idx: usize,
    pub status_message: String,
    pub memory_usage_mb: u64,
    pub current_hints: HashMap<u32, [char; 2]>,
    pub typed_hint: String,
    pub bookmarks: Vec<(String, String)>,

    // Compatibility mirrors
    pub current_url: String,
    pub status_code: u16,
    pub hint_mode: bool,
    pub viewport: VirtualViewport,
    pub link_targets: HashMap<u32, String>,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiApp {
    pub fn new() -> Self {
        let mut initial_tab = BrowserTab::default();
        let home_md = "# Terminal Web Browser (tb)\n\nWelcome to **Terminal Browser** — Fast, native terminal web browsing.\n\n### Navigation Shortcuts:\n- Press `/` or `o` : Open URL or Search web\n- Press `f`       : Activate Vimium 2-letter link hints\n- Press `b`       : Back in history\n- Press `B`       : Forward in history\n- Press `t`       : Open new tab\n- Press `w`       : Close active tab\n- Press `Tab`     : Switch to next tab\n- Press `?`       : Find in page (`n`/`N` for next/prev)\n- Press `r`       : Reload page\n- Press `R`       : Toggle Reader Mode\n- Press `m`       : Bookmark page\n- Press `M`       : View Bookmarks\n- Press `h`       : Show Help cheatsheet\n- Press `q`       : Quit browser\n\n### Quick Start:\nType `/` and enter any URL (e.g. `news.ycombinator.com`) or search query (e.g. `rust lang tutorials`).";

        let (nodes, _) = Self::parse_markdown(home_md, &mut initial_tab.link_targets);
        initial_tab.viewport.set_nodes(nodes);
        initial_tab.raw_content = home_md.to_string();

        let current_url = initial_tab.url.clone();
        let status_code = initial_tab.status_code;
        let viewport = initial_tab.viewport.clone();
        let link_targets = initial_tab.link_targets.clone();

        Self {
            tabs: vec![initial_tab],
            active_tab: 0,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            input_cursor: 0,
            search_query: String::new(),
            search_matches: Vec::new(),
            current_match_idx: 0,
            status_message: "Ready. Press [/] to navigate or search, [h] for help".to_string(),
            memory_usage_mb: 18,
            current_hints: HashMap::new(),
            typed_hint: String::new(),
            bookmarks: if let Ok(db) = term_storage::DatabaseManager::open_default() {
                if let Ok(bms) = term_storage::BookmarkVault::list(db.conn()) {
                    if !bms.is_empty() {
                        bms
                    } else {
                        vec![
                            ("DuckDuckGo".to_string(), "https://lite.duckduckgo.com/lite/".to_string()),
                            ("Hacker News".to_string(), "https://news.ycombinator.com".to_string()),
                            ("Wikipedia".to_string(), "https://en.wikipedia.org".to_string()),
                        ]
                    }
                } else {
                    vec![
                        ("DuckDuckGo".to_string(), "https://lite.duckduckgo.com/lite/".to_string()),
                        ("Hacker News".to_string(), "https://news.ycombinator.com".to_string()),
                        ("Wikipedia".to_string(), "https://en.wikipedia.org".to_string()),
                    ]
                }
            } else {
                vec![
                    ("DuckDuckGo".to_string(), "https://lite.duckduckgo.com/lite/".to_string()),
                    ("Hacker News".to_string(), "https://news.ycombinator.com".to_string()),
                    ("Wikipedia".to_string(), "https://en.wikipedia.org".to_string()),
                ]
            },
            current_url,
            status_code,
            hint_mode: false,
            viewport,
            link_targets,
        }
    }

    fn sync_mirrors(&mut self) {
        if let Some(tab) = self.tabs.get(self.active_tab) {
            self.current_url = tab.url.clone();
            self.status_code = tab.status_code;
            self.viewport = tab.viewport.clone();
            self.link_targets = tab.link_targets.clone();
        }
        self.hint_mode = self.input_mode == InputMode::VimiumHints;
    }

    pub fn current_tab(&self) -> &BrowserTab {
        &self.tabs[self.active_tab]
    }

    pub fn current_tab_mut(&mut self) -> &mut BrowserTab {
        &mut self.tabs[self.active_tab]
    }

    pub fn load_snapshot(&mut self, snapshot: PageSnapshot) {
        let tab = self.current_tab_mut();
        tab.url = snapshot.url.clone();
        tab.status_code = snapshot.http_status;
        tab.title = snapshot
            .title
            .as_ref()
            .filter(|t| !t.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| Self::url_domain(&snapshot.url));

        let mut render_nodes = Vec::new();
        for node in snapshot.nodes {
            let label = node.name.unwrap_or_else(|| node.role.clone());
            let line = Line::from(vec![
                Span::styled(format!("[{}] ", node.role), Style::default().fg(Color::Cyan)),
                Span::raw(label),
            ]);
            render_nodes.push(RenderNode {
                ref_id: node.ref_id,
                height: 1,
                hint_label: None,
                content: line,
            });
        }
        tab.viewport.set_nodes(render_nodes);
        self.sync_mirrors();
    }

    fn url_domain(url: &str) -> String {
        if let Ok(parsed) = url::Url::parse(url) {
            parsed.host_str().unwrap_or(url).to_string()
        } else {
            url.chars().take(20).collect()
        }
    }

    pub fn parse_markdown(content: &str, link_targets: &mut HashMap<u32, String>) -> (Vec<RenderNode>, Option<String>) {
        let mut render_nodes = Vec::new();
        link_targets.clear();
        let mut next_ref_id: u32 = 1;
        let mut extracted_title: Option<String> = None;

        for line_str in content.lines() {
            let trimmed = line_str.trim();
            if trimmed.is_empty() {
                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::raw(""),
                });
                continue;
            }

            if trimmed.starts_with("# ") {
                let title = &trimmed[2..];
                if extracted_title.is_none() {
                    extracted_title = Some(title.to_string());
                }
                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::from(vec![
                        Span::styled("█ ", Style::default().fg(Color::Yellow)),
                        Span::styled(title.to_string(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    ]),
                });
            } else if trimmed.starts_with("## ") {
                let heading = &trimmed[3..];
                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::from(vec![
                        Span::styled("## ", Style::default().fg(Color::Cyan)),
                        Span::styled(heading.to_string(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    ]),
                });
            } else if trimmed.starts_with("### ") {
                let heading = &trimmed[4..];
                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::from(vec![
                        Span::styled("### ", Style::default().fg(Color::Green)),
                        Span::styled(heading.to_string(), Style::default().fg(Color::Green)),
                    ]),
                });
            } else if trimmed.contains("](") {
                let mut spans = Vec::new();
                let mut remaining = trimmed;
                let mut line_ref: Option<u32> = None;

                while let Some(start_bracket) = remaining.find('[') {
                    if let Some(end_bracket) = remaining[start_bracket..].find(']') {
                        let actual_end_bracket = start_bracket + end_bracket;
                        if remaining[actual_end_bracket..].starts_with("](") {
                            if let Some(end_paren) = remaining[actual_end_bracket + 2..].find(')') {
                                let actual_end_paren = actual_end_bracket + 2 + end_paren;
                                let prefix = &remaining[..start_bracket];
                                let label = &remaining[start_bracket + 1..actual_end_bracket];
                                let href = &remaining[actual_end_bracket + 2..actual_end_paren];

                                if !prefix.is_empty() {
                                    spans.push(Span::raw(prefix.to_string()));
                                }

                                let ref_id = next_ref_id;
                                next_ref_id += 1;
                                line_ref = Some(ref_id);
                                link_targets.insert(ref_id, href.to_string());

                                spans.push(Span::styled(
                                    format!("[{}]", label),
                                    Style::default().fg(Color::LightBlue).add_modifier(Modifier::UNDERLINED),
                                ));
                                spans.push(Span::styled(
                                    format!(" (#{})", ref_id),
                                    Style::default().fg(Color::DarkGray),
                                ));

                                remaining = &remaining[actual_end_paren + 1..];
                                continue;
                            }
                        }
                    }
                    break;
                }

                if !remaining.is_empty() {
                    spans.push(Span::raw(remaining.to_string()));
                }

                render_nodes.push(RenderNode {
                    ref_id: line_ref,
                    height: 1,
                    hint_label: None,
                    content: Line::from(spans),
                });
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::from(vec![
                        Span::styled(" • ", Style::default().fg(Color::LightMagenta)),
                        Span::raw(trimmed[2..].to_string()),
                    ]),
                });
            } else {
                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::raw(trimmed.to_string()),
                });
            }
        }

        if render_nodes.is_empty() {
            render_nodes.push(RenderNode {
                ref_id: None,
                height: 1,
                hint_label: None,
                content: Line::raw("(Empty document)"),
            });
        }

        (render_nodes, extracted_title)
    }

    pub fn load_markdown_content(&mut self, url: &str, content: &str) {
        let tab = self.current_tab_mut();
        tab.url = url.to_string();
        tab.status_code = 200;
        tab.raw_content = content.to_string();
        tab.is_reader_mode = false;

        let (nodes, title) = Self::parse_markdown(content, &mut tab.link_targets);
        tab.title = title.unwrap_or_else(|| Self::url_domain(url));
        tab.viewport.set_nodes(nodes);
        self.status_message = format!("Loaded: {} ({} lines)", tab.title, tab.viewport.nodes.len());
        self.sync_mirrors();
    }

    pub async fn load_url(&mut self, target_url: &str) -> anyhow::Result<()> {
        let current_tab_url = self.current_tab().url.clone();
        if !current_tab_url.is_empty() && current_tab_url != "about:home" && current_tab_url != target_url {
            let tab = self.current_tab_mut();
            tab.history_back.push(current_tab_url);
            tab.history_forward.clear();
        }

        self.status_message = format!("Connecting to {}...", target_url);
        let client = FastPathClient::new(None);
        match client.fetch_page_content(target_url).await {
            Ok((content, status)) => {
                let tab = self.current_tab_mut();
                tab.status_code = status;
                self.load_markdown_content(target_url, &content);
            }
            Err(e) => {
                let tab = self.current_tab_mut();
                tab.status_code = 500;
                let err_md = format!("# Connection Error\n\nFailed to load `{}`:\n\n```\n{}\n```\n\n- Press `b` to go back\n- Press `r` to retry\n- Press `/` to enter another URL", target_url, e);
                self.load_markdown_content(target_url, &err_md);
                self.status_message = format!("Fetch error: {}", e);
            }
        }
        self.sync_mirrors();
        Ok(())
    }

    pub async fn search_and_display(&mut self, query: &str) -> anyhow::Result<()> {
        self.status_message = format!("Searching web for \"{}\"...", query);
        let client = FastPathClient::new(None);
        match client.search_web_results(query).await {
            Ok(results) => {
                let mut md = format!("# Web Search: \"{}\"\n\nFound {} organic results:\n\n", query, results.len());
                for r in &results {
                    md.push_str(&format!("### [{}]({})\n- **URL**: `{}`\n\n", r.title, r.url, r.url));
                }
                md.push_str("\n---\n*Press `f` to jump to any result with Vimium hints*");
                let search_url = format!("about:search?q={}", query.replace(' ', "+"));
                self.load_markdown_content(&search_url, &md);
                self.current_tab_mut().title = format!("Search: {}", query);
                self.status_message = format!("Found {} search results for \"{}\"", results.len(), query);
            }
            Err(e) => {
                self.status_message = format!("Search failed: {}", e);
                let err_md = format!("# Search Failed\n\nCould not perform search for `{}`:\n{}", query, e);
                self.load_markdown_content("about:search-error", &err_md);
            }
        }
        self.sync_mirrors();
        Ok(())
    }

    pub async fn go_back(&mut self) -> anyhow::Result<()> {
        let prev_url = self.current_tab_mut().history_back.pop();
        if let Some(target) = prev_url {
            let current = self.current_tab().url.clone();
            self.current_tab_mut().history_forward.push(current);
            self.status_message = format!("Navigating Back to {}", target);
            let client = FastPathClient::new(None);
            if let Ok((content, status)) = client.fetch_page_content(&target).await {
                let tab = self.current_tab_mut();
                tab.status_code = status;
                self.load_markdown_content(&target, &content);
            }
        } else {
            self.status_message = "History back stack is empty".to_string();
        }
        self.sync_mirrors();
        Ok(())
    }

    pub async fn go_forward(&mut self) -> anyhow::Result<()> {
        let next_url = self.current_tab_mut().history_forward.pop();
        if let Some(target) = next_url {
            let current = self.current_tab().url.clone();
            self.current_tab_mut().history_back.push(current);
            self.status_message = format!("Navigating Forward to {}", target);
            let client = FastPathClient::new(None);
            if let Ok((content, status)) = client.fetch_page_content(&target).await {
                let tab = self.current_tab_mut();
                tab.status_code = status;
                self.load_markdown_content(&target, &content);
            }
        } else {
            self.status_message = "History forward stack is empty".to_string();
        }
        self.sync_mirrors();
        Ok(())
    }

    pub fn new_tab(&mut self) {
        let mut tab = BrowserTab::default();
        let home_md = "# New Tab\n\nType `/` or `o` to enter a URL or search query.";
        let (nodes, _) = Self::parse_markdown(home_md, &mut tab.link_targets);
        tab.viewport.set_nodes(nodes);
        tab.raw_content = home_md.to_string();
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
        self.status_message = format!("Opened Tab {}", self.active_tab + 1);
        self.sync_mirrors();
    }

    pub fn close_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.tabs.remove(self.active_tab);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
            self.status_message = format!("Closed Tab. Active Tab: {}", self.active_tab + 1);
        } else {
            let mut tab = BrowserTab::default();
            let home_md = "# Terminal Web Browser (tb)\n\nTab reset to Home.";
            let (nodes, _) = Self::parse_markdown(home_md, &mut tab.link_targets);
            tab.viewport.set_nodes(nodes);
            tab.raw_content = home_md.to_string();
            self.tabs[0] = tab;
            self.status_message = "Reset tab to Home".to_string();
        }
        self.sync_mirrors();
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
            self.status_message = format!("Tab {}/{}: {}", self.active_tab + 1, self.tabs.len(), self.current_tab().title);
            self.sync_mirrors();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = if self.active_tab == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab - 1
            };
            self.status_message = format!("Tab {}/{}: {}", self.active_tab + 1, self.tabs.len(), self.current_tab().title);
            self.sync_mirrors();
        }
    }

    pub fn add_bookmark(&mut self) {
        let tab = self.current_tab();
        let title = tab.title.clone();
        let url = tab.url.clone();
        if !url.is_empty() && url != "about:home" {
            if let Ok(db) = term_storage::DatabaseManager::open_default() {
                let _ = term_storage::BookmarkVault::add(db.conn(), &url, &title);
            }
            if !self.bookmarks.iter().any(|(_, u)| u == &url) {
                self.bookmarks.push((title.clone(), url));
            }
            self.status_message = format!("Bookmarked in SQLite: \"{}\"", title);
        } else {
            self.status_message = "Invalid URL for bookmarking".to_string();
        }
    }

    pub fn show_bookmarks(&mut self) {
        if let Ok(db) = term_storage::DatabaseManager::open_default() {
            if let Ok(bms) = term_storage::BookmarkVault::list(db.conn()) {
                if !bms.is_empty() {
                    self.bookmarks = bms;
                }
            }
        }
        let mut md = "# Saved Bookmarks (SQLite)\n\n".to_string();
        if self.bookmarks.is_empty() {
            md.push_str("*(No bookmarks saved yet. Press `m` on any page to bookmark)*\n");
        } else {
            for (title, url) in &self.bookmarks {
                md.push_str(&format!("- [{}]({})\n", title, url));
            }
        }
        md.push_str("\n---\n*Press `f` to jump to any bookmark*");
        self.load_markdown_content("about:bookmarks", &md);
        self.current_tab_mut().title = "Bookmarks".to_string();
        self.status_message = format!("Displaying {} bookmarks", self.bookmarks.len());
        self.sync_mirrors();
    }

    pub fn show_history(&mut self) {
        let mut md = "# Browsing History (SQLite)\n\n".to_string();
        if let Ok(db) = term_storage::DatabaseManager::open_default() {
            if let Ok(records) = term_storage::HistoryVault::query_recent(db.conn(), 50) {
                if records.is_empty() {
                    md.push_str("*(No browsing history recorded yet)*\n");
                } else {
                    for r in records {
                        let title = r.title.unwrap_or_else(|| r.url.clone());
                        md.push_str(&format!("- [{}]({}) *(Visited {}x)*\n", title, r.url, r.visit_count));
                    }
                }
            }
        }
        md.push_str("\n---\n*Press `f` to jump to any history entry*");
        self.load_markdown_content("about:history", &md);
        self.current_tab_mut().title = "History".to_string();
        self.status_message = "Browsing history loaded from SQLite".to_string();
        self.sync_mirrors();
    }

    pub fn toggle_reader_mode(&mut self) {
        let tab = self.current_tab_mut();
        if tab.is_reader_mode {
            let (nodes, _) = Self::parse_markdown(&tab.raw_content, &mut tab.link_targets);
            tab.viewport.set_nodes(nodes);
            tab.is_reader_mode = false;
            self.status_message = "Reader mode disabled (Standard View)".to_string();
        } else {
            let mut prose_lines = Vec::new();
            for line in tab.raw_content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') || trimmed.len() > 60 || (!trimmed.starts_with('[') && trimmed.contains(' ')) {
                    prose_lines.push(trimmed);
                }
            }
            let reader_md = format!("# [Reader Mode] {}\n\n{}", tab.title, prose_lines.join("\n\n"));
            let (nodes, _) = Self::parse_markdown(&reader_md, &mut tab.link_targets);
            tab.viewport.set_nodes(nodes);
            tab.is_reader_mode = true;
            self.status_message = "Reader mode enabled (Distraction-free)".to_string();
        }
        self.sync_mirrors();
    }

    pub fn perform_in_page_search(&mut self) {
        if self.search_query.trim().is_empty() {
            self.status_message = "Search query is empty".to_string();
            return;
        }

        let query = self.search_query.to_lowercase();
        let tab = self.current_tab();
        let mut matches = Vec::new();

        for (idx, node) in tab.viewport.nodes.iter().enumerate() {
            let line_text: String = node.content.spans.iter().map(|s| s.content.as_ref()).collect();
            if line_text.to_lowercase().contains(&query) {
                matches.push(idx);
            }
        }

        if !matches.is_empty() {
            self.search_matches = matches;
            self.current_match_idx = 0;
            let target_line = self.search_matches[0];
            self.current_tab_mut().viewport.scroll_y = target_line.saturating_sub(2);
            self.status_message = format!("Found {} matches for \"{}\" (1/{})", self.search_matches.len(), self.search_query, self.search_matches.len());
        } else {
            self.search_matches.clear();
            self.status_message = format!("No matches found for \"{}\"", self.search_query);
        }
        self.sync_mirrors();
    }

    pub fn next_search_match(&mut self) {
        if !self.search_matches.is_empty() {
            self.current_match_idx = (self.current_match_idx + 1) % self.search_matches.len();
            let target_line = self.search_matches[self.current_match_idx];
            self.current_tab_mut().viewport.scroll_y = target_line.saturating_sub(2);
            self.status_message = format!("Match {}/{} for \"{}\"", self.current_match_idx + 1, self.search_matches.len(), self.search_query);
            self.sync_mirrors();
        }
    }

    pub fn prev_search_match(&mut self) {
        if !self.search_matches.is_empty() {
            self.current_match_idx = if self.current_match_idx == 0 {
                self.search_matches.len() - 1
            } else {
                self.current_match_idx - 1
            };
            let target_line = self.search_matches[self.current_match_idx];
            self.current_tab_mut().viewport.scroll_y = target_line.saturating_sub(2);
            self.status_message = format!("Match {}/{} for \"{}\"", self.current_match_idx + 1, self.search_matches.len(), self.search_query);
            self.sync_mirrors();
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let bottom_height = match self.input_mode {
            InputMode::UrlBar | InputMode::InPageSearch => 2,
            _ => 1,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header & Tab strip
                Constraint::Min(5),    // Main Viewport
                Constraint::Length(bottom_height), // Status & Input Bar
            ])
            .split(frame.area());

        // Header: Tab Strip & URL
        let mut tab_spans = Vec::new();
        for (i, tab) in self.tabs.iter().enumerate() {
            let title_display = if tab.title.len() > 18 {
                format!("{}...", &tab.title[..16])
            } else {
                tab.title.clone()
            };
            if i == self.active_tab {
                tab_spans.push(Span::styled(
                    format!(" [Tab {}: {}*] ", i + 1, title_display),
                    Style::default().fg(Color::Yellow).bg(Color::Blue).add_modifier(Modifier::BOLD),
                ));
            } else {
                tab_spans.push(Span::styled(
                    format!(" Tab {}: {} ", i + 1, title_display),
                    Style::default().fg(Color::Gray),
                ));
            }
        }
        let tab_line = Line::from(tab_spans);

        let active_tab = self.current_tab();
        let scroll_pct = if active_tab.viewport.nodes.is_empty() {
            0
        } else {
            (active_tab.viewport.scroll_y * 100) / active_tab.viewport.nodes.len().max(1)
        };

        let info_line = Line::from(vec![
            Span::styled(format!(" URL: {} ", active_tab.url), Style::default().fg(Color::White)),
            Span::styled(format!("| Scroll: {}% ", scroll_pct), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("| HTTP {} ", active_tab.status_code), Style::default().fg(if active_tab.status_code == 200 { Color::Green } else { Color::Red })),
            Span::styled(format!("| Mem: {}MB", self.memory_usage_mb), Style::default().fg(Color::DarkGray)),
        ]);

        let header = Paragraph::new(vec![tab_line, info_line])
            .block(Block::default().borders(Borders::ALL).title("Terminal Browser v0.1.0"));
        frame.render_widget(header, chunks[0]);

        // Main Viewport
        let viewport_title = format!("Page Viewport ({})", active_tab.title);
        let viewport_block = Block::default().borders(Borders::ALL).title(viewport_title);
        let inner_area = viewport_block.inner(chunks[1]);
        frame.render_widget(viewport_block, chunks[1]);
        active_tab.viewport.render_visible_slice(inner_area, frame.buffer_mut());

        // Bottom Bar / Input Bar
        match self.input_mode {
            InputMode::UrlBar => {
                let p = Paragraph::new(format!("🌐 Open URL or Search: {}█", self.input_buffer))
                    .style(Style::default().fg(Color::Yellow).bg(Color::Rgb(20, 20, 50)).add_modifier(Modifier::BOLD));
                frame.render_widget(p, chunks[2]);
            }
            InputMode::InPageSearch => {
                let p = Paragraph::new(format!("🔍 Find in page: {}█", self.input_buffer))
                    .style(Style::default().fg(Color::Green).bg(Color::Rgb(10, 40, 20)).add_modifier(Modifier::BOLD));
                frame.render_widget(p, chunks[2]);
            }
            InputMode::VimiumHints => {
                let p = Paragraph::new(format!("⚡ Vimium Hints: Type 2 letters to jump [Esc to cancel] Typed: \"{}\"", self.typed_hint))
                    .style(Style::default().fg(Color::Yellow).bg(Color::Black).add_modifier(Modifier::BOLD));
                frame.render_widget(p, chunks[2]);
            }
            InputMode::HelpModal => {
                let p = Paragraph::new("❓ Cheatsheet: [/] Go/Search [f] Hints [b/B] Back/Fwd [t/w] New/Close Tab [Tab] Next Tab [?] Find [n/N] Next/Prev [R] Reader [m/M] Bookmarks [q] Quit")
                    .style(Style::default().fg(Color::Cyan).bg(Color::DarkGray).add_modifier(Modifier::BOLD));
                frame.render_widget(p, chunks[2]);
            }
            InputMode::Normal => {
                let status = Paragraph::new(format!(
                    " [/] Go/Search | [f] Links | [b] Back | [t] Tab | [?] Find | [R] Reader | [h] Help | {}",
                    self.status_message
                ))
                .style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
                frame.render_widget(status, chunks[2]);
            }
        }
    }

    pub async fn run_interactive(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide)?;
        let _guard = TerminalGuard;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        self.sync_mirrors();

        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match self.input_mode {
                            InputMode::UrlBar => {
                                match key.code {
                                    KeyCode::Esc => {
                                        self.input_mode = InputMode::Normal;
                                        self.input_buffer.clear();
                                        self.status_message = "Navigation cancelled".to_string();
                                    }
                                    KeyCode::Enter => {
                                        let raw = self.input_buffer.trim().to_string();
                                        self.input_mode = InputMode::Normal;
                                        self.input_buffer.clear();

                                        if !raw.is_empty() {
                                            if raw.starts_with("http://") || raw.starts_with("https://") || raw.starts_with("about:") {
                                                let _ = self.load_url(&raw).await;
                                            } else if !raw.contains(' ') && raw.contains('.') {
                                                let url = format!("https://{}", raw);
                                                let _ = self.load_url(&url).await;
                                            } else {
                                                let _ = self.search_and_display(&raw).await;
                                            }
                                        }
                                    }
                                    KeyCode::Backspace => {
                                        self.input_buffer.pop();
                                    }
                                    KeyCode::Char(c) => {
                                        self.input_buffer.push(c);
                                    }
                                    _ => {}
                                }
                            }
                            InputMode::InPageSearch => {
                                match key.code {
                                    KeyCode::Esc => {
                                        self.input_mode = InputMode::Normal;
                                        self.input_buffer.clear();
                                        self.status_message = "Search cancelled".to_string();
                                    }
                                    KeyCode::Enter => {
                                        self.search_query = self.input_buffer.trim().to_string();
                                        self.input_mode = InputMode::Normal;
                                        self.input_buffer.clear();
                                        self.perform_in_page_search();
                                    }
                                    KeyCode::Backspace => {
                                        self.input_buffer.pop();
                                    }
                                    KeyCode::Char(c) => {
                                        self.input_buffer.push(c);
                                    }
                                    _ => {}
                                }
                            }
                            InputMode::VimiumHints => {
                                match key.code {
                                    KeyCode::Esc | KeyCode::Char('q') => {
                                        self.input_mode = InputMode::Normal;
                                        self.typed_hint.clear();
                                        self.current_tab_mut().viewport.clear_hints();
                                        self.status_message = "Hints cancelled".to_string();
                                    }
                                    KeyCode::Char(c) => {
                                        self.typed_hint.push(c);
                                        if self.typed_hint.len() == 2 {
                                            let chars: Vec<char> = self.typed_hint.chars().collect();
                                            let pair = [chars[0], chars[1]];
                                            if let Some(target_ref) = crate::hints::resolve_hint(&self.current_hints, pair) {
                                                let href_opt = self.current_tab().link_targets.get(&target_ref).cloned();
                                                if let Some(href) = href_opt {
                                                    let base_url = self.current_tab().url.clone();
                                                    let target = if href.starts_with("http") {
                                                        href
                                                    } else if href.starts_with('/') {
                                                        if let Ok(base) = url::Url::parse(&base_url) {
                                                            format!("{}://{}{}", base.scheme(), base.host_str().unwrap_or(""), href)
                                                        } else {
                                                            href
                                                        }
                                                    } else {
                                                        href
                                                    };
                                                    self.status_message = format!("Loading: {}", target);
                                                    terminal.draw(|f| self.render(f))?;
                                                    let _ = self.load_url(&target).await;
                                                } else {
                                                    self.status_message = format!("Selected element #{}", target_ref);
                                                }
                                            } else {
                                                self.status_message = format!("No link found for [{}{}]", pair[0], pair[1]);
                                            }
                                            self.input_mode = InputMode::Normal;
                                            self.typed_hint.clear();
                                            self.current_tab_mut().viewport.clear_hints();
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            InputMode::HelpModal => {
                                if matches!(key.code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h')) {
                                    self.input_mode = InputMode::Normal;
                                    self.status_message = "Help closed".to_string();
                                }
                            }
                            InputMode::Normal => {
                                match key.code {
                                    // Quit
                                    KeyCode::Char('q') => break,
                                    KeyCode::Esc => {
                                        self.status_message = "Press 'q' to quit".to_string();
                                    }

                                    // Scrolling
                                    KeyCode::Char('j') | KeyCode::Down => self.current_tab_mut().viewport.scroll_down(1),
                                    KeyCode::Char('k') | KeyCode::Up => self.current_tab_mut().viewport.scroll_up(1),
                                    KeyCode::Char('d') | KeyCode::PageDown => self.current_tab_mut().viewport.scroll_down(15),
                                    KeyCode::Char('u') | KeyCode::PageUp => self.current_tab_mut().viewport.scroll_up(15),
                                    KeyCode::Char('g') | KeyCode::Home => self.current_tab_mut().viewport.scroll_y = 0,
                                    KeyCode::Char('G') | KeyCode::End => self.current_tab_mut().viewport.scroll_down(usize::MAX / 2),

                                    // Address Bar / URL / Search
                                    KeyCode::Char('/') | KeyCode::Char('o') => {
                                        self.input_mode = InputMode::UrlBar;
                                        self.input_buffer.clear();
                                    }

                                    // In-page search
                                    KeyCode::Char('?') => {
                                        self.input_mode = InputMode::InPageSearch;
                                        self.input_buffer.clear();
                                    }
                                    KeyCode::Char('n') => self.next_search_match(),
                                    KeyCode::Char('N') => self.prev_search_match(),

                                    // Navigation History
                                    KeyCode::Char('b') | KeyCode::Backspace | KeyCode::Char('H') => {
                                        let _ = self.go_back().await;
                                    }
                                    KeyCode::Char('B') | KeyCode::Char('L') => {
                                        let _ = self.go_forward().await;
                                    }
                                    KeyCode::Char('r') => {
                                        let url = self.current_tab().url.clone();
                                        if !url.is_empty() && url != "about:home" {
                                            let _ = self.load_url(&url).await;
                                        }
                                    }
                                    KeyCode::Char('R') => {
                                        self.toggle_reader_mode();
                                    }

                                    // Tabs
                                    KeyCode::Char('t') => self.new_tab(),
                                    KeyCode::Char('w') => self.close_tab(),
                                    KeyCode::Tab => self.next_tab(),
                                    KeyCode::BackTab => self.prev_tab(),
                                    KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                                        let idx = (c as usize) - ('1' as usize);
                                        if idx < self.tabs.len() {
                                            self.active_tab = idx;
                                            self.status_message = format!("Switched to Tab {}", idx + 1);
                                            self.sync_mirrors();
                                        }
                                    }

                                    // Bookmarks
                                    KeyCode::Char('m') => self.add_bookmark(),
                                    KeyCode::Char('M') => self.show_bookmarks(),

                                    // Help
                                    KeyCode::Char('h') | KeyCode::F(1) => {
                                        self.input_mode = InputMode::HelpModal;
                                    }

                                    // Vimium hints
                                    KeyCode::Char('f') => {
                                        let size = terminal.size()?;
                                        let rect = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                                        let visible_refs = self.current_tab().viewport.get_visible_interactive_ref_ids(rect);
                                        if visible_refs.is_empty() {
                                            self.status_message = "No interactive links visible in viewport".to_string();
                                        } else {
                                            let hints = crate::hints::generate_hints(&visible_refs);
                                            self.current_tab_mut().viewport.apply_hints(&hints);
                                            self.current_hints = hints;
                                            self.input_mode = InputMode::VimiumHints;
                                            self.typed_hint.clear();
                                            self.status_message = "Vimium Hint Mode: Type 2 letters to jump [Esc to cancel]".to_string();
                                        }
                                    }

                                    // Ctrl combos
                                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
