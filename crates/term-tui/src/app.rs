use crate::viewport::{RenderNode, VirtualViewport};
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEventKind},
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

#[derive(Debug, Clone, PartialEq)]
pub struct TuiApp {
    pub viewport: VirtualViewport,
    pub current_url: String,
    pub status_code: u16,
    pub status_message: String,
    pub hint_mode: bool,
    pub memory_usage_mb: u64,
    pub current_hints: HashMap<u32, [char; 2]>,
    pub link_targets: HashMap<u32, String>,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiApp {
    pub fn new() -> Self {
        Self {
            viewport: VirtualViewport::new(),
            current_url: "about:blank".to_string(),
            status_code: 200,
            status_message: "Ready".to_string(),
            hint_mode: false,
            memory_usage_mb: 18,
            current_hints: HashMap::new(),
            link_targets: HashMap::new(),
        }
    }

    pub fn load_snapshot(&mut self, snapshot: PageSnapshot) {
        self.current_url = snapshot.url;
        self.status_code = snapshot.http_status;
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
        self.viewport.set_nodes(render_nodes);
    }

    pub fn load_markdown_content(&mut self, url: &str, content: &str) {
        self.current_url = url.to_string();
        self.status_code = 200;
        self.status_message = "Page Loaded".to_string();
        let mut render_nodes = Vec::new();
        self.link_targets.clear();
        let mut next_ref_id: u32 = 1;

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
                // Parse simple markdown link [label](url)
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
                                self.link_targets.insert(ref_id, href.to_string());

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

        self.viewport.set_nodes(render_nodes);
    }

    pub async fn load_url(&mut self, target_url: &str) -> anyhow::Result<()> {
        self.status_message = format!("Connecting to {}...", target_url);
        let client = FastPathClient::new(None);
        match client.fetch_page_content(target_url).await {
            Ok((content, status)) => {
                self.status_code = status;
                self.load_markdown_content(target_url, &content);
                self.status_message = format!("Ready ({} lines)", self.viewport.nodes.len());
            }
            Err(e) => {
                self.status_code = 500;
                self.status_message = format!("Fetch error: {}", e);
                let err_md = format!("# Connection Error\n\nFailed to load `{}`:\n{}", target_url, e);
                self.load_markdown_content(target_url, &err_md);
            }
        }
        Ok(())
    }

    pub fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header / Tab Bar
                Constraint::Min(5),    // Main Viewport
                Constraint::Length(1), // Status Bar
            ])
            .split(frame.area());

        // Header
        let header = Paragraph::new(format!(" URL: {} | Mem: {}MB", self.current_url, self.memory_usage_mb))
            .block(Block::default().borders(Borders::ALL).title("Terminal Browser v0.1.0"));
        frame.render_widget(header, chunks[0]);

        // Main Viewport
        let viewport_block = Block::default().borders(Borders::ALL).title("Page Viewport");
        let inner_area = viewport_block.inner(chunks[1]);
        frame.render_widget(viewport_block, chunks[1]);
        self.viewport.render_visible_slice(inner_area, frame.buffer_mut());

        // Status Bar
        let status = Paragraph::new(format!(
            " Status: {} | Hints: [f] | Reload: [r] | Quit: [q] | {}",
            self.status_code, self.status_message
        ))
        .style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
        frame.render_widget(status, chunks[2]);
    }

    pub async fn run_interactive(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide)?;
        let _guard = TerminalGuard;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut typed_hint = String::new();

        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if self.hint_mode {
                            match key.code {
                                KeyCode::Esc | KeyCode::Char('q') => {
                                    self.hint_mode = false;
                                    typed_hint.clear();
                                    self.viewport.clear_hints();
                                    self.status_message = "Hints cancelled".to_string();
                                }
                                KeyCode::Char(c) => {
                                    typed_hint.push(c);
                                    if typed_hint.len() == 2 {
                                        let chars: Vec<char> = typed_hint.chars().collect();
                                        let pair = [chars[0], chars[1]];
                                        if let Some(target_ref) = crate::hints::resolve_hint(&self.current_hints, pair) {
                                            if let Some(href) = self.link_targets.get(&target_ref).cloned() {
                                                self.status_message = format!("Loading: {}", href);
                                                terminal.draw(|f| self.render(f))?;
                                                let target = if href.starts_with("http") {
                                                    href
                                                } else if href.starts_with('/') {
                                                    if let Ok(base) = url::Url::parse(&self.current_url) {
                                                        format!("{}://{}{}", base.scheme(), base.host_str().unwrap_or(""), href)
                                                    } else {
                                                        href
                                                    }
                                                } else {
                                                    href
                                                };
                                                let _ = self.load_url(&target).await;
                                            } else {
                                                self.status_message = format!("Selected element #{}", target_ref);
                                            }
                                        } else {
                                            self.status_message = format!("No link found for [{}{}]", pair[0], pair[1]);
                                        }
                                        self.hint_mode = false;
                                        typed_hint.clear();
                                        self.viewport.clear_hints();
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            match key.code {
                                KeyCode::Char('q') | KeyCode::Esc => break,
                                KeyCode::Char('j') | KeyCode::Down => self.viewport.scroll_down(1),
                                KeyCode::Char('k') | KeyCode::Up => self.viewport.scroll_up(1),
                                KeyCode::Char('d') | KeyCode::PageDown => self.viewport.scroll_down(15),
                                KeyCode::Char('u') | KeyCode::PageUp => self.viewport.scroll_up(15),
                                KeyCode::Char('g') | KeyCode::Home => self.viewport.scroll_y = 0,
                                KeyCode::Char('G') | KeyCode::End => self.viewport.scroll_down(usize::MAX / 2),
                                KeyCode::Char('r') => {
                                    let url = self.current_url.clone();
                                    let _ = self.load_url(&url).await;
                                }
                                KeyCode::Char('f') => {
                                    let size = terminal.size()?;
                                    let rect = ratatui::layout::Rect::new(0, 0, size.width, size.height);
                                    let visible_refs = self.viewport.get_visible_interactive_ref_ids(rect);
                                    if visible_refs.is_empty() {
                                        self.status_message = "No interactive links visible in viewport".to_string();
                                    } else {
                                        self.current_hints = crate::hints::generate_hints(&visible_refs);
                                        self.viewport.apply_hints(&self.current_hints);
                                        self.hint_mode = true;
                                        typed_hint.clear();
                                        self.status_message = "Vimium Hint Mode: Type 2 letters to jump [Esc to cancel]".to_string();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
