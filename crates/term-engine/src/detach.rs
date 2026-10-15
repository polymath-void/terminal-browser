/// Spawns an on-demand visible OS browser window attached to the current URL for solving CAPTCHAs/2FA
pub async fn detach_headed(current_url: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        tokio::process::Command::new("cmd")
            .args(["/c", "start", "msedge", current_url])
            .spawn()?;
    }
    #[cfg(target_os = "macos")]
    {
        tokio::process::Command::new("open").arg(current_url).spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        tokio::process::Command::new("xdg-open").arg(current_url).spawn()?;
    }
    tracing::info!("Detached headed browser window for URL: {}", current_url);
    Ok(())
}
