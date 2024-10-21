use serde::Deserialize;

pub mod shell;
pub mod wayland;

pub trait ActionBackend {
    async fn execute(&self) -> Result<(), anyhow::Error>;
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action")]
#[serde(rename_all = "snake_case")]
pub enum ActionSpec {
    Shell(shell::ShellAction),
}

impl ActionSpec {
    pub async fn execute(&self) -> Result<(), anyhow::Error> {
        match self {
            ActionSpec::Shell(action) => action.execute().await,
        }
    }
}
