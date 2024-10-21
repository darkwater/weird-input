use futures::Stream;
use serde::{de::DeserializeOwned, Deserialize};
use std::fmt::Debug;

use self::hass_zha::HassZhaConfig;

pub mod cec;
pub mod hass_zha;

#[derive(Debug)]
pub struct Event {
    pub device: String,
    pub name: String,
}

pub trait TriggerBackend: Debug {
    type Config: DeserializeOwned;

    async fn new(config: Self::Config) -> Result<Self, anyhow::Error>
    where
        Self: Sized;

    fn stream(self) -> impl Stream<Item = Event>;
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum TriggerSpec {
    HassZha(HassZhaConfig),
}
