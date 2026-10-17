use crate::forms::{FocusRing, TextInputWidget};
use crate::viewport::RenderNode;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::collections::HashMap;

/// Parsed CSS styles applicable to HTML elements
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ElementCss {
    pub color: Option<Color>,
    pub background: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub text_align: TextAlign,
    pub border: bool,
    pub border_color: Option<Color>,
    pub display_none: bool,
    pub margin_top: u16,
    pub margin_bottom: u16,
    pub padding_left: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl ElementCss {
    pub fn to_ratatui_style(&self) -> Style {
        let mut s = Style::default();
        if let Some(c) = self.color {
            s = s.fg(c);
        }
        if let Some(bg) = self.background {
            s = s.bg(bg);
        }
        if self.bold {
            s = s.add_modifier(Modifier::BOLD);
        }
        if self.italic {
            s = s.add_modifier(Modifier::ITALIC);
        }
        if self.underline {
            s = s.add_modifier(Modifier::UNDERLINED);
        }
        s
    }

    /// Merges an overriding CSS style over a base style
    pub fn merge(&self, other: &ElementCss) -> ElementCss {
        ElementCss {
            color: other.color.or(self.color),
            background: other.background.or(self.background),
            bold: self.bold || other.bold,
            italic: self.italic || other.italic,
            underline: self.underline || other.underline,
            text_align: if other.text_align != TextAlign::Left { other.text_align } else { self.text_align },
            border: self.border || other.border,
            border_color: other.border_color.or(self.border_color),
            display_none: self.display_none || other.display_none,
            margin_top: self.margin_top.max(other.margin_top),
            margin_bottom: self.margin_bottom.max(other.margin_bottom),
            padding_left: self.padding_left + other.padding_left,
        }
    }
}

/// Parses CSS declarations from strings like `color: #ff0000; font-weight: bold; text-align: center;`
pub fn parse_inline_css(style_str: &str) -> ElementCss {
    let mut css = ElementCss::default();
    for declaration in style_str.split(';') {
        let trimmed = declaration.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }
        let prop = parts[0].trim().to_lowercase();
        let val = parts[1].trim().to_lowercase();

        match prop.as_str() {
            "color" => {
                css.color = parse_color(&val);
            }
            "background" | "background-color" => {
                css.background = parse_color(&val);
            }
            "font-weight" => {
                if val.contains("bold") || val.contains("700") || val.contains("800") || val.contains("900") {
                    css.bold = true;
                }
            }
            "font-style" => {
                if val.contains("italic") {
                    css.italic = true;
                }
            }
            "text-decoration" => {
                if val.contains("underline") {
                    css.underline = true;
                }
            }
            "text-align" => {
                if val == "center" {
                    css.text_align = TextAlign::Center;
                } else if val == "right" {
                    css.text_align = TextAlign::Right;
                } else {
                    css.text_align = TextAlign::Left;
                }
            }
            "border" | "border-style" => {
                if val != "none" && val != "0" {
                    css.border = true;
                }
            }
            "border-color" => {
                css.border_color = parse_color(&val);
                css.border = true;
            }
            "display" => {
                if val == "none" {
                    css.display_none = true;
                }
            }
            "margin-top" => {
                css.margin_top = parse_pixel_or_rem(&val);
            }
            "margin-bottom" => {
                css.margin_bottom = parse_pixel_or_rem(&val);
            }
            "padding-left" | "margin-left" => {
                css.padding_left = parse_pixel_or_rem(&val);
            }
            _ => {}
        }
    }
    css
}

