use std::collections::HashMap;

use anyhow::{bail, Result};
use reqwest::blocking::Client;

use crate::{
    provider::{Provider, Type},
    USER_AGENT,
};

pub struct Doughnut {
    client: Client,
    userid: String,
}

impl Default for Doughnut {
    fn default() -> Self {
        Self {
            client: Client::default(),
            userid: crate::discord::get_user_id().unwrap(),
        }
    }
}

impl Provider for Doughnut {
    fn check_avatar_id(&self, _avatar_id: &str) -> Result<bool> {
        bail!("Cache Only")
    }

    fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let response = self
            .client
            .post("https://avtr1.nekosunevr.co.uk/v1/vrchat/avatars/store/putavatarExternal")
            .header("User-Agent", USER_AGENT)
            .json(&HashMap::from([
                ("id", avatar_id),
                ("userid", &self.userid),
            ]))
            .send()?;

        let status = response.status();
        let text = response.text()?;
        debug!("[{}] {status} | {text}", Type::DOUGHNUT);

        let unique = match status.as_u16() {
            200 => false,
            404 => true,
            _ => {
                error!("[{}] {status} | {text}", Type::DOUGHNUT);
                false
            }
        };

        Ok(unique)
    }
}
