use std::{collections::HashMap, time::Duration};

use anyhow::{bail, Result};
use reqwest::blocking::Client;

use crate::{
    discord::{DEVELOPER_ID, USER},
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
        // TODO: Print VRCDB Statistics
        // Waiting on VRCDB Leaderboard

        Self { client, userid }
    }

    fn default() -> String {
        warn!("Error: Discord RPC Connection Failed\n");
        warn!("This may be due to one of the following reasons:");
        warn!("1. Discord is not running on your system.");
        warn!("2. VRC-LOG was restarted too quickly.\n");
        warn!("The User ID will default to the developer: ShayBox");

        std::env::var("DISCORD").unwrap_or_else(|_| DEVELOPER_ID.to_owned())
    }
}

impl Default for VRCDB {
    fn default() -> Self {
        let client = Client::default();
        let userid = USER.clone().map_or_else(Self::default, |user| {
            let userid = user.id.unwrap_or_else(Self::default);
            if userid == "1045800378228281345" {
                warn!("Vesktop & arRPC doesn't support fetching user info");
                warn!("You can supply the 'DISCORD' env variable manually");
                warn!("The User ID will default to the developer: ShayBox");

                std::env::var("DISCORD").unwrap_or_else(|_| DEVELOPER_ID.to_owned())
            } else {
                if let Some(username) = user.username {
                    info!("[{}] Authenticated as {username}", Type::VRCDB);
                }

                userid
            }
        });

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

        let status = response.status();
        debug!("[{}] {status} | {}", Type::VRCDB, response.text()?);

        let unique = match status.as_u16() {
            200 | 404 => false,
            201 => true,
            429 => {
                warn!("[{}] 429 Rate Limit, Please Wait 1 Minute...", Type::VRCDB);
                std::thread::sleep(Duration::from_secs(60));
                self.send_avatar_id(avatar_id)?
            }
            _ => {
                error!("[{}] {status}", Type::VRCDB);
                false
            }
        };

        Ok(unique)
    }
}
