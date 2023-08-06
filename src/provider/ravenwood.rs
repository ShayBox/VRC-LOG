use std::collections::HashMap;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::provider::{Provider, USER_AGENT};

#[derive(Deserialize, Serialize)]
pub struct RavenwoodResponse {
    status: Status,
}

#[derive(Deserialize, Serialize)]
pub struct Status {
    message: String,
    status: i64,
}

#[derive(Default)]
pub struct Ravenwood {
    client: Client,
}

impl Provider for Ravenwood {
    fn send_avatar_id(&self, avatar_id: &str) -> anyhow::Result<bool> {
        let response = self
            .client
            .put("https://api.ravenwood.dev/avatars/putavatar")
            .header("User-Agent", USER_AGENT)
            .json(&HashMap::from([
                ("id", avatar_id),
                ("userid", "358558305997684739"),
            ]))
            .send()?
            .json::<RavenwoodResponse>()?;

        Ok(response.status.status == 201)
    }
}
