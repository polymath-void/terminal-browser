pub struct SyntheticDispatcher;

impl SyntheticDispatcher {
    /// Builds a JavaScript snippet to dispatch synthetic input and change events for React/SPA hydration
    pub fn build_react_change_script(backend_node_id: u32, new_value: &str) -> String {
        format!(
            r#"(() => {{
                const el = window.__term_resolve_node({backend_node_id});
                if (!el) return false;
                const proto = el instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
                const nativeSetter = Object.getOwnPropertyDescriptor(proto, 'value').set;
                nativeSetter.call(el, {value:?});
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                el.dispatchEvent(new Event('change', {{ bubbles: true }}));
                return true;
            }})()"#,
            backend_node_id = backend_node_id,
            value = new_value
        )
    }
}
