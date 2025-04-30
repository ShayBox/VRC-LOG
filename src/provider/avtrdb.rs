use std::{time::Duration, vec};

use anyhow::{bail, Result};
use reqwest::{blocking::Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use crate::{
    provider::{Provider, Type},
    USER_AGENT,
};

const BASE_URL: &str = "https://api.avtrdb.com/v2/";

pub struct AvtrDB {
    client:      Client,
    attribution: Option<String>,
    base_url:    Url,
}

impl AvtrDB {
    #[must_use]
    pub fn new(attribution: Option<String>, base_url: Url) -> Self {
        Self {
            attribution,
            base_url,
            ..Default::default()
        }
    }
}

impl Default for AvtrDB {
    fn default() -> Self {
        // Only alphanumeric strings up to 30 characters or nothing are allowed
        // if these conditions are not met, the given avatars will not be ingested
        let attribution = std::env::var("AVTRDB_ATTRIBUTION")
            .ok()
            .or_else(crate::discord::get_user_id);

        Self {
            attribution,
            base_url: Url::parse(BASE_URL).expect("Failed to parse BASE_URL"),
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

impl Provider for AvtrDB {
    fn check_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let mut url = self.base_url.join("avatar/search")?;
        url.set_query(Some(format!("query={avatar_id}").as_str()));

        let response = self.client.get(url).send()?;
        let status = response.status();
        let text = response.text()?;
        debug!("[{}] {status} | {text}", Type::AVTRDB);

        if status != StatusCode::OK {
            bail!(
                "[{}] Failed to check avatar: {status} | {text}",
                Type::AVTRDB
            );
        }

        let data = serde_json::from_str::<AvtrDBSearchResponse>(&text)?;

        Ok(data.avatars.len() == 1)
    }

    // The API supports batching, but this interface does not
    // FIXME: adapt ProviderTrait to support batching
    fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let request = AvtrDBRequest {
            avatar_ids:  vec![avatar_id.to_string()],
            attribution: self.attribution.clone(),
        };

        debug!(
            "[{}] Sending {:#?}",
            Type::AVTRDB,
            serde_json::to_string(&request)?
        );

        let response = self
            .client
            .post(self.base_url.join("avatar/ingest")?)
            .json(&request)
            .send()?;

        let status = response.status();
        let text = response.text()?;
        let data = serde_json::from_str::<AvtrDBResponse>(&text)?;
        debug!("[{}] {status} | {text}", Type::AVTRDB);

        let unique = match status {
            StatusCode::OK => data.valid_avatar_ids == 1,
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("[{}] 429 Rate Limit, trying again in 10 seconds", Type::AVTRDB);
                std::thread::sleep(Duration::from_secs(10));
                self.send_avatar_id(avatar_id)?
            }
            _ => bail!("[{}] Unknown Error: {status} | {text}", Type::AVTRDB),
        };
        
        Ok(unique)
    }
}
