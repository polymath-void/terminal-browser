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

#[derive(Debug, Clone)]
pub struct SanitizedText {
    pub content: String,
    pub is_suspicious: bool,
    pub origin_domain: String,
}

pub fn sanitize_web_text(text: &str, origin_domain: &str) -> SanitizedText {
    let mut is_suspicious = false;
    for pattern in INJECTION_PATTERNS {
        if let Ok(re) = Regex::new(pattern) {
            if re.is_match(text) {
                is_suspicious = true;
                break;
            }
        }
    }

    let content = if is_suspicious {
        format!(
            "<!-- BEGIN UNTRUSTED_WEB_CONTENT origin=\"{}\" flagged=\"injection_risk\" -->\n{}\n<!-- END UNTRUSTED_WEB_CONTENT -->",
            origin_domain, text
        )
    } else {
        text.to_string()
    };

    SanitizedText {
        content,
        is_suspicious,
        origin_domain: origin_domain.to_string(),
    }
}