/// Converts CSS color strings (hex `#fff`, `#1e90ff`, named colors `red`, `blue`, etc.) to Ratatui Color
pub fn parse_color(val: &str) -> Option<Color> {
    let clean = val.trim().to_lowercase();

    // 1. Hex colors: #RGB, #RRGGBB, #RRGGBBAA
    if clean.starts_with('#') {
        let hex = &clean[1..];
        if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            return Some(Color::Rgb(r, g, b));
        } else if hex.len() >= 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
    }

    // 2. Named colors
    match clean.as_str() {
        "red" | "crimson" => Some(Color::Red),
        "lightred" | "pink" | "coral" => Some(Color::LightRed),
        "green" | "lime" => Some(Color::Green),
        "lightgreen" => Some(Color::LightGreen),
        "yellow" | "gold" => Some(Color::Yellow),
        "lightyellow" => Some(Color::LightYellow),
        "blue" | "navy" => Some(Color::Blue),
        "lightblue" | "skyblue" | "dodgerblue" => Some(Color::LightBlue),
        "magenta" | "purple" | "violet" => Some(Color::Magenta),
        "lightmagenta" => Some(Color::LightMagenta),
        "cyan" | "teal" | "aqua" => Some(Color::Cyan),
        "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        "black" => Some(Color::Black),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        _ => None,
    }
}

fn parse_pixel_or_rem(val: &str) -> u16 {
    let num_str: String = val.chars().filter(|c| c.is_ascii_digit()).collect();
    if let Ok(num) = num_str.parse::<u16>() {
        (num / 8).max(1).min(10)
    } else {
        1
    }
}

/// Stylesheet registry storing CSS rules for tags, classes, and IDs
#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    pub tag_rules: HashMap<String, ElementCss>,
    pub class_rules: HashMap<String, ElementCss>,
    pub id_rules: HashMap<String, ElementCss>,
}

impl Stylesheet {
    pub fn parse(css_text: &str) -> Self {
        let mut sheet = Stylesheet::default();
        let mut clean = css_text.replace('\n', " ");
        while let Some(start) = clean.find('{') {
            if let Some(end) = clean[start..].find('}') {
                let actual_end = start + end;
                let selector_str = clean[..start].trim();
                let body = &clean[start + 1..actual_end];
                let css = parse_inline_css(body);

                for sel in selector_str.split(',') {
                    let s = sel.trim().to_lowercase();
                    if s.starts_with('.') {
                        sheet.class_rules.insert(s[1..].to_string(), css.clone());
                    } else if s.starts_with('#') {
                        sheet.id_rules.insert(s[1..].to_string(), css.clone());
                    } else if !s.is_empty() {
                        sheet.tag_rules.insert(s, css.clone());
                    }
                }

                clean = clean[actual_end + 1..].to_string();
            } else {
                break;
            }
        }
        sheet
    }

    pub fn resolve(&self, tag: &str, class_attr: Option<&str>, id_attr: Option<&str>, inline_style: Option<&str>) -> ElementCss {
        let mut effective = ElementCss::default();

        // 1. Tag rule
        if let Some(rule) = self.tag_rules.get(&tag.to_lowercase()) {
            effective = effective.merge(rule);
        }

        // 2. Class rules
        if let Some(classes) = class_attr {
            for c in classes.split_whitespace() {
                if let Some(rule) = self.class_rules.get(&c.to_lowercase()) {
                    effective = effective.merge(rule);
                }
            }
        }

        // 3. ID rule
        if let Some(id) = id_attr {
            if let Some(rule) = self.id_rules.get(&id.to_lowercase()) {
                effective = effective.merge(rule);
            }
        }

        // 4. Inline style (highest priority)
        if let Some(inline) = inline_style {
            let inline_css = parse_inline_css(inline);
            effective = effective.merge(&inline_css);
        }

        effective
    }
}

/// Advanced HTML & CSS visual layout engine
pub struct HtmlRenderer;

