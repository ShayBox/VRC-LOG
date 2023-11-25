use std::collections::HashMap;

use anyhow::bail;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::{
    discord::{User, DEVELOPER_ID, USER},
    provider::Provider,
};

const USER_AGENT: &str = concat!(
    "VRC-LOG/",
    env!("CARGO_PKG_VERSION"),
    " shaybox@shaybox.com"
);

#[derive(Deserialize, Serialize)]
pub struct Response {
    status: Status,
}

#[derive(Deserialize, Serialize)]
pub struct Status {
    message: String,
    status:  i64,
}

pub struct Ravenwood {
    client:  Client,
    user_id: String,
}

impl Ravenwood {
    #[must_use]
    pub const fn new(client: Client, user_id: String) -> Self {
        Self { client, user_id }
    }
}

impl Default for Ravenwood {
    fn default() -> Self {
        let client = Client::default();
        let user_id = USER.clone().map_or_else(
            || {
                eprintln!("Error: Discord RPC Connection Failed\n");
                eprintln!("This may be due to one of the following reasons:");
                eprintln!("1. Discord is not running on your system.");
                eprintln!("2. VRC-LOG was restarted too quickly.\n");
                eprintln!("The User ID will default to the developer: ShayBox");

                DEVELOPER_ID.to_owned()
            },
            |user| {
                let User { id, name, nick } = user;
                println!("[Ravenwood] Authenticated as {nick} ({name})");

                id
            },
        );

        Self::new(client, user_id)
    }
}

impl Provider for Ravenwood {
    fn check_avatar_id(&self, _avatar_id: &str) -> anyhow::Result<bool> {
        bail!("Unsupported")
    }

    fn send_avatar_id(&self, avatar_id: &str) -> anyhow::Result<bool> {
        let response = self
            .client
            .put("https://api.ravenwood.dev/avatars/putavatar")
            .header("User-Agent", USER_AGENT)
            .json(&HashMap::from([
                ("id", avatar_id),
                ("userid", &self.user_id),
            ]))
            .send()?
            .json::<Response>()?;

        Ok(response.status.status == 201)
    }
}
