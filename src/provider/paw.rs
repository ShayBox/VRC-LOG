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
const AVATAR_URL: &str = "https://paw-api.amelia.fun/avatar";

pub struct Paw<'a> {
    settings: &'a Settings,
    client:   Client,
}

impl<'a> Paw<'a> {
    #[must_use]
    pub fn new(settings: &'a Settings) -> Self {
        Self {
            settings,
            client: Client::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct PawResponse {
    success: bool,
    code:    u16,
    result:  Option<Value>,
    avatar:  Option<Value>,
}

#[async_trait]
impl Provider for Paw<'_> {
    fn kind(&self) -> ProviderKind {
        ProviderKind::PAW
    }

    async fn check_avatar_id(&self, _avatar_id: &str) -> Result<bool> {
        let kind = self.kind();
        let response = self
            .client
            .get(AVATAR_URL)
            .header("User-Agent", USER_AGENT)
            .query(&[("avatarId", _avatar_id)])
            .timeout(Duration::from_secs(3))
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!("[{kind}] {status} | {text}");

        if status != StatusCode::OK {
            bail!("[{kind}] Failed to check avatar: {status} | {text}");
        }

        let data = serde_json::from_str::<PawResponse>(&text)?;

        Ok(data.success && data.code == 200 && data.result.is_some())
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
