use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONNECTION, USER_AGENT};
use std::sync::{Arc, Mutex};
use term_storage::{CookieVault, DatabaseManager, StoredCookie};
use url::Url;

pub const DEFAULT_AGENT_USER_AGENT: &str =
    "TerminalAgent/1.0 (+https://github.com/org/terminal-browser) Chrome/130.0.0.0";

#[derive(Clone)]
pub struct FastPathClient {
    client: reqwest::Client,
    user_agent: String,
    vault: Option<Arc<Mutex<DatabaseManager>>>,
}

impl std::fmt::Debug for FastPathClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FastPathClient")
            .field("user_agent", &self.user_agent)
            .field("has_vault", &self.vault.is_some())
            .finish()
    }
}

impl Default for FastPathClient {
    fn default() -> Self {
        Self::new(None)
    }
}

impl FastPathClient {
    /// Creates a new `FastPathClient` with default SQLite vault connection
    pub fn new(user_agent: Option<String>) -> Self {
        let vault = DatabaseManager::open_default()
            .ok()
            .map(|db| Arc::new(Mutex::new(db)));
        Self::new_with_vault(user_agent, vault)
    }

    /// Creates a new `FastPathClient` with an explicit vault manager (e.g. in-memory for testing)
    pub fn new_with_vault(
        user_agent: Option<String>,
        vault: Option<Arc<Mutex<DatabaseManager>>>,
    ) -> Self {
        let ua = user_agent.unwrap_or_else(|| DEFAULT_AGENT_USER_AGENT.to_string());
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            client,
            user_agent: ua,
            vault,
        }
    }

    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }

    pub fn vault(&self) -> Option<Arc<Mutex<DatabaseManager>>> {
        self.vault.clone()
    }

    pub fn build_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_str(&self.user_agent).unwrap());
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("text/markdown, text/plain;q=0.9, text/html;q=0.5, */*;q=0.1"),
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

    /// Queries SQLite CookieVault for the domain and builds the `Cookie` request header
    pub fn get_cookie_header_for_url(&self, target_url: &str) -> Option<String> {
        let parsed = Url::parse(target_url).ok()?;
        let domain = parsed.host_str()?;
        let vault_arc = self.vault.as_ref()?;
        let vault = vault_arc.lock().ok()?;
        let cookies = CookieVault::get_for_domain(vault.conn(), domain).ok()?;
        CookieVault::build_cookie_header(&cookies)
    }

    /// Executes an HTTP request with automatic RFC 6265 cookie injection and redirect handling.
    /// Captures `Set-Cookie` headers into SQLite `CookieVault` at every hop.
    pub async fn execute_with_redirects(
        &self,
        method: reqwest::Method,
        target_url: &str,
        form_body: Option<Vec<(String, String)>>,
        max_redirects: usize,
    ) -> anyhow::Result<(reqwest::Response, String, Vec<StoredCookie>)> {
        let mut current_url = target_url.to_string();
        let mut current_method = method;
        let mut current_form = form_body;
        let mut captured_cookies = Vec::new();

        for hop in 0..=max_redirects {
            let parsed_url = Url::parse(&current_url)?;
            let domain = parsed_url.host_str().unwrap_or("").to_string();

            let mut headers = self.build_headers();
            if let Some(cookie_hdr) = self.get_cookie_header_for_url(&current_url) {
                if let Ok(hdr_val) = HeaderValue::from_str(&cookie_hdr) {
                    headers.insert(reqwest::header::COOKIE, hdr_val);
                }
            }

            let mut req = self
                .client
                .request(current_method.clone(), &current_url)
                .headers(headers);

            if let Some(form) = &current_form {
                req = req.form(form);
            }

            let resp = req.send().await?;

            // 1. Capture Set-Cookie headers into CookieVault
            for cookie_header in resp.headers().get_all(reqwest::header::SET_COOKIE) {
                if let Ok(raw_str) = cookie_header.to_str() {
                    if let Some(parsed_cookie) = CookieVault::parse_set_cookie(&domain, raw_str) {
                        if let Some(vault_arc) = &self.vault {
                            if let Ok(vault) = vault_arc.lock() {
                                let _ = CookieVault::upsert(vault.conn(), &parsed_cookie);
                            }
                        }
                        captured_cookies.push(parsed_cookie);
                    }
                }
            }

            // 2. Handle HTTP Redirections (301, 302, 303, 307, 308)
            if resp.status().is_redirection() {
                if hop >= max_redirects {
                    anyhow::bail!("Too many redirects while requesting: {}", current_url);
                }

                if let Some(loc_val) = resp
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                {
                    let next_url = parsed_url.join(loc_val)?.to_string();
                    let status = resp.status().as_u16();

                    // RFC 7231: 303 changes method to GET and drops body
                    // Common web convention: 301/302 from POST also changes to GET
                    if status == 303
                        || ((status == 301 || status == 302) && current_method == reqwest::Method::POST)
                    {
                        current_method = reqwest::Method::GET;
                        current_form = None;
                    }

                    current_url = next_url;
                    continue;
                }
            }

            return Ok((resp, current_url, captured_cookies));
        }

        anyhow::bail!("Exceeded maximum redirect limit for {}", target_url);
    }

    /// Submits form data (e.g. login credentials) via POST and stores captured session cookies
    pub async fn post_form(
        &self,
        target_url: &str,
        form_data: &[(&str, &str)],
    ) -> anyhow::Result<(String, u16, Vec<StoredCookie>)> {
        let form_vec = form_data
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let (resp, final_url, cookies) = self
            .execute_with_redirects(reqwest::Method::POST, target_url, Some(form_vec), 10)
            .await?;
        let status = resp.status().as_u16();
        let text = resp.text().await?;
        let domain = Url::parse(&final_url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());
        let sanitized = crate::sanitizer::sanitize_web_text(&text, &domain);
        Ok((sanitized.content, status, cookies))
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
        let (resp, _, _) = self
            .execute_with_redirects(reqwest::Method::GET, &llms_url, None, 3)
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
        let (resp, _, _) = self
            .execute_with_redirects(reqwest::Method::GET, target_url, None, 5)
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

        let (resp, final_url, _) = self
            .execute_with_redirects(reqwest::Method::GET, target_url, None, 10)
            .await?;
        let status = resp.status().as_u16();
        let text = resp.text().await?;
        let domain = Url::parse(&final_url)
            .ok()
            .and_then(|u| u.host_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());
        let sanitized = crate::sanitizer::sanitize_web_text(&text, &domain);
        Ok((sanitized.content, status))
    }

    /// Retrieves all cookies stored in the vault for a given domain
    pub fn get_stored_cookies_for_domain(&self, domain: &str) -> Vec<StoredCookie> {
        if let Some(vault_arc) = &self.vault {
            if let Ok(vault) = vault_arc.lock() {
                return CookieVault::get_for_domain(vault.conn(), domain).unwrap_or_default();
            }
        }
        Vec::new()
    }

    /// Retrieves all cookies stored across all domains
    pub fn get_all_stored_cookies(&self) -> Vec<StoredCookie> {
        if let Some(vault_arc) = &self.vault {
            if let Ok(vault) = vault_arc.lock() {
                return CookieVault::get_all(vault.conn()).unwrap_or_default();
            }
        }
        Vec::new()
    }

    /// Clears all cookies from the vault
    pub fn clear_all_cookies(&self) -> anyhow::Result<usize> {
        if let Some(vault_arc) = &self.vault {
            if let Ok(vault) = vault_arc.lock() {
                return CookieVault::clear_all(vault.conn());
            }
        }
        Ok(0)
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
        assert!(headers
            .get(ACCEPT)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("text/markdown"));
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

    #[test]
    fn test_cookie_injection_from_vault() {
        let db = DatabaseManager::open_in_memory().unwrap();
        let cookie = StoredCookie {
            domain: "example.com".to_string(),
            name: "session_id".to_string(),
            value: "token_999".to_string(),
            path: "/".to_string(),
            expires_at: None,
            is_secure: true,
            is_httponly: true,
            same_site: "Lax".to_string(),
        };
        CookieVault::upsert(db.conn(), &cookie).unwrap();

        let client = FastPathClient::new_with_vault(None, Some(Arc::new(Mutex::new(db))));
        let cookie_header = client.get_cookie_header_for_url("https://example.com/dashboard");
        assert_eq!(cookie_header, Some("session_id=token_999".to_string()));

        let other_header = client.get_cookie_header_for_url("https://otherdomain.org/index");
        assert_eq!(other_header, None);
    }
}
