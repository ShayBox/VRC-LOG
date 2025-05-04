use std::{collections::HashMap, time::Duration};

use anyhow::{bail, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};

use crate::{
    provider::{Provider, Type},
    USER_AGENT,
};

const URL: &str = "https://avatar.worldbalancer.com/v1/vrchat/avatars/store/putavatarExternal";

pub struct VrcWB {
    client: Client,
    userid: String,
}

impl Default for VrcWB {
    fn default() -> Self {
        Self {
            client: Client::default(),
            userid: crate::discord::get_user_id().unwrap(),
        }
    }
}

#[async_trait]
impl Provider for VrcWB {
    async fn check_avatar_id(&self, _avatar_id: &str) -> Result<bool> {
        bail!("Unsupported/Unused")
    }

    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let name = Type::VRCWB(self);
        let response = self
            .client
            .post(URL)
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
            _ => bail!("[{name}] {status} | {text}"),
        };

        Ok(unique)
    }
}