impl HtmlRenderer {
    /// Renders full HTML document into structured TUI `RenderNode`s keeping tables, inputs,
    /// buttons, blockquotes, and visual styling in place.
    pub fn render_html(
        raw_html: &str,
        link_targets: &mut HashMap<u32, String>,
        focus_ring: &mut FocusRing,
        viewport_width: u16,
    ) -> (Vec<RenderNode>, Option<String>) {
        link_targets.clear();
        focus_ring.clear();

        let mut render_nodes = Vec::new();
        let mut next_ref_id: u32 = 1;
        let mut extracted_title: Option<String> = None;

        let width = if viewport_width > 20 { viewport_width } else { 80 };

        // 1. Extract <title>
        if let Some(title_start) = raw_html.find("<title>") {
            if let Some(title_end) = raw_html[title_start..].find("</title>") {
                let title = &raw_html[title_start + 7..title_start + title_end];
                extracted_title = Some(decode_html_entities(title.trim()));
            }
        }

        // 2. Extract <style> blocks into Stylesheet
        let mut stylesheet_text = String::new();
        let mut search_pos = 0;
        while let Some(start) = raw_html[search_pos..].find("<style") {
            let actual_start = search_pos + start;
            if let Some(tag_close) = raw_html[actual_start..].find('>') {
                let content_start = actual_start + tag_close + 1;
                if let Some(end) = raw_html[content_start..].find("</style>") {
                    stylesheet_text.push_str(&raw_html[content_start..content_start + end]);
                    stylesheet_text.push('\n');
                    search_pos = content_start + end + 8;
                    continue;
                }
            }
            break;
        }
        let stylesheet = Stylesheet::parse(&stylesheet_text);

        // 3. Extract and render <table> elements in place
        // If the document contains tables, we parse them into aligned Unicode box tables
        let mut processed_html = raw_html.to_string();

        // Strip head, styles, scripts, noscripts, and svgs from body rendering
        let noise_tags = [
            r"(?is)<head[^>]*>.*?</head>",
            r"(?is)<style[^>]*>.*?</style>",
            r"(?is)<script[^>]*>.*?</script>",
            r"(?is)<noscript[^>]*>.*?</noscript>",
            r"(?is)<svg[^>]*>.*?</svg>",
            r"(?is)<!--.*?-->",
        ];
        for pattern in noise_tags {
            if let Ok(re) = regex::Regex::new(pattern) {
                processed_html = re.replace_all(&processed_html, "").to_string();
            }
        }

        // Parse tables
        if let Ok(re_table) = regex::Regex::new(r"(?is)<table[^>]*>(.*?)</table>") {
            for cap in re_table.captures_iter(&processed_html) {
                let table_inner = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let table_nodes = render_table_box(table_inner, width, &stylesheet);
                if !table_nodes.is_empty() {
                    render_nodes.extend(table_nodes);
                }
            }
        }

        // Render main body content lines preserving visual elements
        let lines: Vec<&str> = processed_html.lines().collect();
        for raw_line in lines {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with("<table") || line.starts_with("</table")
                || line.starts_with("<tr") || line.starts_with("</tr")
                || line.starts_with("<td") || line.starts_with("</td")
                || line.starts_with("<th") || line.starts_with("</th")
            {
                continue;
            }

            // Headings h1..h3
            if let Some(h1_match) = extract_tag_content(line, "h1") {
                let css = stylesheet.resolve("h1", None, None, None);
                let style = css.to_ratatui_style()
                    .fg(css.color.unwrap_or(Color::Yellow))
                    .add_modifier(Modifier::BOLD);

                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::from(vec![
                        Span::styled("█ ", Style::default().fg(Color::Yellow)),
                        Span::styled(decode_html_entities(&h1_match), style),
                    ]),
                });
                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::raw(""),
                });
                continue;
            }

            if let Some(h2_match) = extract_tag_content(line, "h2") {
                let css = stylesheet.resolve("h2", None, None, None);
                let style = css.to_ratatui_style()
                    .fg(css.color.unwrap_or(Color::Cyan))
                    .add_modifier(Modifier::BOLD);

                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::from(vec![
                        Span::styled("▓▓ ", Style::default().fg(Color::Cyan)),
                        Span::styled(decode_html_entities(&h2_match), style),
                    ]),
                });
                continue;
            }

            if let Some(h3_match) = extract_tag_content(line, "h3") {
                let css = stylesheet.resolve("h3", None, None, None);
                let style = css.to_ratatui_style()
                    .fg(css.color.unwrap_or(Color::Green))
                    .add_modifier(Modifier::BOLD);

                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::from(vec![
                        Span::styled("▌ ", Style::default().fg(Color::Green)),
                        Span::styled(decode_html_entities(&h3_match), style),
                    ]),
                });
                continue;
            }

            // Interactive Form Inputs: <input type="..." placeholder="..." value="...">
            if line.contains("<input") {
                let input_ref = next_ref_id;
                next_ref_id += 1;

                let is_password = line.contains("type=\"password\"");
                let is_checkbox = line.contains("type=\"checkbox\"");
                let is_radio = line.contains("type=\"radio\"");
                let is_submit = line.contains("type=\"submit\"");

                let placeholder = extract_attr(line, "placeholder");
                let value = extract_attr(line, "value").unwrap_or_default();
                let name = extract_attr(line, "name").unwrap_or_else(|| format!("field_{}", input_ref));

                if is_checkbox {
                    let checked = line.contains("checked");
                    let check_box = if checked { "[✔]" } else { "[ ]" };
                    render_nodes.push(RenderNode {
                        ref_id: Some(input_ref),
                        height: 1,
                        hint_label: None,
                        content: Line::from(vec![
                            Span::styled(format!("{} ", check_box), Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD)),
                            Span::raw(format!("{} (#{})", name, input_ref)),
                        ]),
                    });
                } else if is_radio {
                    let checked = line.contains("checked");
                    let radio_box = if checked { "(●)" } else { "( )" };
                    render_nodes.push(RenderNode {
                        ref_id: Some(input_ref),
                        height: 1,
                        hint_label: None,
                        content: Line::from(vec![
                            Span::styled(format!("{} ", radio_box), Style::default().fg(Color::Yellow)),
                            Span::raw(format!("{} (#{})", name, input_ref)),
                        ]),
                    });
                } else if is_submit {
                    let label = if !value.is_empty() { value } else { "Submit".to_string() };
                    render_nodes.push(RenderNode {
                        ref_id: Some(input_ref),
                        height: 1,
                        hint_label: None,
                        content: Line::from(vec![
                            Span::styled(format!("[ ⯈ {} ]", label), Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
                            Span::styled(format!(" (#{})", input_ref), Style::default().fg(Color::DarkGray)),
                        ]),
                    });
                } else {
                    // Text / Password input box
                    let display_val = if !value.is_empty() {
                        if is_password { "••••••••••••".to_string() } else { value.clone() }
                    } else if let Some(p) = &placeholder {
                        format!("{}...", p)
                    } else {
                        "________________________".to_string()
                    };

                    let box_line = format!("[ {:<28} ]", display_val);
                    render_nodes.push(RenderNode {
                        ref_id: Some(input_ref),
                        height: 1,
                        hint_label: None,
                        content: Line::from(vec![
                            Span::styled(box_line, Style::default().fg(Color::White).bg(Color::Rgb(30, 30, 35))),
                            Span::styled(format!(" (#{})", input_ref), Style::default().fg(Color::DarkGray)),
                        ]),
                    });

                    // Register into FocusRing
                    let mut widget = TextInputWidget::new(input_ref, input_ref, placeholder);
                    widget.value = value;
                    focus_ring.register_field(widget);
                }
                continue;
            }

            // Buttons: <button>Text</button>
            if let Some(btn_match) = extract_tag_content(line, "button") {
                let btn_ref = next_ref_id;
                next_ref_id += 1;
                let clean_text = decode_html_entities(&strip_html_tags(&btn_match)).trim().to_string();
                let display_text = if !clean_text.is_empty() { clean_text } else { "Button".to_string() };

                render_nodes.push(RenderNode {
                    ref_id: Some(btn_ref),
                    height: 1,
                    hint_label: None,
                    content: Line::from(vec![
                        Span::styled(format!("[ ⯈ {} ]", display_text), Style::default().fg(Color::White).bg(Color::Blue).add_modifier(Modifier::BOLD)),
                        Span::styled(format!(" (#{})", btn_ref), Style::default().fg(Color::DarkGray)),
                    ]),
                });
                continue;
            }

            // Blockquotes: <blockquote>...</blockquote>
            if let Some(quote_match) = extract_tag_content(line, "blockquote") {
                let clean_quote = decode_html_entities(&quote_match);
                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::from(vec![
                        Span::styled("▎ ", Style::default().fg(Color::LightBlue).add_modifier(Modifier::BOLD)),
                        Span::styled(clean_quote, Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC)),
                    ]),
                });
                continue;
            }

            // Pre / Code blocks: <pre><code>...</code></pre>
            if line.contains("<pre") || line.contains("<code") {
                let clean_code = decode_html_entities(line)
                    .replace("<pre>", "")
                    .replace("</pre>", "")
                    .replace("<code>", "")
                    .replace("</code>", "");

                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::from(vec![
                        Span::styled("  │ ", Style::default().fg(Color::DarkGray)),
                        Span::styled(clean_code, Style::default().fg(Color::Green)),
                    ]),
                });
                continue;
            }

            // Horizontal rules: <hr>
            if line.contains("<hr") {
                let rule_str = "─".repeat(width.saturating_sub(4) as usize);
                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::from(vec![
                        Span::styled(rule_str, Style::default().fg(Color::DarkGray)),
                    ]),
                });
                continue;
            }

            // Links: <a href="...">text</a>
            if line.contains("<a ") && line.contains("</a>") {
                let mut spans = Vec::new();
                let mut remaining = line;
                let mut line_ref = None;

                while let Some(start_a) = remaining.find("<a ") {
                    if let Some(tag_end) = remaining[start_a..].find('>') {
                        let actual_tag_end = start_a + tag_end;
                        let tag_str = &remaining[start_a..actual_tag_end + 1];
                        let href = extract_attr(tag_str, "href").unwrap_or_default();

                        if let Some(close_a) = remaining[actual_tag_end..].find("</a>") {
                            let label_raw = &remaining[actual_tag_end + 1..actual_tag_end + close_a];
                            let label_clean = if label_raw.contains("<img") {
                                extract_attr(label_raw, "alt")
                                    .or_else(|| extract_attr(label_raw, "title"))
                                    .unwrap_or_else(|| strip_html_tags(label_raw))
                            } else {
                                strip_html_tags(label_raw)
                            };
                            let label = decode_html_entities(&label_clean).trim().to_string();
                            let display_label = if !label.is_empty() { label } else { href.clone() };

                            let prefix = &remaining[..start_a];
                            if !prefix.is_empty() {
                                spans.push(Span::raw(strip_html_tags(prefix)));
                            }

                            let link_ref = next_ref_id;
                            next_ref_id += 1;
                            line_ref = Some(link_ref);
                            link_targets.insert(link_ref, href);

                            spans.push(Span::styled(
                                format!("[{}]", display_label),
                                Style::default().fg(Color::LightBlue).add_modifier(Modifier::UNDERLINED),
                            ));
                            spans.push(Span::styled(
                                format!(" (#{})", link_ref),
                                Style::default().fg(Color::DarkGray),
                            ));

                            remaining = &remaining[actual_tag_end + close_a + 4..];
                            continue;
                        }
                    }
                    break;
                }

                if !remaining.is_empty() {
                    spans.push(Span::raw(strip_html_tags(remaining)));
                }

                render_nodes.push(RenderNode {
                    ref_id: line_ref,
                    height: 1,
                    hint_label: None,
                    content: Line::from(spans),
                });
                continue;
            }

            // Standard paragraphs and text with CSS styling
            let clean_text = decode_html_entities(&strip_html_tags(line));
            if !clean_text.trim().is_empty() {
                let css = stylesheet.resolve("p", None, None, None);
                let style = css.to_ratatui_style();

                let aligned_line = match css.text_align {
                    TextAlign::Center => {
                        let pad = (width as usize).saturating_sub(clean_text.len()) / 2;
                        format!("{}{}", " ".repeat(pad), clean_text)
                    }
                    TextAlign::Right => {
                        let pad = (width as usize).saturating_sub(clean_text.len());
                        format!("{}{}", " ".repeat(pad), clean_text)
                    }
                    TextAlign::Left => clean_text,
                };

                render_nodes.push(RenderNode {
                    ref_id: None,
                    height: 1,
                    hint_label: None,
                    content: Line::from(vec![Span::styled(aligned_line, style)]),
                });
            }
        }

        if render_nodes.is_empty() {
            render_nodes.push(RenderNode {
                ref_id: None,
                height: 1,
                hint_label: None,
                content: Line::raw("(Empty webpage)"),
            });
        }

        (render_nodes, extracted_title)
    }
}

