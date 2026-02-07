use std::time::Duration;

use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    provider::{Provider, ProviderKind},
    settings::Settings,
    USER_AGENT,
};

const URL: &str = "https://paw-api.amelia.fun/update";

pub struct Paw {
    client: Client,
}

impl Paw {
    #[must_use]
    pub fn new(_settings: &Settings) -> Self {
        Self {
            client: Client::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PawResponse {
    success: bool,
    code:    u16,
    result:  Option<Value>,
    avatar:  Option<Value>,
}

#[async_trait]
impl Provider for Paw {
    fn kind(&self) -> ProviderKind {
        ProviderKind::PAW
    }

    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let kind = self.kind();
        let response = self
            .client
            .post(URL)
            .header("User-Agent", USER_AGENT)
            .query(&[("avatarId", avatar_id)])
            .timeout(Duration::from_secs(3))
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!("[{kind}] {status} | {text}");

        let unique = match status {
            StatusCode::OK => {
                let data = serde_json::from_str::<PawResponse>(&text)?;
                #[allow(clippy::nonminimal_bool)]
                !matches!(data.avatar.as_ref(), Some(avatar) if !avatar.is_null() && !(avatar.is_array() && avatar.as_array().unwrap().is_empty()))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("[{kind}] 429 Rate Limit, Please Wait 10 seconds...");
                tokio::time::sleep(Duration::from_secs(10)).await;
                Box::pin(self.send_avatar_id(avatar_id)).await?
            }
            _ => bail!("[{kind}] {status} | {text}"),
        };

        Ok(unique)
    }
}
