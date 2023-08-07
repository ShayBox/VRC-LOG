use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use discord_presence::Client as DiscordRPC;
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
    status: i64,
}

pub struct Ravenwood {
    client: Client,
    discord_id: Arc<RwLock<String>>,
}

impl Default for Ravenwood {
    fn default() -> Self {
        println!("[Ravenwood] Attempting to get your Discord ID using RPC");

        let client = Client::new();
        let discord_id = Arc::new(RwLock::new(String::from(USER_ID)));
        let discord_id_rpc = discord_id.clone();
        let mut discord_rpc = DiscordRPC::new(CLIENT_ID);
        discord_rpc.on_error(|ctx| eprintln!("{ctx:#?}"));
        discord_rpc.on_ready(move |ctx| {
            if let Some(event) = ctx.event.as_object() {
                if let Some(user) = event.get("user").and_then(Value::as_object) {
                    if let Some(id) = user.get("id").and_then(Value::as_str) {
                        println!("[Ravenwood] Got your Discord ID: {id}");
                        *discord_id_rpc.write().unwrap() = id.to_owned();
                    }
                }
            };
        });
        let _ = discord_rpc.start();
        std::thread::sleep(Duration::from_secs(15));

        if *discord_id.read().unwrap() == USER_ID {
            println!("[Ravenwood] Couldn't get your Discord ID, please make sure Discord is open");
            println!("[Ravenwood] Defaulting to the ID of the developer of this program, ShayBox");
        }

        Self { client, discord_id }
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
                ("userid", &self.discord_id.read().unwrap()),
            ]))
            .send()?
            .json::<RavenwoodResponse>()?;

        Ok(response.status.status == 201)
    }
}
