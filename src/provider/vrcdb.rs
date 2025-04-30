use std::{collections::HashMap, time::Duration};

use anyhow::{bail, Result};
use reqwest::{blocking::Client, StatusCode};

use crate::{
    provider::{Provider, Type::VRCDB},
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

impl Provider for VrcDB {
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
        debug!("[{VRCDB}] {status} | {text}");

        let unique = match status {
            StatusCode::OK => false,
            StatusCode::NOT_FOUND => true,
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("[{VRCDB}] 429 Rate Limit, Please Wait 1 Minute...");
                std::thread::sleep(Duration::from_secs(60));
                self.send_avatar_id(avatar_id)?
            }
            StatusCode::INTERNAL_SERVER_ERROR => {
                info!("^ Pending in Queue: {VRCDB}");
                debug!("New Avatars can take up to a day to be processed");
                true
            }
            _ => bail!("[{VRCDB}] {status} | {text}"),
        };

        Ok(unique)
    }
}
