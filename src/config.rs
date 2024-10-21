use anyhow::Context;
use serde::Deserialize;

use crate::backends::{actions::ActionSpec, triggers::TriggerSpec};

pub fn read() -> anyhow::Result<Config> {
    let path = dirs::config_dir()
        .context("failed to get config directory")?
        .join("weird-input")
        .join("config.toml");

    let data = std::fs::read_to_string(&path).context("failed to read config file")?;

    toml::from_str(&data).context("failed to parse config file")
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub triggers: Vec<NamedTriggerSpec>,
    pub mapping: Vec<Mapping>,
}

#[derive(Debug, Deserialize)]
pub struct NamedTriggerSpec {
    pub name: String,
    #[serde(flatten)]
    pub spec: TriggerSpec,
}

#[derive(Debug, Deserialize)]
pub struct Mapping {
    pub trigger: String,
    #[serde(flatten)]
    pub action: ActionSpec,
}
