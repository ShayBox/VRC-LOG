use std::{time::Duration, vec};

use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    provider::{Provider, ProviderKind},
    settings::Settings,
    USER_AGENT,
};

const URL: &str = "https://api.avtrdb.com/v2/";

pub struct AvtrDB<'a> {
    settings: &'a Settings,
    client:   Client,
}

impl<'a> AvtrDB<'a> {
    #[must_use]
    pub fn new(settings: &'a Settings) -> Self {
        Self {
            settings,
            client: Client::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct IngestResponse {
    valid_avatar_ids: u64,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    avatars: Vec<Value>,
}

#[async_trait]
impl Provider for AvtrDB<'_> {
    fn kind(&self) -> ProviderKind {
        ProviderKind::AVTRDB
    }

    async fn check_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let kind = self.kind();
        let mut url = Url::parse(URL)?.join("avatar/search")?;
        url.set_query(Some(format!("query={avatar_id}").as_str()));

        let response = self
            .client
            .get(url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!("[{kind}] {status} | {text}");

        if status != StatusCode::OK {
            bail!("[{kind}] Failed to check avatar: {status} | {text}");
        }

        let data = serde_json::from_str::<SearchResponse>(&text)?;

        Ok(data.avatars.len() == 1)
    }

    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let kind = self.kind();
        let json = json!({
            "avatar_ids":  vec![avatar_id.to_string()],
            "attribution": self.settings.attribution.get_user_id(),
        });

        debug!("[{kind}] Sending {json:#?}");

        let response = self
            .client
            .post(Url::parse(URL)?.join("avatar/ingest")?)
            .header("User-Agent", USER_AGENT)
            .json(&json)
            .timeout(Duration::from_secs(3))
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        let data = serde_json::from_str::<IngestResponse>(&text)?;
        debug!("[{kind}] {status} | {text}");

        let unique = match status {
            StatusCode::OK => data.valid_avatar_ids == 1,
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("[{kind}] 429 Rate Limit, trying again in 10 seconds");
                tokio::time::sleep(Duration::from_secs(10)).await;
                Box::pin(self.send_avatar_id(avatar_id)).await?
            }
            _ => bail!("[{kind}] Unknown Error: {status} | {text}"),
        };

        Ok(unique)
    }
}
