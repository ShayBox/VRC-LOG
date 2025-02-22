use std::collections::HashMap;

use crate::{
    provider::{Provider, Type},
    USER_AGENT,
};
use anyhow::{bail, Result};
use reqwest::blocking::Client;
use reqwest::StatusCode;

const URL: &str = "https://avatarwb.worldbalancer.duia.us/v1/vrchat/avatars/store/putavatarExternal";

pub struct VRCWB {
    client: Client,
    userid: String,
}

impl Default for VRCWB {
    fn default() -> Self {
        Self {
            client: Client::default(),
            userid: crate::discord::get_user_id().unwrap(),
        }
    }
}

impl Provider for VRCWB {
    fn check_avatar_id(&self, _avatar_id: &str) -> Result<bool> {
        bail!("Cache Only")
    }

    fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let response = self
            .client
            .post(URL)
            .header("User-Agent", USER_AGENT)
            .json(&HashMap::from([
                ("id", avatar_id),
                ("userid", &self.userid),
            ]))
            .send()?;

        let status = response.status();
        let text = response.text()?;
        debug!("[{}] {status} | {text}", Type::VRCWB);

        let unique = match status {
            StatusCode::OK => false,
            StatusCode::NOT_FOUND => true,
            _ => bail!("[{}] {status} | {text}", Type::VRCWB),
        };

        Ok(unique)
    }
}
