use std::{sync::Arc, time::Duration};

use cached::proc_macro::once;
use discord_presence::{
    models::{EventData, PartialUser},
    Client,
};
use parking_lot::Mutex;

pub const CLIENT_ID: u64 = 1_137_885_877_918_502_923;
pub const DEVELOPER_ID: &str = "358558305997684739";

pub struct Discord {
    pub client: Client,
    pub user:   Arc<Mutex<Option<PartialUser>>>,
}

impl Discord {
    fn start() -> Self {
        let mut discord = Self {
            client: Client::new(CLIENT_ID),
            user:   Arc::default(),
        };

        let user = discord.user.clone();
        discord
            .client
            .on_ready(move |ctx| {
                if let EventData::Ready(event) = ctx.event {
                    *user.lock() = event.user;
                };
            })
            .persist();

        discord.client.start();
        discord
    }
}

#[once(sync_writes = true)]
pub fn get_dev_id() -> String {
    warn!("Error: Discord RPC Connection Failed\n");
    warn!("This may be due to one of the following reasons:");
    warn!("1. Discord is not running on your system.");
    warn!("2. VRC-LOG was restarted too quickly.\n");
    warn!("The User ID will default to the developer: ShayBox");

    std::env::var("DISCORD").unwrap_or_else(|_| DEVELOPER_ID.to_owned())
}

#[once(option = true, sync_writes = true)]
pub fn get_user_id() -> Option<String> {
    let discord = Discord::start();
    let user = discord.user.lock().clone();

    // block_until_event will never timeout
    std::thread::sleep(Duration::from_secs(5));
    discord.client.shutdown().ok()?;

    Some(match user {
        None => get_dev_id(),
        Some(user) => {
            let userid = user.id.unwrap_or_else(get_dev_id);
            if userid == "1045800378228281345" {
                warn!("Vesktop & arRPC doesn't support fetching user info");
                warn!("You can supply the 'DISCORD' env variable manually");
                warn!("The User ID will default to the developer: ShayBox");

                std::env::var("DISCORD").unwrap_or_else(|_| DEVELOPER_ID.to_owned())
            } else {
                if let Some(username) = user.username {
                    info!("[Discord] Authenticated as {username}");
                }

                userid
            }
        }
    })
}
