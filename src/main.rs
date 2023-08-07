use std::collections::HashMap;

use vrc_log::{
    config::VRChatConfig,
    provider,
    provider::{prelude::*, Provider, Providers},
};

fn main() -> anyhow::Result<()> {
    let cache = Sqlite::new()?;
    let config = VRChatConfig::load()?;
    let receiver = vrc_log::watch(config.cache_directory)?;
    let providers: Providers = HashMap::from([
        // ("Sqlite", provider!(Sqlite::new()?)),
        ("Ravenwood", provider!(Ravenwood::default())),
    ]);

    loop {
        let Ok(path) = receiver.recv() else {
            continue;
        };

        let Ok(avatar_ids) = vrc_log::parse_avatar_ids(path) else {
            continue;
        };

        avatar_ids
            .iter()
            .filter(|avatar_id| !cache.send_avatar_id(avatar_id).unwrap())
            .for_each(|avatar_id| {
                println!("vrcx://avatar/{avatar_id}");
                let _ = providers
                    .iter()
                    .map(|(avatar_id, provider)| provider.send_avatar_id(avatar_id));
            });
    }
}
