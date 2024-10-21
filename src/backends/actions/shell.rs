use anyhow::Context as _;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ShellAction {
    pub command: String,
}

impl super::ActionBackend for ShellAction {
    async fn execute(&self) -> Result<(), anyhow::Error> {
        let status = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .status()
            .await
            .context("failed to execute command")?;

        if !status.success() {
            anyhow::bail!("command failed with exit code: {}", status);
        }

        Ok(())
    }
}