/// Helper to parse HTML tables into a beautiful, aligned Unicode box table
fn render_table_box(table_html: &str, max_width: u16, _stylesheet: &Stylesheet) -> Vec<RenderNode> {
    let mut rows: Vec<Vec<(String, bool)>> = Vec::new(); // (text, is_header)

    let re_tr = match regex::Regex::new(r"(?is)<tr[^>]*>(.*?)</tr>") {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let re_cell = match regex::Regex::new(r#"(?is)<(th|td)[^>]*>(.*?)</(?:th|td)>"#) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    for tr_cap in re_tr.captures_iter(table_html) {
        let tr_inner = tr_cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let mut row_cells = Vec::new();

        for cell_cap in re_cell.captures_iter(tr_inner) {
            let tag = cell_cap.get(1).map(|m| m.as_str()).unwrap_or("td");
            let content = cell_cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let clean = decode_html_entities(&strip_html_tags(content)).trim().to_string();
            row_cells.push((clean, tag.eq_ignore_ascii_case("th")));
        }

        if !row_cells.is_empty() {
            rows.push(row_cells);
        }
    }

    if rows.is_empty() {
        return Vec::new();
    }

    // Determine number of columns
    let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if num_cols == 0 {
        return Vec::new();
    }

    // Compute column widths
    let mut col_widths = vec![4usize; num_cols];
    for row in &rows {
        for (i, (cell, _)) in row.iter().enumerate() {
            if i < num_cols {
                col_widths[i] = col_widths[i].max(cell.len() + 2);
            }
        }
    }

    // Adjust column widths to fit max_width if necessary
    let total_width: usize = col_widths.iter().sum::<usize>() + num_cols + 1;
    if total_width > max_width as usize && max_width > 30 {
        let max_col = (max_width as usize - num_cols - 1) / num_cols;
        for w in &mut col_widths {
            *w = (*w).min(max_col.max(6));
        }
    }

    let mut nodes = Vec::new();

    // Top border: ┌──────┬──────┐
    let mut top_line = String::from("┌");
    for (i, w) in col_widths.iter().enumerate() {
        top_line.push_str(&"─".repeat(*w));
        if i < num_cols - 1 {
            top_line.push('┬');
        } else {
            top_line.push('┐');
        }
    }
    nodes.push(RenderNode {
        ref_id: None,
        height: 1,
        hint_label: None,
        content: Line::from(vec![Span::styled(top_line, Style::default().fg(Color::DarkGray))]),
    });

    // Rows
    for (r_idx, row) in rows.iter().enumerate() {
        let is_header_row = row.iter().any(|(_, is_th)| *is_th);
        let mut row_spans = Vec::new();
        row_spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));

        for (i, w) in col_widths.iter().enumerate() {
            let (text, is_th) = row.get(i).map(|(s, h)| (s.as_str(), *h)).unwrap_or(("", false));
            let truncated = if text.len() > w.saturating_sub(1) {
                format!("{}…", &text[..w.saturating_sub(2)])
            } else {
                text.to_string()
            };
            let cell_padded = format!(" {:<width$}", truncated, width = w.saturating_sub(1));

            let cell_style = if is_th || is_header_row {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            row_spans.push(Span::styled(cell_padded, cell_style));
            row_spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
        }

        nodes.push(RenderNode {
            ref_id: None,
            height: 1,
            hint_label: None,
            content: Line::from(row_spans),
        });

        // Header separator: ├──────┼──────┤
        if is_header_row && r_idx < rows.len() - 1 {
            let mut sep_line = String::from("├");
            for (i, w) in col_widths.iter().enumerate() {
                sep_line.push_str(&"─".repeat(*w));
                if i < num_cols - 1 {
                    sep_line.push('┼');
                } else {
                    sep_line.push('┤');
                }
            }
            nodes.push(RenderNode {
                ref_id: None,
                height: 1,
                hint_label: None,
                content: Line::from(vec![Span::styled(sep_line, Style::default().fg(Color::DarkGray))]),
            });
        }
    }

    // Bottom border: └──────┴──────┘
    let mut bot_line = String::from("└");
    for (i, w) in col_widths.iter().enumerate() {
        bot_line.push_str(&"─".repeat(*w));
        if i < num_cols - 1 {
            bot_line.push('┴');
        } else {
            bot_line.push('┘');
        }
    }
    nodes.push(RenderNode {
        ref_id: None,
        height: 1,
        hint_label: None,
        content: Line::from(vec![Span::styled(bot_line, Style::default().fg(Color::DarkGray))]),
    });

    nodes
}

