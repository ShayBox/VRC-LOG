use std::{collections::HashMap, time::Duration};

use anyhow::{bail, Result};
use reqwest::blocking::Client;

use crate::{
    provider::{Provider, Type},
    USER_AGENT,
};

pub struct AvtrDB {
    client: Client,
    userid: String,
}

impl Default for AvtrDB {
    fn default() -> Self {
        Self {
            client: Client::default(),
            userid: crate::discord::get_user_id().unwrap(),
        }
    }
}

impl Provider for AvtrDB {
    fn check_avatar_id(&self, _avatar_id: &str) -> Result<bool> {
        bail!("Cache Only")
    }

    fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let response = self
            .client
            .put("...")
            .header("User-Agent", USER_AGENT)
            .json(&HashMap::from([
                ("id", avatar_id),
                ("userid", &self.userid),
            ]))
            .send()?;

        let status = response.status();
        debug!("[{}] {status} | {}", Type::AvtrDB, response.text()?);

        let unique = match status.as_u16() {
            200 | 404 => false,
            201 => true,
            429 => {
                warn!("[{}] 429 Rate Limit, Please Wait 1 Minute...", Type::AvtrDB);
                std::thread::sleep(Duration::from_secs(60));
                self.send_avatar_id(avatar_id)?
            }
            _ => {
                error!("[{}] {status}", Type::AvtrDB);
                false
            }
        };

        Ok(unique)
    }
}
