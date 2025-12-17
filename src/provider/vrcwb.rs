use std::{sync::Arc, time::Duration};

use anyhow::{Result, bail};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde_json::json;

use crate::{
    USER_AGENT,
    provider::{Provider, ProviderKind},
    settings::Settings,
};

const URL: &str = "https://avatar.worldbalancer.com/v1/vrchat/avatars/store/putavatarExternal";

pub struct VrcWB {
    settings: Arc<Settings>,
    client: Client,
}

impl VrcWB {
    #[must_use]
    pub fn new(settings: Arc<Settings>) -> Self {
        Self {
            settings,
            client: Client::default(),
        }
    }
}

#[async_trait]
impl Provider for VrcWB {
    fn kind(&self) -> ProviderKind {
        ProviderKind::VRCWB
    }

    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let kind = self.kind();
        let json = json!({
            "id": avatar_id,
            "userid": self.settings.attribution.get_user_id(),
        });

        debug!("[{kind}] Sending {json:#?}");

        let response = self
            .client
            .post(URL)
            .header("User-Agent", USER_AGENT)
            .json(&json)
            .timeout(Duration::from_secs(3))
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!("[{kind}] {status} | {text}");

        let unique = match status {
            StatusCode::OK => false,
            StatusCode::NOT_FOUND => true,
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("[{kind}] 429 Rate Limit, Please Wait 1 Minute...");
                tokio::time::sleep(Duration::from_secs(60)).await;
                Box::pin(self.send_avatar_id(avatar_id)).await?
            }
            _ => bail!("[{kind}] {status} | {text}"),
        };

        Ok(unique)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