fn extract_tag_content(html: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start_idx = html.find(&open)?;
    let tag_close_idx = html[start_idx..].find('>')?;
    let content_start = start_idx + tag_close_idx + 1;
    let end_idx = html[content_start..].find(&close)?;
    Some(html[content_start..content_start + end_idx].to_string())
}

fn extract_attr(tag: &str, attr: &str) -> Option<String> {
    let pattern_double = format!("{}=\"", attr);
    let pattern_single = format!("{}='", attr);

    if let Some(idx) = tag.find(&pattern_double) {
        let val_start = idx + pattern_double.len();
        if let Some(end) = tag[val_start..].find('"') {
            return Some(tag[val_start..val_start + end].to_string());
        }
    }

    if let Some(idx) = tag.find(&pattern_single) {
        let val_start = idx + pattern_single.len();
        if let Some(end) = tag[val_start..].find('\'') {
            return Some(tag[val_start..val_start + end].to_string());
        }
    }

    None
}

fn strip_html_tags(s: &str) -> String {
    let mut in_tag = false;
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }
    out
}

fn decode_html_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&middot;", "·")
        .replace("&bull;", "•")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_inline_css() {
        let css = parse_inline_css("color: #ff0000; font-weight: bold; text-align: center;");
        assert_eq!(css.color, Some(Color::Rgb(255, 0, 0)));
        assert!(css.bold);
        assert_eq!(css.text_align, TextAlign::Center);
    }

    #[test]
    fn test_table_box_rendering() {
        let table_html = "
            <table>
                <tr><th>Feature</th><th>Status</th></tr>
                <tr><td>CDP</td><td>Active</td></tr>
            </table>
        ";
        let nodes = render_table_box(table_html, 80, &Stylesheet::default());
        assert!(!nodes.is_empty());
        // Should contain top border ┌, content │, separator ├, and bottom border └
        let text: String = nodes.iter().map(|n| format!("{:?}", n.content)).collect();
        assert!(text.contains('┌'));
        assert!(text.contains('│'));
        assert!(text.contains('└'));
    }

    #[test]
    fn test_form_inputs_render_in_place() {
        let html = r#"
            <form>
                <input type="text" placeholder="Username" value="antigravity" />
                <button>Submit</button>
            </form>
        "#;
        let mut links = HashMap::new();
        let mut focus_ring = FocusRing::new();
        let (nodes, _) = HtmlRenderer::render_html(html, &mut links, &mut focus_ring, 80);

        assert!(!nodes.is_empty());
        assert_eq!(focus_ring.fields.len(), 1);
        let text: String = nodes.iter().map(|n| format!("{:?}", n.content)).collect();
        assert!(text.contains("antigravity"));
        assert!(text.contains("Submit"));
    }
}
