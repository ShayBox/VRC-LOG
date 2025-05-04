use std::{time::Duration, vec};

use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};

use crate::{
    provider::{Provider, Type},
    USER_AGENT,
};

const URL: &str = "https://api.avtrdb.com/v2/";

pub struct AvtrDB {
    attribution: Option<String>,
    client:      Client,
    url:         Url,
}

impl AvtrDB {
    #[must_use]
    pub fn new(attribution: Option<String>, url: Url) -> Self {
        Self {
            attribution,
            url,
            ..Default::default()
        }
    }
}

impl Default for AvtrDB {
    fn default() -> Self {
        // Only alphanumeric strings up to 30 characters or nothing are allowed.
        // if these conditions are not met, the given avatars will not be ingested.
        let attribution = std::env::var("AVTRDB_ATTRIBUTION")
            .ok()
            .or_else(crate::discord::get_user_id);

        Self {
            attribution,
            url: Url::parse(URL).expect("Failed to parse URL"),
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent(USER_AGENT)
                .build()
                .unwrap(),
        }
    }
}

#[derive(Debug, Serialize)]
struct AvtrDBRequest {
    avatar_ids:  Vec<String>,
    attribution: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AvtrDBResponse {
    valid_avatar_ids: u64,
}

#[derive(Debug, Deserialize)]
struct AvtrDBSearchResponse {
    avatars: Vec<serde_json::Value>,
}

#[async_trait]
impl Provider for AvtrDB {
    async fn check_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let name = Type::AVTRDB(self);
        let mut url = self.url.join("avatar/search")?;
        url.set_query(Some(format!("query={avatar_id}").as_str()));

        let response = self.client.get(url).send().await?;
        let status = response.status();
        let text = response.text().await?;
        debug!("[{name}] {status} | {text}");

        if status != StatusCode::OK {
            bail!("[{name}] Failed to check avatar: {status} | {text}");
        }

        let data = serde_json::from_str::<AvtrDBSearchResponse>(&text)?;

        Ok(data.avatars.len() == 1)
    }

    // The API supports batching, but this interface does not
    // FIXME: adapt ProviderTrait to support batching
    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let name = Type::AVTRDB(self);
        let request = AvtrDBRequest {
            avatar_ids:  vec![avatar_id.to_string()],
            attribution: self.attribution.clone(),
        };

        debug!("[{name}] Sending {:#?}", serde_json::to_string(&request)?);

        let response = self
            .client
            .post(self.url.join("avatar/ingest")?)
            .json(&request)
            .timeout(Duration::from_secs(3))
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        let data = serde_json::from_str::<AvtrDBResponse>(&text)?;
        debug!("[{name}] {status} | {text}");

        let unique = match status {
            StatusCode::OK => data.valid_avatar_ids == 1,
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("[{name}] 429 Rate Limit, trying again in 10 seconds");
                tokio::time::sleep(Duration::from_secs(10)).await;
                Box::pin(self.send_avatar_id(avatar_id)).await?
            }
            _ => bail!("[{name}] Unknown Error: {status} | {text}"),
        };

        Ok(unique)
    }
}
