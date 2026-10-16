use ratatui::{buffer::Buffer, layout::Rect, text::Line};
use term_tui::{filter_hints, generate_hints, resolve_hint, RenderNode, VirtualViewport};

#[test]
fn test_vimium_full_lifecycle() {
    let mut vp = VirtualViewport::new();
    let nodes: Vec<RenderNode> = (0..20)
        .map(|i| RenderNode {
            ref_id: Some(100 + i as u32),
            height: 1,
            hint_label: None,
            content: Line::from(format!("Interactive element #{}", 100 + i)),
        })
        .collect();
    vp.set_nodes(nodes);

    let area = Rect::new(0, 0, 80, 10);

    // 1. Get visible interactive Ref IDs in the viewport
    let visible_refs = vp.get_visible_interactive_ref_ids(area);
    assert_eq!(visible_refs.len(), 10);
    assert_eq!(visible_refs[0], 100);
    assert_eq!(visible_refs[9], 109);

    // 2. Generate Vimium two-letter hint labels
    let hints = generate_hints(&visible_refs);
    assert_eq!(hints.len(), 10);

    // Check specific mnemonic codes
    assert_eq!(hints.get(&100), Some(&['a', 'a']));
    assert_eq!(hints.get(&101), Some(&['a', 's']));
    assert_eq!(hints.get(&102), Some(&['a', 'd']));

    // 3. Apply hints to viewport nodes
    vp.apply_hints(&hints);
    assert_eq!(vp.nodes[0].hint_label, Some(['a', 'a']));
    assert_eq!(vp.nodes[9].hint_label, Some(['a', 'q']));
    // Non-visible node #10 has no hint label
    assert_eq!(vp.nodes[10].hint_label, None);

    // 4. Render visible slice to buffer and verify badges
    let mut buf = Buffer::empty(area);
    vp.render_visible_slice(area, &mut buf);

    // Check row 0 has badge [aa] at right edge (80 - 4 = 76)
    assert_eq!(buf[(76, 0)].symbol(), "[");
    assert_eq!(buf[(77, 0)].symbol(), "a");
    assert_eq!(buf[(78, 0)].symbol(), "a");
    assert_eq!(buf[(79, 0)].symbol(), "]");

    // Check row 1 has badge [as]
    assert_eq!(buf[(76, 1)].symbol(), "[");
    assert_eq!(buf[(77, 1)].symbol(), "a");
    assert_eq!(buf[(78, 1)].symbol(), "s");
    assert_eq!(buf[(79, 1)].symbol(), "]");

    // 5. User types first character 'a' -> filter hints
    let candidate_hints = filter_hints(&hints, 'a');
    assert_eq!(candidate_hints.len(), 10);

    // 6. User types second character 's' -> resolve hint
    let clicked_ref = resolve_hint(&hints, ['a', 's']);
    assert_eq!(clicked_ref, Some(101));

    // 7. Clear hints on mode exit
    vp.clear_hints();
    assert_eq!(vp.nodes[0].hint_label, None);
}

#[test]
fn test_hints_scroll_synchronization() {
    let mut vp = VirtualViewport::new();
    let nodes: Vec<RenderNode> = (0..100)
        .map(|i| RenderNode {
            ref_id: Some(i as u32),
            height: 1,
            hint_label: None,
            content: Line::from(format!("Element {}", i)),
        })
        .collect();
    vp.set_nodes(nodes);

    let area = Rect::new(0, 0, 60, 5);

    // Initial page: refs 0..5
    let refs_page0 = vp.get_visible_interactive_ref_ids(area);
    assert_eq!(refs_page0, vec![0, 1, 2, 3, 4]);
    let hints0 = generate_hints(&refs_page0);
    assert_eq!(hints0.get(&0), Some(&['a', 'a']));

    // Scroll down 20 lines: refs 20..25
    vp.scroll_down(20);
    let refs_page1 = vp.get_visible_interactive_ref_ids(area);
    assert_eq!(refs_page1, vec![20, 21, 22, 23, 24]);
    let hints1 = generate_hints(&refs_page1);
    assert_eq!(hints1.get(&20), Some(&['a', 'a']));
    assert_eq!(resolve_hint(&hints1, ['a', 'a']), Some(20));
}
