use ratatui::{buffer::Buffer, layout::Rect, text::Line};
use term_tui::{RenderNode, VirtualViewport};

#[test]
fn test_slicing_1000_line_document() {
    let mut vp = VirtualViewport::new();
    let count = 1000;
    let nodes: Vec<RenderNode> = (0..count)
        .map(|i| RenderNode {
            ref_id: if i % 3 == 0 { Some(i as u32) } else { None },
            height: 1,
            hint_label: None,
            content: Line::from(format!("Line #{:04}: Semantic content for test", i)),
        })
        .collect();
    vp.set_nodes(nodes);

    let area = Rect::new(0, 0, 100, 30);
    let mut buf = Buffer::empty(area);

    // 1. Initial render at top (scroll_y = 0)
    vp.render_visible_slice(area, &mut buf);
    let visible_refs = vp.get_visible_interactive_ref_ids(area);
    assert_eq!(visible_refs, vec![0, 3, 6, 9, 12, 15, 18, 21, 24, 27]);

    // 2. Continuous rapid scrolling down through the entire 1,000 lines
    for step in (0..=1000).step_by(25) {
        vp.scroll_y = step;
        let mut step_buf = Buffer::empty(area);
        // Slicing must not panic at any step
        vp.render_visible_slice(area, &mut step_buf);
        let refs = vp.get_visible_interactive_ref_ids(area);
        // Each visible window has at most 30 lines
        assert!(refs.len() <= 30);
    }

    // 3. Scroll beyond document end (clamping)
    vp.scroll_down(5000);
    assert_eq!(vp.scroll_y, 999);
    let mut end_buf = Buffer::empty(area);
    vp.render_visible_slice(area, &mut end_buf);

    // 4. Scroll back to top
    vp.scroll_up(10000);
    assert_eq!(vp.scroll_y, 0);
}

#[test]
fn test_slicing_10000_line_stress_no_panic() {
    let mut vp = VirtualViewport::new();
    let count = 10000;
    let nodes: Vec<RenderNode> = (0..count)
        .map(|i| RenderNode {
            ref_id: Some(i as u32),
            height: 1,
            hint_label: None,
            content: Line::from(format!("Stress item {}", i)),
        })
        .collect();
    vp.set_nodes(nodes);

    let area = Rect::new(0, 0, 120, 40);
    let mut buf = Buffer::empty(area);

    // Render slices from various offsets
    let offsets = [0, 50, 500, 1500, 5000, 9950, 9999];
    for &offset in &offsets {
        vp.scroll_y = offset;
        vp.render_visible_slice(area, &mut buf);
        let visible = vp.get_visible_interactive_ref_ids(area);
        let expected_count = (count - offset).min(area.height as usize);
        assert_eq!(visible.len(), expected_count);
    }
}

#[test]
fn test_variable_node_heights() {
    let mut vp = VirtualViewport::new();
    // Create nodes with heights: 1, 2, 3, 4, 5
    let nodes = vec![
        RenderNode {
            ref_id: Some(1),
            height: 2,
            hint_label: None,
            content: Line::from("Node 1 (height 2)"),
        },
        RenderNode {
            ref_id: Some(2),
            height: 3,
            hint_label: None,
            content: Line::from("Node 2 (height 3)"),
        },
        RenderNode {
            ref_id: Some(3),
            height: 1,
            hint_label: None,
            content: Line::from("Node 3 (height 1)"),
        },
        RenderNode {
            ref_id: Some(4),
            height: 4,
            hint_label: None,
            content: Line::from("Node 4 (height 4)"),
        },
    ];
    vp.set_nodes(nodes);

    // Viewport height 5: Node 1 (rows 0, 1), Node 2 (rows 2, 3, 4). Total rows = 5.
    // Node 3 starts at row 5 which is >= area.bottom(), so only nodes 1 and 2 are visible.
    let area = Rect::new(0, 0, 80, 5);
    let visible = vp.get_visible_interactive_ref_ids(area);
    assert_eq!(visible, vec![1, 2]);

    // Viewport height 6: now Node 3 is also visible
    let area6 = Rect::new(0, 0, 80, 6);
    let visible6 = vp.get_visible_interactive_ref_ids(area6);
    assert_eq!(visible6, vec![1, 2, 3]);
}

#[test]
fn test_extreme_boundary_rects() {
    let mut vp = VirtualViewport::new();
    vp.set_nodes(vec![RenderNode {
        ref_id: Some(1),
        height: 10,
        hint_label: None,
        content: Line::from("Boundary test"),
    }]);

    // Rect near u16::MAX
    let boundary_area = Rect::new(0, 65530, 80, 5);
    let mut buf = Buffer::empty(Rect::new(0, 0, 100, 100));
    // Must not panic with integer overflow
    vp.render_visible_slice(boundary_area, &mut buf);
}
