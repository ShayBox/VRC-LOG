use std::collections::HashMap;

use anyhow::{bail, Result};
use reqwest::{blocking::Client, StatusCode};

use crate::{
    provider::{Provider, Type::VRCWB},
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

impl Provider for VrcWB {
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
        debug!("[{VRCWB}] {status} | {text}");

        let unique = match status {
            StatusCode::OK => false,
            StatusCode::NOT_FOUND => true,
            _ => bail!("[{VRCWB}] {status} | {text}"),
        };

        Ok(unique)
    }
}
