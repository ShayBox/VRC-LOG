use std::collections::HashMap;

use anyhow::{bail, Result};
use reqwest::blocking::Client;

use crate::{
    discord::{User, DEVELOPER_ID, USER},
    provider::{Provider, Type},
};

const USER_AGENT: &str = concat!(
    "VRC-LOG/",
    env!("CARGO_PKG_VERSION"),
    " shaybox@shaybox.com"
);

pub struct VRCDB {
    client: Client,
    userid: String,
}

impl VRCDB {
    #[must_use]
    pub const fn new(client: Client, userid: String) -> Self {
        Self { client, userid }
    }
}

impl Default for VRCDB {
    fn default() -> Self {
        let client = Client::default();
        let userid = USER.clone().map_or_else(
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
                println!("{} Authenticated as {nick} ({name})", Type::VRCDB);

                id
            },
        );

        Self::new(client, userid)
    }
}

impl Provider for VRCDB {
    fn check_avatar_id(&self, _avatar_id: &str) -> Result<bool> {
        bail!("Unsupported")
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

        Ok(response.status() == 201)
    }
}
