use crate::forms::TextInputWidget;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FocusRing {
    pub active_index: Option<usize>,
    pub fields: Vec<u32>, // ref_ids of form fields in document order
    pub widgets: HashMap<u32, TextInputWidget>,
    pub form_action: Option<String>,
}

impl FocusRing {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_field(&mut self, widget: TextInputWidget) {
        let ref_id = widget.ref_id;
        if !self.fields.contains(&ref_id) {
            self.fields.push(ref_id);
        }
        self.widgets.insert(ref_id, widget);
    }

    pub fn clear(&mut self) {
        self.active_index = None;
        self.fields.clear();
        self.widgets.clear();
        self.form_action = None;
    }

    pub fn focus_next(&mut self) -> Option<u32> {
        if self.fields.is_empty() {
            return None;
        }

        let next_idx = match self.active_index {
            Some(i) => (i + 1) % self.fields.len(),
            None => 0,
        };
        self.set_focus(next_idx)
    }

    pub fn focus_prev(&mut self) -> Option<u32> {
        if self.fields.is_empty() {
            return None;
        }

        let prev_idx = match self.active_index {
            Some(0) | None => self.fields.len() - 1,
            Some(i) => i - 1,
        };
        self.set_focus(prev_idx)
    }

    pub fn set_focus(&mut self, index: usize) -> Option<u32> {
        if index >= self.fields.len() {
            return None;
        }
        for w in self.widgets.values_mut() {
            w.is_focused = false;
        }

        self.active_index = Some(index);
        let ref_id = self.fields[index];
        if let Some(widget) = self.widgets.get_mut(&ref_id) {
            widget.is_focused = true;
        }
        Some(ref_id)
    }

    pub fn current_focused_widget(&self) -> Option<&TextInputWidget> {
        let ref_id = self.fields.get(self.active_index?)?;
        self.widgets.get(ref_id)
    }

    pub fn current_focused_widget_mut(&mut self) -> Option<&mut TextInputWidget> {
        let ref_id = self.fields.get(self.active_index?)?;
        self.widgets.get_mut(ref_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_ring_cycle() {
        let mut ring = FocusRing::new();
        let w1 = TextInputWidget::new(10, 100, None);
        let w2 = TextInputWidget::new(20, 200, None);
        ring.register_field(w1);
        ring.register_field(w2);

        assert_eq!(ring.focus_next(), Some(10));
        assert!(ring.current_focused_widget().unwrap().is_focused);
        assert_eq!(ring.focus_next(), Some(20));
        assert_eq!(ring.focus_next(), Some(10));
        assert_eq!(ring.focus_prev(), Some(20));
    }
}
