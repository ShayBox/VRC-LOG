use std::{collections::HashMap, time::Duration};

use anyhow::{bail, Result};
use reqwest::blocking::Client;

use crate::{
    provider::{Provider, Type},
    USER_AGENT,
};

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
            .put("https://search.bs002.de/api/Avatar/putavatar")
            .header("User-Agent", USER_AGENT)
            .json(&HashMap::from([
                ("id", avatar_id),
                ("userid", &self.userid),
            ]))
            .send()?;

        let status = response.status();
        let text = response.text()?;
        debug!("[{}] {status} | {text}", Type::VRCDB);

        let unique = match status.as_u16() {
            200 => false,
            404 => true,
            429 => {
                warn!("[{}] 429 Rate Limit, Please Wait 1 Minute...", Type::VRCDB);
                std::thread::sleep(Duration::from_secs(60));
                self.send_avatar_id(avatar_id)?
            }
            500 => {
                info!("^ Pending in Queue: {}", Type::VRCDB);
                debug!("New Avatars can take up to a day to be processed");
                true
            }
            _ => bail!("[{}] {status} | {text}", Type::VRCDB),
        };

        Ok(unique)
    }
}
