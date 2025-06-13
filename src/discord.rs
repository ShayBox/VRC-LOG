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
                }
            })
            .persist();

        discord.client.start();
        discord
    }
}

#[once(option = true, sync_writes = true)]
pub fn get_user() -> Option<PartialUser> {
    let discord = Discord::start();
    std::thread::sleep(Duration::from_secs(5));
    discord.client.shutdown().ok()?;

    if let Some(user) = discord.user.lock().as_ref() {
        if let Some(user_id) = &user.id {
            if user_id != "1045800378228281345" {
                if let Some(username) = &user.username {
                    info!("[Discord] Authenticated as {username}");
                }

                return Some(user.clone());
            }

            warn!("Vesktop & arRPC doesn't support fetching user info");
            warn!("You can supply the 'DISCORD' env variable manually");
        }
    } else {
        warn!("Error: Discord RPC Connection Failed\n");
        warn!("This may be due to one of the following reasons:");
        warn!("1. Discord is not running on your system.");
        warn!("2. VRC-LOG was restarted too quickly.\n");
    }

    None
}
