/// Spawns an on-demand visible OS browser window attached to the current URL for solving CAPTCHAs/2FA.
/// Dispatches strictly based on the runtime environment:
/// - Termux (Android): ONLY `termux-open-url` (or fallback `termux-open`)
/// - Windows PC: `cmd /c start <url>`
/// - Desktop Linux: `xdg-open <url>`
/// - macOS: `open <url>`
pub async fn detach_headed(current_url: &str) -> anyhow::Result<()> {
    let is_termux = std::env::var("TERMUX_VERSION").is_ok()
        || std::env::var("PREFIX").map(|p| p.contains("com.termux")).unwrap_or(false)
        || cfg!(target_os = "android");

    if is_termux {
        // Strict Termux environment handling: ONLY termux-open-url (or fallback termux-open)
        tracing::info!("Detected Termux environment. Launching via termux-open-url: {}", current_url);
        let res = tokio::process::Command::new("termux-open-url")
            .arg(current_url)
            .spawn();
        match res {
            Ok(_) => {}
            Err(e) => {
                tracing::warn!("termux-open-url failed ({}), trying termux-open...", e);
                tokio::process::Command::new("termux-open")
                    .arg(current_url)
                    .spawn()?;
            }
        }
    } else {
        #[cfg(target_os = "windows")]
        {
            tracing::info!("Detected Windows PC environment. Launching default browser: {}", current_url);
            tokio::process::Command::new("cmd")
                .args(["/c", "start", "", current_url])
                .spawn()?;
        }
        #[cfg(target_os = "macos")]
        {
            tracing::info!("Detected macOS environment. Launching via open: {}", current_url);
            tokio::process::Command::new("open").arg(current_url).spawn()?;
        }
        #[cfg(all(target_os = "linux", not(target_os = "android")))]
        {
            tracing::info!("Detected Linux PC/Desktop environment. Launching via xdg-open: {}", current_url);
            tokio::process::Command::new("xdg-open").arg(current_url).spawn()?;
        }
    }

    tracing::info!("Detached headed browser window for URL: {}", current_url);
    Ok(())
}

