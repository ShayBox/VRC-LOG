use std::{collections::HashMap, time::Duration};

use anyhow::{bail, Result};
use reqwest::{blocking::Client, StatusCode};

use crate::{
    provider::{Provider, Type::VRCDS},
    USER_AGENT,
};

const URL: &str = "https://avtr.nekosunevr.co.uk/v1/vrchat/avatars/store/putavatarExternal";

pub struct VrcDS {
    client: Client,
    userid: String,
}

impl Default for VrcDS {
    fn default() -> Self {
        Self {
            client: Client::default(),
            userid: crate::discord::get_user_id().unwrap(),
        }
    }
}

impl Provider for VrcDS {
    fn check_avatar_id(&self, _avatar_id: &str) -> Result<bool> {
        bail!("Cache Only")
    }

    fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let response = match self
            .client
            .post(URL)
            .header("User-Agent", USER_AGENT)
            .json(&HashMap::from([
                ("id", avatar_id),
                ("userid", &self.userid),
            ]))
            .timeout(Duration::from_secs(1)) // TODO: Remove when API is more stable.
            .send()
        {
            Ok(response) => response,
            Err(error) => {
                // Ignore for cache purposes, it goes offline too often.
                warn!("[{VRCDS}] {error}");
                return Ok(false);
            }
        };

        let status = response.status();
        let text = response.text()?;
        debug!("[{VRCDS}] {status} | {text}");

        let unique = match status {
            StatusCode::OK => false,
            StatusCode::NOT_FOUND => true,
            _ => bail!("[{VRCDS}] {status} | {text}"),
        };

        Ok(unique)
    }
}
