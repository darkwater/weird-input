#![allow(async_fn_in_trait)]

use futures::StreamExt as _;
use tokio::pin;

use self::backends::triggers::{hass_zha::HassZhaBackend, TriggerBackend as _, TriggerSpec};

pub mod config;

pub mod backends {
    pub mod actions;
    pub mod triggers;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let config = config::read()?;

    for trigger in config.triggers {
        match trigger.spec {
            TriggerSpec::HassZha(zha_config) => {
                let res = HassZhaBackend::new(zha_config).await?;
                let stream = res.stream();

                pin!(stream);

                while let Some(event) = stream.next().await {
                    tracing::info!("event: {:?}", event);

                    let trigger = format!("{}.{}.{}", trigger.name, event.device, event.name);

                    for mapping in config.mapping.iter().filter(|m| m.trigger == trigger) {
                        mapping.action.execute().await?;
                    }
                }
            }
        }
    }

    Ok(())
}
