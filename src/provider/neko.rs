use std::collections::HashMap;

use anyhow::{bail, Result};
use reqwest::blocking::Client;

use crate::{
    provider::{Provider, Type},
    USER_AGENT,
};

pub struct Neko {
    client: Client,
    userid: String,
}

impl Default for Neko {
    fn default() -> Self {
        Self {
            client: Client::default(),
            userid: crate::discord::get_user_id().unwrap(),
        }
    }
}

impl Provider for Neko {
    fn check_avatar_id(&self, _avatar_id: &str) -> Result<bool> {
        bail!("Cache Only")
    }

    fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let response = self
            .client
            .post("https://avtr.nekosunevr.co.uk/v1/vrchat/avatars/store/putavatarExternal")
            .header("User-Agent", USER_AGENT)
            .json(&HashMap::from([
                ("id", avatar_id),
                ("userid", &self.userid),
            ]))
            .send()?;

        let status = response.status();
        let text = response.text()?;
        debug!("[{}] {status} | {text}", Type::NEKO);

        let unique = match status.as_u16() {
            200 => false,
            404 => true,
            _ => bail!("[{}] {status} | {text}", Type::NEKO),
        };

        Ok(unique)
    }
}
