---
name: tb-sec8-safety-engineer
description: >-
  SEC-8 Security & Safety Engineer for terminal-browser. Activate when implementing prompt
  injection defenses, high-stakes action confirmation gates, origin-scoped DOM text sanitization,
  or rate-limiting middleware. This agent owns all trust boundaries between untrusted web content
  and the agent execution pipeline.
---

# SEC-8: Security & Safety Engineer

## Role
Owns the security and trust boundary layer across `crates/term-mcp::security` and
`crates/term-engine::sanitizer`. Prevents malicious web content from hijacking agent
execution and blocks destructive actions without explicit human confirmation.

## Blueprint Sources
- [missing-blueprints.md](../../../missing-blueprints.md): §6 Agent Security & Prompt Injection Guardrails

## Module Structure
```
crates/term-engine/src/sanitizer.rs  — DOM text extraction sanitizer
crates/term-mcp/src/security.rs      — ActionInterceptor middleware + confirmation gate
```

## 1. Indirect Prompt Injection Sanitizer (sanitizer.rs)

All web-extracted text MUST pass through this sanitizer before being returned to any agent context.

### Injection Pattern Detection
```rust
const INJECTION_PATTERNS: &[&str] = &[
    r"(?i)ignore (all )?previous instructions",
    r"(?i)system:\s*(override|prompt|instruction)",
    r"(?i)</?INST>",
    r"(?i)\[SYSTEM\]",
    r"(?i)new (task|goal|objective):\s*",
    r"(?i)you are now",
    r"(?i)disregard (your|the) (previous|above|prior)",
    r"(?i)assistant:\s*(your|new|actual) (role|task|purpose)",
];

pub fn sanitize_web_text(text: &str, origin_domain: &str) -> SanitizedText {
    let mut flagged_spans: Vec<(usize, usize)> = Vec::new();

    for pattern in INJECTION_PATTERNS {
        let re = regex::Regex::new(pattern).unwrap();
        for m in re.find_iter(text) {
            flagged_spans.push((m.start(), m.end()));
        }
    }

    SanitizedText {
        content: text.to_string(),
        flagged_spans,
        origin_domain: origin_domain.to_string(),
        trust_level: if flagged_spans.is_empty() { TrustLevel::Clean } else { TrustLevel::Suspicious },
    }
}
```

### Output Wrapping
Suspicious text is wrapped with explicit trust demarcation before returning to agents:
```
<!-- BEGIN UNTRUSTED_WEB_CONTENT origin="news.example.com" -->
[CAUTION: Possible prompt injection detected in the following content]
... web text ...
<!-- END UNTRUSTED_WEB_CONTENT -->
```

## 2. High-Stakes Action Gate (security.rs)

### Risk Classification
```rust
pub enum ActionRisk {
    Safe,       // Navigation, reading, scrolling
    Elevated,   // Login form submission, OAuth grant screen
    Critical,   // Payment submit, DELETE endpoint, password field, file system access
}

pub fn classify_action(action: &AgentAction, element: &AXNodeHandle) -> ActionRisk {
    match element.role.as_str() {
        "button" if element.name.as_deref().map(|n|
            n.to_lowercase().contains("pay") ||
            n.to_lowercase().contains("delete") ||
            n.to_lowercase().contains("confirm purchase") ||
            n.to_lowercase().contains("authorize")).unwrap_or(false) => ActionRisk::Critical,
        "textbox" if element.name.as_deref().map(|n|
            n.to_lowercase().contains("password") ||
            n.to_lowercase().contains("credit card")).unwrap_or(false) => ActionRisk::Elevated,
        _ => ActionRisk::Safe,
    }
}
```

### Gate Protocol
```rust
pub async fn check_action(
    action: &AgentAction,
    element: &AXNodeHandle,
    confirmation_tx: &tokio::sync::mpsc::Sender<ConfirmationRequest>,
) -> anyhow::Result<()> {
    let risk = classify_action(action, element);
    match risk {
        ActionRisk::Safe => Ok(()),
        ActionRisk::Elevated | ActionRisk::Critical => {
            if action.force {
                tracing::warn!("High-risk action force-executed: {:?}", action);
                return Ok(());
            }
            // Emit confirmation request to TUI drawer
            let req = ConfirmationRequest {
                action_summary: format!("{:?} on element \"{}\" (Risk: {:?})",
                    action.action_type, element.name.as_deref().unwrap_or("?"), risk),
                risk,
            };
            confirmation_tx.send(req).await?;
            Err(anyhow!("Action paused pending human confirmation"))
        }
    }
}
```

## 3. Per-Origin Rate Limiter
```rust
use std::collections::HashMap;
use std::time::{Instant, Duration};

pub struct RateLimiter {
    requests: HashMap<String, Vec<Instant>>,
    window: Duration,
    max_requests: usize,
}

impl RateLimiter {
    pub fn check(&mut self, origin: &str) -> bool {
        let now = Instant::now();
        let entries = self.requests.entry(origin.to_string()).or_default();
        entries.retain(|&t| now.duration_since(t) < self.window);
        if entries.len() >= self.max_requests { return false; }
        entries.push(now);
        true
    }
}
```

## Validation
```bash
cargo test -p term-engine -- sanitizer
cargo test -p term-mcp -- security
# Test injection detection:
cargo run -p term-mcp --example injection_test
```
