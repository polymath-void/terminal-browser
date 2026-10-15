use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Line,
};

pub struct RenderNode {
    pub ref_id: Option<u32>,
    pub height: u16,
    pub hint_label: Option<[char; 2]>,
    pub content: Line<'static>,
}

pub struct VirtualViewport {
    pub nodes: Vec<RenderNode>,
    pub scroll_y: usize,
}

impl VirtualViewport {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            scroll_y: 0,
        }
    }

    pub fn set_nodes(&mut self, nodes: Vec<RenderNode>) {
        self.nodes = nodes;
        self.scroll_y = 0;
    }

    pub fn scroll_down(&mut self, lines: usize) {
        if !self.nodes.is_empty() {
            self.scroll_y = (self.scroll_y + lines).min(self.nodes.len().saturating_sub(1));
        }
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_y = self.scroll_y.saturating_sub(lines);
    }

    pub fn render_visible_slice(&self, area: Rect, buf: &mut Buffer) {
        let mut row = area.top();
        for node in self.nodes.iter().skip(self.scroll_y) {
            if row >= area.bottom() {
                break;
            }
            buf.set_line(area.left(), row, &node.content, area.width);
            row += node.height.max(1);
        }
    }

    pub fn get_interactive_ref_ids(&self) -> Vec<u32> {
        self.nodes
            .iter()
            .filter_map(|n| n.ref_id)
            .collect()
    }
}
