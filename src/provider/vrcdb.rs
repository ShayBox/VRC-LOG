use std::{collections::HashMap, time::Duration};

use anyhow::{bail, Result};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use crate::{
    provider::{Provider, Type},
    USER_AGENT,
};

const URL: &str = "https://search.bs002.de/api/Avatar/putavatar";

pub struct VRCDB {
    client: Client,
    userid: String,
}

impl Default for VRCDB {
    fn default() -> Self {
        Self {
            client: Client::default(),
            userid: crate::discord::get_user_id().unwrap(),
        }
    }
}

impl Provider for VRCDB {
    fn check_avatar_id(&self, _avatar_id: &str) -> Result<bool> {
        bail!("Cache Only")
    }

    fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let response = self
            .client
            .put(URL)
            .header("User-Agent", USER_AGENT)
            .json(&HashMap::from([
                ("id", avatar_id),
                ("userid", &self.userid),
            ]))
            .send()?;

        let status = response.status();
        let text = response.text()?;
        debug!("[{}] {status} | {text}", Type::VRCDB);

        let unique = match status {
            StatusCode::OK => false,
            StatusCode::NOT_FOUND => true,
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("[{}] 429 Rate Limit, Please Wait 1 Minute...", Type::VRCDB);
                std::thread::sleep(Duration::from_secs(60));
                self.send_avatar_id(avatar_id)?
            }
            StatusCode::INTERNAL_SERVER_ERROR => {
                info!("^ Pending in Queue: {}", Type::VRCDB);
                debug!("New Avatars can take up to a day to be processed");
                true
            }
            _ => bail!("[{}] {status} | {text}", Type::VRCDB),
        };

        Ok(unique)
    }
}
