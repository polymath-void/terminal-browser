use std::time::Duration;
use tokio::process::{Child, Command};

pub struct ProcessSupervisor {
    child: Option<Child>,
    restart_attempts: u32,
    max_memory_mb: u64,
    navigations_count: u32,
    max_navigations_before_recycle: u32,
}

impl ProcessSupervisor {
    pub fn new(max_memory_mb: u64, max_navs: u32) -> Self {
        Self {
            child: None,
            restart_attempts: 0,
            max_memory_mb,
            navigations_count: 0,
            max_navigations_before_recycle: max_navs,
        }
    }

    pub fn max_memory_mb(&self) -> u64 {
        self.max_memory_mb
    }

    pub async fn spawn_engine(&mut self, binary_path: &str, args: &[&str]) -> anyhow::Result<()> {
        let child = Command::new(binary_path)
            .args(args)
            .kill_on_drop(true)
            .spawn()?;
        self.child = Some(child);
        self.restart_attempts = 0;
        tracing::info!("Browser subprocess spawned successfully");
        Ok(())
    }

    pub async fn check_and_recycle_if_needed(&mut self) -> anyhow::Result<bool> {
        self.navigations_count += 1;
        if self.navigations_count >= self.max_navigations_before_recycle {
            tracing::warn!(
                "Recycling browser process after {} navigations to mitigate memory leaks",
                self.navigations_count
            );
            self.recycle().await?;
            return Ok(true);
        }
        Ok(false)
    }

    pub async fn recycle(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
        }
        self.navigations_count = 0;
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    pub fn is_alive(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            matches!(child.try_wait(), Ok(None))
        } else {
            false
        }
    }
}
