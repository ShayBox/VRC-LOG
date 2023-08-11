use std::{collections::HashMap, sync::Arc, time::Duration};

use discord_presence::Client as DiscordRPC;
use parking_lot::RwLock;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::provider::Provider;

const CLIENT_ID: u64 = 1137885877918502923;
const USER_ID: &str = "358558305997684739";
const USER_AGENT: &str = concat!(
    "VRC-LOG/",
    env!("CARGO_PKG_VERSION"),
    " shaybox@shaybox.com"
);

#[derive(Deserialize, Serialize)]
pub struct RavenwoodResponse {
    status: Status,
}

#[derive(Deserialize, Serialize)]
pub struct Status {
    message: String,
    status:  i64,
}

pub struct Ravenwood {
    client:  Client,
    user_id: Arc<RwLock<String>>,
}

impl Ravenwood {
    pub fn new(client: Client, user_id: Arc<RwLock<String>>) -> Self {
        Self { client, user_id }
    }
}

impl Default for Ravenwood {
    fn default() -> Self {
        let client = Client::default();
        let user_id = Arc::new(RwLock::new(String::new()));
        let user_id_rpc = user_id.clone();

        let mut discord_rpc = DiscordRPC::new(CLIENT_ID);
        discord_rpc.on_error(|ctx| eprintln!("{ctx:#?}"));
        discord_rpc.on_ready(move |ctx| {
            if let Some(event) = ctx.event.as_object() {
                if let Some(user) = event.get("user").and_then(Value::as_object) {
                    if let Some(id) = user.get("id").and_then(Value::as_str) {
                        *user_id_rpc.write() = id.into();
                    }
                }
            };
        });
        let _ = discord_rpc.start();
        std::thread::sleep(Duration::from_secs(5));

        if *user_id.read() == String::new() {
            *user_id.write() = USER_ID.into();
            println!("[Ravenwood] Couldn't get your Discord ID, please make sure Discord is open");
            println!("[Ravenwood] Defaulting to the ID of the developer of this program, ShayBox");
        }

        Self::new(client, user_id)
    }
}

impl Provider for Ravenwood {
    fn send_avatar_id(&self, avatar_id: &str) -> anyhow::Result<bool> {
        let response = self
            .client
            .put("https://api.ravenwood.dev/avatars/putavatar")
            .header("User-Agent", USER_AGENT)
            .json(&HashMap::from([
                ("id", avatar_id),
                ("userid", &self.user_id.read()),
            ]))
            .send()?
            .json::<RavenwoodResponse>()?;

        Ok(response.status.status == 201)
    }
}
