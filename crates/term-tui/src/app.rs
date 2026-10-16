use crate::viewport::{RenderNode, VirtualViewport};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use term_core::PageSnapshot;

#[derive(Debug, Clone, PartialEq)]
pub struct TuiApp {
    pub viewport: VirtualViewport,
    pub current_url: String,
    pub status_code: u16,
    pub status_message: String,
    pub hint_mode: bool,
    pub memory_usage_mb: u64,
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
            " Status: {} | Hints: [f] | Quit: [q] | {}",
            self.status_code, self.status_message
        ))
        .style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));
        frame.render_widget(status, chunks[2]);
    }
}
