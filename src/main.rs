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

        for avatar_id in avatar_ids {
            if !cache.send_avatar_id(&avatar_id)? {
                continue;
            };

            println!("vrcx://avatar/{avatar_id}");

            for (name, provider) in &providers {
                if provider.send_avatar_id(&avatar_id)? {
                    println!("^ Successfully Submitted to {name} ^");
                };
            }
        }
    }
}
