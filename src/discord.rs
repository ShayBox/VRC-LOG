use std::{sync::Arc, time::Duration};

use discord_presence::Client;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

pub const CLIENT_ID: u64 = 1_137_885_877_918_502_923;
pub const DEVELOPER_ID: &str = "358558305997684739";

#[derive(Clone, Deserialize, Serialize)]
pub struct Event {
    pub user: User,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct User {
    pub id:   String,
    #[serde(rename = "username")]
    pub name: String,
    #[serde(rename = "global_name")]
    pub nick: String,
}

lazy_static::lazy_static! {
    pub static ref USER: Option<User> = {
        let user = Arc::new(RwLock::new(None));
        let user_clone = user.clone();

        let mut client = Client::new(CLIENT_ID);
        // client.on_error(|ctx| eprintln!("{ctx:#?}")); // discord-presence v0.5.18 constantly emits errors...
        client.on_ready(move |ctx| {
            let Ok(event) = serde_json::from_value::<Event>(ctx.event) else {
                return;
            };

            *user.write() = Some(event.user);
        });

        let thread = client.start();
        std::thread::sleep(Duration::from_secs(5));
        thread.stop().expect("Failed to stop RPC thread");

        let user = user_clone.read();

        user.clone()
    };
}
