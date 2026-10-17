use futures::StreamExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use term_storage::{CookieVault, DatabaseManager, StoredCookie};

use chromiumoxide::browser::{Browser, BrowserConfig};

/// Auto-detects installed Chromium / Chrome browser binary across platforms
pub fn find_browser_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CHROME_BIN") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(path) = std::env::var("CHROME_PATH") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    // Android Termux candidates
    let termux_candidates = [
        "/data/data/com.termux/files/usr/bin/chromium",
        "/data/data/com.termux/files/usr/bin/chromium-browser",
    ];
    for c in &termux_candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }

    // Standard Linux candidates
    let linux_candidates = [
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/snap/bin/chromium",
    ];
    for c in &linux_candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }

    // Windows candidates
    let windows_candidates = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    ];
    for c in &windows_candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }

    // macOS candidates
    let mac_candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
    ];
    for c in &mac_candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }

    None
}

/// Checks whether a local browser binary is available for Headless CDP execution
pub fn is_cdp_available() -> bool {
    find_browser_binary().is_some()
}

/// Inspects HTML content to detect if client-side JavaScript execution is mandatory
pub fn is_js_required(html: &str) -> bool {
    let lower = html.to_lowercase();

    // 1. Explicit JS required warnings
    if lower.contains("enable javascript")
        || lower.contains("please enable js")
        || lower.contains("javascript is disabled")
        || lower.contains("requires javascript to work")
        || lower.contains("javascript is required")
        || lower.contains("you need to enable javascript")
    {
        return true;
    }

    // 2. Anti-bot / WAF challenge pages
    if lower.contains("cf-browser-verification")
        || lower.contains("just a moment...")
        || lower.contains("turnstile")
        || lower.contains("funcaptcha")
        || lower.contains("arkoselabs")
    {
        return true;
    }

    // 3. Empty Single-Page Application (SPA) containers with script bundles
    let has_spa_container = lower.contains("<div id=\"root\"></div>")
        || lower.contains("<div id=\"app\"></div>")
        || lower.contains("<div id=\"__next\"></div>")
        || lower.contains("<div id=\"signup-container\"></div>");

    if has_spa_container && lower.contains("<script") && html.len() < 5000 {
        return true;
    }

    false
}

#[derive(Debug, Clone)]
pub struct RenderedPage {
    pub url: String,
    pub title: Option<String>,
    pub html: String,
    pub cookies: Vec<StoredCookie>,
}

#[derive(Clone)]
pub struct CdpEngine {
    browser_binary: PathBuf,
    vault: Option<Arc<Mutex<DatabaseManager>>>,
}

impl std::fmt::Debug for CdpEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CdpEngine")
            .field("browser_binary", &self.browser_binary)
            .field("has_vault", &self.vault.is_some())
            .finish()
    }
}

impl CdpEngine {
    /// Creates a new `CdpEngine` by locating the installed Chromium binary and default SQLite vault
    pub fn new() -> anyhow::Result<Self> {
        let binary = find_browser_binary().ok_or_else(|| {
            anyhow::anyhow!(
                "No Chromium or Chrome binary found. Please install Chromium (e.g. 'pkg install chromium' on Termux) or set CHROME_BIN."
            )
        })?;

        let vault = DatabaseManager::open_default()
            .ok()
            .map(|db| Arc::new(Mutex::new(db)));

        Ok(Self {
            browser_binary: binary,
            vault,
        })
    }

    pub fn with_binary(binary: PathBuf, vault: Option<Arc<Mutex<DatabaseManager>>>) -> Self {
        Self {
            browser_binary: binary,
            vault,
        }
    }

    /// Launches a headless browser session, executes JavaScript, waits for hydration, and extracts the hydrated DOM
    pub async fn render_page(&self, target_url: &str, timeout_secs: u64) -> anyhow::Result<RenderedPage> {
        let mut builder = BrowserConfig::builder();
        builder = builder
            .chrome_executable(&self.browser_binary)
            .no_sandbox()
            .arg("--headless=new")
            .arg("--disable-gpu")
            .arg("--blink-settings=imagesEnabled=false")
            .arg("--disable-remote-fonts")
            .arg("--disable-background-networking")
            .arg("--disable-sync")
            .arg("--disable-dev-shm-usage");

        let config = builder
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build BrowserConfig: {}", e))?;

        let (mut browser, mut handler) = Browser::launch(config).await?;

        // Spawn background handler task
        let handler_task = tokio::task::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        let page_result = async {
            let page = browser.new_page(target_url).await?;

            // Wait for navigation / DOM hydration with timeout
            tokio::time::sleep(Duration::from_millis(1500)).await;

            let title = page.get_title().await.ok().flatten();
            let html = page.content().await.unwrap_or_default();

            // Extract session cookies if vault is present
            let mut captured_cookies = Vec::new();
            if let Ok(cookies) = page.get_cookies().await {
                for c in cookies {
                    let stored = StoredCookie {
                        domain: c.domain.clone(),
                        name: c.name.clone(),
                        value: c.value.clone(),
                        path: c.path.clone(),
                        expires_at: if c.expires > 0.0 { Some(c.expires as i64) } else { None },
                        is_secure: c.secure,
                        is_httponly: c.http_only,
                        same_site: format!("{:?}", c.same_site),
                    };

                    if let Some(vault_arc) = &self.vault {
                        if let Ok(vault) = vault_arc.lock() {
                            let _ = CookieVault::upsert(vault.conn(), &stored);
                        }
                    }
                    captured_cookies.push(stored);
                }
            }

            Ok::<RenderedPage, anyhow::Error>(RenderedPage {
                url: target_url.to_string(),
                title,
                html,
                cookies: captured_cookies,
            })
        };

        let result = tokio::time::timeout(Duration::from_secs(timeout_secs), page_result).await;

        // Cleanup browser
        let _ = browser.close().await;
        handler_task.abort();

        match result {
            Ok(inner_res) => inner_res,
            Err(_) => anyhow::bail!("Headless CDP page render timed out after {}s", timeout_secs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_js_required_detection() {
        assert!(is_js_required("<html><body><noscript>Please enable JS and disable any ad blocker</noscript></body></html>"));
        assert!(is_js_required("<div id=\"root\"></div><script src=\"app.js\"></script>"));
        assert!(is_js_required("<div id=\"app\"></div><script src=\"main.js\"></script>"));
        assert!(is_js_required("<h1>Just a moment...</h1><p>Checking your browser before accessing.</p>"));
        assert!(is_js_required("<p>Turnstile verification in progress</p>"));

        // Static pages should not require JS
        assert!(!is_js_required("<html><body><h1>Welcome</h1><p>This is a purely static Wikipedia article with rich paragraphs.</p></body></html>"));
        assert!(!is_js_required("# Markdown Document\n\nPure markdown summary text."));
    }

    #[test]
    fn test_find_browser_binary_safe() {
        // Should not panic even if no browser is installed
        let _ = find_browser_binary();
    }
}
