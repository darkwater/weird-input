use std::sync::atomic::{AtomicI32, Ordering};

use anyhow::{anyhow, Context as _};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio_tungstenite::tungstenite;

static ID: AtomicI32 = AtomicI32::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HassWrapper<T> {
    pub id: i32,
    #[serde(flatten)]
    pub data: T,
}

impl HassWrapper<HassRequest> {
    pub fn new(data: HassRequest) -> Self {
        Self {
            id: ID.fetch_add(1, Ordering::Relaxed),
            data,
        }
    }
}

impl HassWrapper<HassResponse> {
    pub fn into_result<T: DeserializeOwned>(self) -> Result<T, anyhow::Error> {
        match self.data {
            HassResponse::Result { success, result } => {
                if success {
                    Ok(serde_json::from_value(result).unwrap())
                } else {
                    Err(anyhow!("Request failed: {:?}", result).context("Request failed"))
                }
            }
            _ => {
                tracing::error!("Not a HassResponse::Result: {:?}", self);
                Err(anyhow!("Unexpected response"))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum HassRequest {
    SubscribeEvents {
        #[serde(skip_serializing_if = "Option::is_none")]
        event_type: Option<String>,
    },
    GetStates,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum HassResponse {
    Result { success: bool, result: serde_json::Value },
    Event { event: HassEvent },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum AuthMessage {
    AuthRequired { ha_version: String },
    Auth { access_token: String },
    AuthOk { ha_version: String },
    AuthInvalid { message: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "event_type", content = "data")]
pub enum HassEvent {
    StateChanged {
        entity_id: String,
        old_state: serde_json::Value,
        new_state: serde_json::Value,
    },
    ZhaEvent {
        device_ieee: String,
        command: String,
        params: serde_json::Value,
        #[serde(flatten)]
        rest: serde_json::Value,
    },
}

impl<T: Serialize> From<HassWrapper<T>> for tungstenite::Message {
    fn from(wrapper: HassWrapper<T>) -> Self {
        tungstenite::Message::Text(serde_json::to_string(&wrapper).unwrap())
    }
}

impl<T: DeserializeOwned> TryFrom<tungstenite::Message> for HassWrapper<T> {
    type Error = serde_json::Error;

    fn try_from(msg: tungstenite::Message) -> Result<Self, Self::Error> {
        serde_json::from_str(msg.to_text().unwrap())
    }
}

impl TryFrom<tungstenite::Message> for HassResponse {
    type Error = serde_json::Error;

    fn try_from(msg: tungstenite::Message) -> Result<Self, Self::Error> {
        HassWrapper::<HassResponse>::try_from(msg).map(|wrapper| wrapper.data)
    }
}

impl From<AuthMessage> for tungstenite::Message {
    fn from(msg: AuthMessage) -> Self {
        tungstenite::Message::Text(serde_json::to_string(&msg).unwrap())
    }
}

impl TryFrom<tungstenite::Message> for AuthMessage {
    type Error = anyhow::Error;

    fn try_from(msg: tungstenite::Message) -> Result<Self, Self::Error> {
        Ok(serde_json::from_str(msg.to_text().context("message was not text/utf8")?)?)
    }
}
