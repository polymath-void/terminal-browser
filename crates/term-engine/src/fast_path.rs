use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONNECTION, USER_AGENT};
use url::Url;

pub const DEFAULT_AGENT_USER_AGENT: &str =
    "TerminalAgent/1.0 (+https://github.com/org/terminal-browser) Chrome/130.0.0.0";

#[derive(Debug, Clone)]
pub struct FastPathClient {
    client: reqwest::Client,
    user_agent: String,
}

impl Default for FastPathClient {
    fn default() -> Self {
        Self::new(None)
    }
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

    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    pub fn build_headers(&self) -> HeaderMap {
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
        self.probe_llms_txt_timed(target_url).await.map(|(text, _, _)| text)
    }

    /// Probes for domain-level `/llms.txt` summary file with latency measurement
    pub async fn probe_llms_txt_timed(
        &self,
        target_url: &str,
    ) -> Option<(String, std::time::Duration, u16)> {
        let parsed = Url::parse(target_url).ok()?;
        let llms_url = format!("{}://{}/llms.txt", parsed.scheme(), parsed.host_str()?);
        let start = std::time::Instant::now();
        let resp = self
            .client
            .get(&llms_url)
            .headers(self.build_headers())
            .send()
            .await
            .ok()?;
        let duration = start.elapsed();
        let status = resp.status().as_u16();

        if resp.status().is_success() {
            let text = resp.text().await.ok()?;
            Some((text, duration, status))
        } else {
            None
        }
    }

    /// Directly negotiates content for markdown / plain text before launching browser
    pub async fn fetch_negotiated(&self, target_url: &str) -> anyhow::Result<Option<String>> {
        self.fetch_negotiated_timed(target_url).await.map(|opt| opt.map(|(text, _, _)| text))
    }

    /// Directly negotiates content for markdown / plain text with latency measurement
    pub async fn fetch_negotiated_timed(
        &self,
        target_url: &str,
    ) -> anyhow::Result<Option<(String, std::time::Duration, u16)>> {
        let start = std::time::Instant::now();
        let resp = self
            .client
            .get(target_url)
            .headers(self.build_headers())
            .send()
            .await?;
        let duration = start.elapsed();
        let status = resp.status().as_u16();

        if resp.status().is_success() {
            let content_type = resp
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");

            if content_type.contains("text/markdown") || content_type.contains("text/plain") {
                let text = resp.text().await?;
                return Ok(Some((text, duration, status)));
            }
        }
        Ok(None)
    }

    /// Fetches page content (either negotiated markdown, /llms.txt, or HTML fallback)
    pub async fn fetch_page_content(&self, target_url: &str) -> anyhow::Result<(String, u16)> {
        if let Some((llms, _, status)) = self.probe_llms_txt_timed(target_url).await {
            return Ok((llms, status));
        }

        if let Ok(Some((md, _, status))) = self.fetch_negotiated_timed(target_url).await {
            return Ok((md, status));
        }

        let resp = self
            .client
            .get(target_url)
            .headers(self.build_headers())
            .send()
            .await?;
        let status = resp.status().as_u16();
        let text = resp.text().await?;
        let domain = url::Url::parse(target_url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());
        let sanitized = crate::sanitizer::sanitize_web_text(&text, &domain);
        Ok((sanitized.content, status))
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_headers_contain_agent_protocols() {
        let client = FastPathClient::new(None);
        let headers = client.build_headers();

        assert_eq!(
            headers.get(USER_AGENT).unwrap().to_str().unwrap(),
            DEFAULT_AGENT_USER_AGENT
        );
        assert_eq!(
            headers.get(ACCEPT).unwrap().to_str().unwrap(),
            "text/markdown, text/plain;q=0.9, text/html;q=0.5"
        );
        assert_eq!(
            headers.get("Sec-Purpose").unwrap().to_str().unwrap(),
            "ai-agent-reading"
        );
        assert_eq!(
            headers.get("X-Agent-Protocol").unwrap().to_str().unwrap(),
            "MCP/1.0"
        );
        assert_eq!(
            headers.get(CONNECTION).unwrap().to_str().unwrap(),
            "keep-alive"
        );
    }

    #[test]
    fn test_custom_user_agent_override() {
        let custom_ua = "CustomAgent/2.5";
        let client = FastPathClient::new(Some(custom_ua.to_string()));
        assert_eq!(client.user_agent(), custom_ua);

        let headers = client.build_headers();
        assert_eq!(
            headers.get(USER_AGENT).unwrap().to_str().unwrap(),
            custom_ua
        );
    }

    #[test]
    fn test_llms_url_resolution() {
        let url = "https://docs.anthropic.com/en/docs/overview";
        let parsed = Url::parse(url).unwrap();
        let llms_url = format!("{}://{}/llms.txt", parsed.scheme(), parsed.host_str().unwrap());
        assert_eq!(llms_url, "https://docs.anthropic.com/llms.txt");
    }
}
