use std::time::Duration;

use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde_json::json;

use crate::{
    provider::{Provider, ProviderKind},
    settings::Settings,
    USER_AGENT,
};

const URL: &str = "https://search.bs002.de/api/Avatar/putavatar";

pub struct VrcDB<'s> {
    settings: &'s Settings,
    client:   Client,
}

impl<'s> VrcDB<'s> {
    #[must_use]
    pub fn new(settings: &'s Settings) -> Self {
        Self {
            settings,
            client: Client::default(),
        }
    }
}

#[async_trait]
impl Provider for VrcDB<'_> {
    fn kind(&self) -> ProviderKind {
        ProviderKind::VRCDB
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
            .put(URL)
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
            StatusCode::INTERNAL_SERVER_ERROR => {
                info!("^ Pending in Queue: {kind}");
                debug!("New Avatars can take up to a day to be processed");
                true
            }
            _ => bail!("[{kind}] {status} | {text}"),
        };

        Ok(unique)
    }
}
