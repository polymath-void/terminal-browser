use regex::Regex;

const INJECTION_PATTERNS: &[&str] = &[
    r"(?i)ignore (all )?previous instructions",
    r"(?i)system:\s*(override|prompt|instruction)",
    r"(?i)</?INST>",
    r"(?i)\[SYSTEM\]",
    r"(?i)new (task|goal|objective):\s*",
    r"(?i)you are now",
    r"(?i)disregard (your|the) (previous|above|prior)",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedText {
    pub content: String,
    pub is_suspicious: bool,
    pub origin_domain: String,
}

/// Converts raw HTML content into clean, readable Markdown by stripping scripts, styles,
/// and converting semantic markup into Markdown format.
pub fn html_to_markdown(raw_html: &str) -> String {
    let mut text = raw_html.to_string();

    // 1. Remove script, style, noscript, svg blocks and HTML comments
    let noise_tags = [
        r"(?is)<script[^>]*>.*?</script>",
        r"(?is)<style[^>]*>.*?</style>",
        r"(?is)<noscript[^>]*>.*?</noscript>",
        r"(?is)<svg[^>]*>.*?</svg>",
        r"(?is)<!--.*?-->",
    ];
    for pattern in noise_tags {
        if let Ok(re) = Regex::new(pattern) {
            text = re.replace_all(&text, "").to_string();
        }
    }

    // 2. Headings
    if let Ok(re) = Regex::new(r"(?is)<h1[^>]*>(.*?)</h1>") {
        text = re.replace_all(&text, "\n\n# $1\n\n").to_string();
    }
    if let Ok(re) = Regex::new(r"(?is)<h2[^>]*>(.*?)</h2>") {
        text = re.replace_all(&text, "\n\n## $1\n\n").to_string();
    }
    if let Ok(re) = Regex::new(r"(?is)<h3[^>]*>(.*?)</h3>") {
        text = re.replace_all(&text, "\n\n### $1\n\n").to_string();
    }

    // 3. Links: <a href="url">text</a> -> [text](url)
    if let Ok(re) = Regex::new(r#"(?is)<a[^>]*href=["']([^"']*)["'][^>]*>(.*?)</a>"#) {
        text = re.replace_all(&text, |caps: &regex::Captures| {
            let href = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let label = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
            if label.is_empty() {
                String::new()
            } else {
                format!(" [{}]({}) ", label, href)
            }
        }).to_string();
    }

    // 4. Buttons and inputs
    if let Ok(re) = Regex::new(r#"(?is)<button[^>]*>(.*?)</button>"#) {
        text = re.replace_all(&text, " [button: $1] ").to_string();
    }
    if let Ok(re) = Regex::new(r#"(?is)<input[^>]*value=["']([^"']+)["'][^>]*>"#) {
        text = re.replace_all(&text, " [input: $1] ").to_string();
    }

    // 5. Lists & Paragraphs & Line Breaks
    if let Ok(re) = Regex::new(r"(?is)<li[^>]*>(.*?)</li>") {
        text = re.replace_all(&text, "\n* $1").to_string();
    }
    if let Ok(re) = Regex::new(r"(?is)<p[^>]*>(.*?)</p>") {
        text = re.replace_all(&text, "\n\n$1\n\n").to_string();
    }
    if let Ok(re) = Regex::new(r"(?i)<br\s*/?>") {
        text = re.replace_all(&text, "\n").to_string();
    }

    // 6. Strip remaining HTML tags
    if let Ok(re) = Regex::new(r"<[^>]+>") {
        text = re.replace_all(&text, " ").to_string();
    }

    // 7. Decode HTML entities
    text = text
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&middot;", "·");

    // 8. Clean up whitespace and empty lines
    let mut cleaned_lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            cleaned_lines.push(trimmed);
        }
    }

    cleaned_lines.join("\n\n")
}

pub fn sanitize_web_text(text: &str, origin_domain: &str) -> SanitizedText {
    // If the content is HTML, convert to clean readable Markdown first
    let processed_text = if text.contains("<html") || text.contains("<!DOCTYPE") || text.contains("<body") || text.contains("<script") {
        html_to_markdown(text)
    } else {
        text.to_string()
    };

    let mut is_suspicious = false;
    for pattern in INJECTION_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(&processed_text) {
                is_suspicious = true;
                break;
            }
        }
    }

    let content = if is_suspicious {
        format!(
            "<!-- BEGIN UNTRUSTED_WEB_CONTENT origin=\"{}\" flagged=\"injection_risk\" -->\n{}\n<!-- END UNTRUSTED_WEB_CONTENT -->",
            origin_domain, processed_text
        )
    } else {
        processed_text
    };

    SanitizedText {
        content,
        is_suspicious,
        origin_domain: origin_domain.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_markdown_strips_scripts_and_styles() {
        let html = r#"
            <html>
            <head>
                <style>body { color: red; }</style>
                <script>alert("evil tracking");</script>
            </head>
            <body>
                <h1>Welcome to Example</h1>
                <p>This is a <a href="https://example.com/about">About Us</a> page.</p>
                <script>window.telemetry = 1;</script>
            </body>
            </html>
        "#;
        let md = html_to_markdown(html);
        assert!(!md.contains("evil tracking"));
        assert!(!md.contains("color: red"));
        assert!(!md.contains("telemetry"));
        assert!(md.contains("# Welcome to Example"));
        assert!(md.contains("[About Us](https://example.com/about)"));
    }

    #[test]
    fn test_sanitize_preserves_clean_text() {
        let result = sanitize_web_text("Normal content about Rust programming.", "example.com");
        assert!(!result.is_suspicious);
        assert_eq!(result.content, "Normal content about Rust programming.");
    }
}
