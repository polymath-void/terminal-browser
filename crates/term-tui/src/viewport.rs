use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Line,
};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct RenderNode {
    pub ref_id: Option<u32>,
    pub height: u16,
    pub hint_label: Option<[char; 2]>,
    pub content: Line<'static>,
}

#[derive(Debug, Clone, Default, PartialEq)]
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

    /// Renders only the visible vertical slice of nodes into the buffer.
    /// This guarantees O(visible_rows) time and O(1) heap allocation regardless of total document size.
    pub fn render_visible_slice(&self, area: Rect, buf: &mut Buffer) {
        let render_area = area.intersection(buf.area);
        if render_area.is_empty() {
            return;
        }

        let mut row = render_area.top();
        for node in self.nodes.iter().skip(self.scroll_y) {
            if row >= render_area.bottom() {
                break;
            }
            buf.set_line(render_area.left(), row, &node.content, render_area.width);

            // Render Vimium hint badge if present
            if let Some([c1, c2]) = node.hint_label {
                let badge = format!("[{}{}]", c1, c2);
                let badge_len = badge.len() as u16;
                let badge_x = render_area.right().saturating_sub(badge_len);
                if badge_x >= render_area.left() {
                    buf.set_string(
                        badge_x,
                        row,
                        &badge,
                        Style::default()
                            .fg(Color::Yellow)
                            .bg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    );
                }
            }

            row = row.saturating_add(node.height.max(1));
        }
    }

    /// Returns all interactive Ref IDs present in the entire document.
    pub fn get_interactive_ref_ids(&self) -> Vec<u32> {
        self.nodes
            .iter()
            .filter_map(|n| n.ref_id)
            .collect()
    }

    /// Returns only the interactive Ref IDs currently visible in the provided viewport area.
    pub fn get_visible_interactive_ref_ids(&self, area: Rect) -> Vec<u32> {
        if area.width == 0 || area.height == 0 {
            return Vec::new();
        }

        let mut row = area.top();
        let mut visible_refs = Vec::new();
        for node in self.nodes.iter().skip(self.scroll_y) {
            if row >= area.bottom() {
                break;
            }
            if let Some(ref_id) = node.ref_id {
                visible_refs.push(ref_id);
            }
            row = row.saturating_add(node.height.max(1));
        }
        visible_refs
    }

    /// Attaches Vimium hint labels to matching nodes by Ref ID.
    pub fn apply_hints(&mut self, hints: &HashMap<u32, [char; 2]>) {
        for node in &mut self.nodes {
            if let Some(ref_id) = node.ref_id {
                node.hint_label = hints.get(&ref_id).copied();
            } else {
                node.hint_label = None;
            }
        }
    }

    /// Clears all hint labels across all nodes.
    pub fn clear_hints(&mut self) {
        for node in &mut self.nodes {
            node.hint_label = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::text::Line;

    #[test]
    fn test_empty_viewport() {
        let mut vp = VirtualViewport::new();
        assert_eq!(vp.nodes.len(), 0);
        assert_eq!(vp.scroll_y, 0);

        vp.scroll_down(10);
        assert_eq!(vp.scroll_y, 0);
        vp.scroll_up(5);
        assert_eq!(vp.scroll_y, 0);

        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        vp.render_visible_slice(area, &mut buf);
        assert_eq!(vp.get_interactive_ref_ids().len(), 0);
    }

    #[test]
    fn test_scroll_clamping() {
        let mut vp = VirtualViewport::new();
        let nodes = (0..50)
            .map(|i| RenderNode {
                ref_id: Some(i),
                height: 1,
                hint_label: None,
                content: Line::from(format!("Line {}", i)),
            })
            .collect();
        vp.set_nodes(nodes);

        assert_eq!(vp.scroll_y, 0);

        vp.scroll_down(10);
        assert_eq!(vp.scroll_y, 10);

        vp.scroll_down(100);
        // Should clamp to len - 1 = 49
        assert_eq!(vp.scroll_y, 49);

        vp.scroll_up(20);
        assert_eq!(vp.scroll_y, 29);

        vp.scroll_up(100);
        // Saturates at 0
        assert_eq!(vp.scroll_y, 0);
    }

    #[test]
    fn test_render_visible_slice_exact_content() {
        let mut vp = VirtualViewport::new();
        let nodes = (0..100)
            .map(|i| RenderNode {
                ref_id: Some(i),
                height: 1,
                hint_label: None,
                content: Line::from(format!("Item-{:03}", i)),
            })
            .collect();
        vp.set_nodes(nodes);

        let area = Rect::new(0, 0, 20, 5);
        let mut buf = Buffer::empty(area);

        // At scroll_y = 0, lines 0..5 should be rendered
        vp.render_visible_slice(area, &mut buf);
        assert_eq!(buf[(0, 0)].symbol(), "I");
        assert_eq!(buf[(5, 0)].symbol(), "0");
        assert_eq!(buf[(6, 0)].symbol(), "0");
        assert_eq!(buf[(7, 0)].symbol(), "0");

        assert_eq!(buf[(5, 4)].symbol(), "0");
        assert_eq!(buf[(6, 4)].symbol(), "0");
        assert_eq!(buf[(7, 4)].symbol(), "4");

        // Scroll down to 10
        vp.scroll_y = 10;
        let mut buf2 = Buffer::empty(area);
        vp.render_visible_slice(area, &mut buf2);
        assert_eq!(buf2[(5, 0)].symbol(), "0");
        assert_eq!(buf2[(6, 0)].symbol(), "1");
        assert_eq!(buf2[(7, 0)].symbol(), "0");
    }

    #[test]
    fn test_zero_dimensions_do_not_panic() {
        let mut vp = VirtualViewport::new();
        vp.set_nodes(vec![RenderNode {
            ref_id: Some(1),
            height: 1,
            hint_label: None,
            content: Line::from("test"),
        }]);

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));
        vp.render_visible_slice(Rect::new(0, 0, 0, 0), &mut buf);
        vp.render_visible_slice(Rect::new(0, 0, 80, 0), &mut buf);
        vp.render_visible_slice(Rect::new(0, 0, 0, 24), &mut buf);

        let visible = vp.get_visible_interactive_ref_ids(Rect::new(0, 0, 0, 0));
        assert!(visible.is_empty());
    }

    #[test]
    fn test_virtual_slicing_1000_plus_lines() {
        let mut vp = VirtualViewport::new();
        let line_count = 2500;
        let nodes = (0..line_count)
            .map(|i| RenderNode {
                ref_id: if i % 2 == 0 { Some(i as u32) } else { None },
                height: 1,
                hint_label: None,
                content: Line::from(format!("Document row {}", i)),
            })
            .collect();
        vp.set_nodes(nodes);

        let area = Rect::new(0, 0, 80, 25);
        let mut buf = Buffer::empty(area);

        // Render at top
        vp.render_visible_slice(area, &mut buf);
        let visible_refs = vp.get_visible_interactive_ref_ids(area);
        assert_eq!(visible_refs.len(), 13); // even indices in 0..25

        // Scroll halfway into a 2,500 line document
        vp.scroll_down(1200);
        assert_eq!(vp.scroll_y, 1200);
        let mut buf_scrolled = Buffer::empty(area);
        vp.render_visible_slice(area, &mut buf_scrolled);

        let visible_refs_scrolled = vp.get_visible_interactive_ref_ids(area);
        assert!(!visible_refs_scrolled.is_empty());
        assert_eq!(visible_refs_scrolled[0], 1200);

        // Scroll to the very end
        vp.scroll_down(5000);
        assert_eq!(vp.scroll_y, 2499);
        let mut buf_end = Buffer::empty(area);
        vp.render_visible_slice(area, &mut buf_end);
    }

    #[test]
    fn test_hint_application_and_badge_rendering() {
        let mut vp = VirtualViewport::new();
        let nodes = vec![
            RenderNode {
                ref_id: Some(42),
                height: 1,
                hint_label: None,
                content: Line::from("Clickable link"),
            },
            RenderNode {
                ref_id: None,
                height: 1,
                hint_label: None,
                content: Line::from("Plain text paragraph"),
            },
        ];
        vp.set_nodes(nodes);

        let mut hints = HashMap::new();
        hints.insert(42, ['a', 'k']);
        vp.apply_hints(&hints);

        assert_eq!(vp.nodes[0].hint_label, Some(['a', 'k']));
        assert_eq!(vp.nodes[1].hint_label, None);

        let area = Rect::new(0, 0, 30, 2);
        let mut buf = Buffer::empty(area);
        vp.render_visible_slice(area, &mut buf);

        // Badge [ak] is 4 characters wide at area.right() - 4 = 30 - 4 = 26
        assert_eq!(buf[(26, 0)].symbol(), "[");
        assert_eq!(buf[(27, 0)].symbol(), "a");
        assert_eq!(buf[(28, 0)].symbol(), "k");
        assert_eq!(buf[(29, 0)].symbol(), "]");

        // Plain text node does not have badge
        assert_ne!(buf[(26, 1)].symbol(), "[");

        // Clear hints
        vp.clear_hints();
        assert_eq!(vp.nodes[0].hint_label, None);
    }
}
