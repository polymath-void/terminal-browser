use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONNECTION, USER_AGENT};
use url::Url;

pub const DEFAULT_AGENT_USER_AGENT: &str =
    "TerminalAgent/1.0 (+https://github.com/org/terminal-browser) Chrome/130.0.0.0";

pub struct FastPathClient {
    client: reqwest::Client,
    user_agent: String,
}

impl FastPathClient {
    pub fn new(user_agent: Option<String>) -> Self {
        let ua = user_agent.unwrap_or_else(|| DEFAULT_AGENT_USER_AGENT.to_string());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .build()
            .unwrap_or_default();
        Self { client, user_agent: ua }
    }

    fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_str(&self.user_agent).unwrap());
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("text/markdown, text/plain;q=0.9, text/html;q=0.5"),
        );
        headers.insert(
            "Sec-Purpose",
            HeaderValue::from_static("ai-agent-reading"),
        );
        headers.insert(
            "X-Agent-Protocol",
            HeaderValue::from_static("MCP/1.0"),
        );
        headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
        headers
    }

    /// Probes for domain-level `/llms.txt` summary file
    pub async fn probe_llms_txt(&self, target_url: &str) -> Option<String> {
        let parsed = Url::parse(target_url).ok()?;
        let llms_url = format!("{}://{}/llms.txt", parsed.scheme(), parsed.host_str()?);
        let resp = self
            .client
            .get(&llms_url)
            .headers(self.build_headers())
            .send()
            .await
            .ok()?;

        if resp.status().is_success() {
            resp.text().await.ok()
        } else {
            None
        }
    }

    /// Directly negotiates content for markdown / plain text before launching browser
    pub async fn fetch_negotiated(&self, target_url: &str) -> anyhow::Result<Option<String>> {
        let resp = self
            .client
            .get(target_url)
            .headers(self.build_headers())
            .send()
            .await?;

        if resp.status().is_success() {
            let content_type = resp
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if content_type.contains("text/markdown") || content_type.contains("text/plain") {
                let text = resp.text().await?;
                return Ok(Some(text));
            }
        }
        Ok(None)
    }
}
