use std::sync::Arc;

use self::api::{AuthMessage, HassEvent, HassRequest, HassResponse, HassWrapper};
use anyhow::Context;
use futures::{SinkExt as _, Stream, StreamExt as _};
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

use super::{Event, TriggerBackend};

mod api;

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug)]
pub struct HassZhaBackend {
    config: Arc<HassZhaConfig>,
    stream: WsStream,
}

#[derive(Debug, Deserialize)]
pub struct HassZhaConfig {
    pub base_url: String,
    pub token: String,
    pub devices: Vec<DeviceSpec>,
}

#[derive(Debug, Deserialize)]
pub struct DeviceSpec {
    name: String,
    ieee: String,
    buttons: Vec<ButtonSpec>,
}

#[derive(Debug, Deserialize)]
pub struct ButtonSpec {
    name: String,
    command: String,
    #[serde(default = "empty_object")]
    params: serde_json::Value,
}

impl TriggerBackend for HassZhaBackend {
    type Config = HassZhaConfig;

    async fn new(config: Self::Config) -> Result<Self, anyhow::Error> {
        let (mut stream, _) = connect_async(format!(
            "{}/api/websocket",
            config
                .base_url
                .strip_suffix('/')
                .unwrap_or(&config.base_url)
        ))
        .await
        .context("failed to connect")?;

        hass_auth(&mut stream, &config.token).await?;

        hass_subscribe(&mut stream).await?;

        let config = Arc::new(config);

        Ok(HassZhaBackend { config, stream })
    }

    fn stream(self) -> impl Stream<Item = Event> {
        self.stream.filter_map(move |ev| {
            let config = self.config.clone();

            async move {
                let msg = match ev {
                    Ok(msg) => msg,
                    Err(e) => {
                        tracing::error!("failed to receive message: {:?}", e);
                        return None;
                    }
                };

                let Ok(HassWrapper {
                    id: _,
                    data:
                        HassResponse::Event {
                            event:
                                HassEvent::ZhaEvent {
                                    device_ieee,
                                    command,
                                    params,
                                    rest: _,
                                },
                        },
                }) = HassWrapper::<HassResponse>::try_from(msg)
                else {
                    return None;
                };

                let device = config.devices.iter().find(|d| d.ieee == device_ieee)?;

                let button = device
                    .buttons
                    .iter()
                    .find(|b| b.command == command && b.params == params)?;

                Some(Event {
                    device: device.name.clone(),
                    name: button.name.clone(),
                })
            }
        })
    }
}

async fn hass_subscribe(stream: &mut WsStream) -> Result<(), anyhow::Error> {
    stream
        .send(
            HassWrapper::new(HassRequest::SubscribeEvents {
                event_type: Some("zha_event".to_owned()),
            })
            .into(),
        )
        .await
        .context("failed to send message")?;

    let subscribe_ok: HassWrapper<HassResponse> = stream
        .next()
        .await
        .context("unexpected end of stream")?
        .context("failed to read message")?
        .try_into()?;

    subscribe_ok
        .into_result::<()>()
        .context("failed to subscribe to events")?;

    Ok(())
}

async fn hass_auth(stream: &mut WsStream, token: &str) -> Result<(), anyhow::Error> {
    let auth_req = stream
        .next()
        .await
        .context("unexpected end of stream")?
        .context("failed to read message")?
        .try_into()?;

    if let AuthMessage::AuthRequired { ha_version } = auth_req {
        tracing::info!("Authenticating with Home Assistant version {}", ha_version);
    } else {
        tracing::warn!("Unexpected message: {:?}", auth_req);
    }

    stream
        .send((AuthMessage::Auth { access_token: token.to_owned() }).into())
        .await
        .context("failed to send message")?;

    let auth_ok = stream
        .next()
        .await
        .context("unexpected end of stream")?
        .context("failed to read message")?
        .try_into()?;

    match auth_ok {
        AuthMessage::AuthOk { ha_version } => {
            tracing::info!("Authenticated with Home Assistant version {}", ha_version);
        }
        AuthMessage::AuthInvalid { message } => {
            tracing::error!("Failed to authenticate: {}", message);
        }
        _ => {
            tracing::warn!("Unexpected message: {:?}", auth_ok);
        }
    }

    Ok(())
}

fn empty_object() -> serde_json::Value {
    serde_json::Value::Object(Default::default())
}
