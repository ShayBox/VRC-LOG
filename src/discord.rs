use std::{sync::Arc, time::Duration};

use discord_presence::{
    models::{EventData, PartialUser},
    Client,
};
use parking_lot::RwLock;

pub const CLIENT_ID: u64 = 1_137_885_877_918_502_923;
pub const DEVELOPER_ID: &str = "358558305997684739";

lazy_static::lazy_static! {
    pub static ref USER: Option<PartialUser> = {
        let user_event = Arc::new(RwLock::new(None));
        let user_clone = user_event.clone();
        let mut client = Client::new(CLIENT_ID);
        client.on_ready(move |ctx| {
            if let EventData::Ready(event) = ctx.event {
                *user_event.write() = event.user;
            };
        }).persist();

        client.start();

        // block_until_event will never timeout
        std::thread::sleep(Duration::from_secs(5));

        client.shutdown().expect("Failed to stop RPC thread");

        let user = user_clone.read();

        user.clone()
    };
}
