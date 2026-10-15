#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsProtocol {
    Kitty,
    Sixel,
    Braille,
    Ascii,
}

impl GraphicsProtocol {
    pub fn detect() -> Self {
        if std::env::var("KITTY_WINDOW_ID").is_ok() {
            return GraphicsProtocol::Kitty;
        }
        if let Ok(term) = std::env::var("TERM") {
            if term.contains("sixel") || term.contains("foot") || term.contains("wezterm") {
                return GraphicsProtocol::Sixel;
            }
        }
        GraphicsProtocol::Braille
    }
}
