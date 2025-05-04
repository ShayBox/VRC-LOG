use std::{collections::HashMap, time::Duration};

use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};

use crate::{
    provider::{Provider, Type},
    USER_AGENT,
};

const URL: &str = "https://search.bs002.de/api/Avatar/putavatar";

pub struct VrcDB {
    client: Client,
    userid: String,
}

impl Default for VrcDB {
    fn default() -> Self {
        Self {
            client: Client::default(),
            userid: crate::discord::get_user_id().unwrap(),
        }
    }
}

#[async_trait]
impl Provider for VrcDB {
    async fn check_avatar_id(&self, _avatar_id: &str) -> Result<bool> {
        bail!("Unsupported/Unused")
    }

    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let name = Type::VRCDB(self);
        let response = self
            .client
            .put(URL)
            .header("User-Agent", USER_AGENT)
            .json(&HashMap::from([
                ("id", avatar_id),
                ("userid", &self.userid),
            ]))
            .timeout(Duration::from_secs(3))
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;
        debug!("[{name}] {status} | {text}");

        let unique = match status {
            StatusCode::OK => false,
            StatusCode::NOT_FOUND => true,
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("[{name}] 429 Rate Limit, Please Wait 1 Minute...");
                tokio::time::sleep(Duration::from_secs(60)).await;
                Box::pin(self.send_avatar_id(avatar_id)).await?
            }
            StatusCode::INTERNAL_SERVER_ERROR => {
                info!("^ Pending in Queue: {name}");
                debug!("New Avatars can take up to a day to be processed");
                true
            }
            _ => bail!("[{name}] {status} | {text}"),
        };

        Ok(unique)
    }
}
