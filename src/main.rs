use std::collections::HashMap;

use anyhow::bail;
use vrc_log::{
    config::VRChatConfig,
    provider,
    provider::{prelude::*, Provider, Providers},
};

fn main() -> anyhow::Result<()> {
    let cache = Sqlite::new()?;
    let config = VRChatConfig::load()?;
    let providers: Providers = HashMap::from([
        // ("Sqlite", provider!(Sqlite::new()?)),
        ("Ravenwood", provider!(Ravenwood::default())),
    ]);

    let (_tx, rx, _watcher) = vrc_log::watch(config.cache_directory)?;
    while let Ok(path) = rx.recv() {
        let Ok(avatar_ids) = vrc_log::parse_avatar_ids(path) else {
            continue;
        };

        for avatar_id in avatar_ids {
            if !cache.send_avatar_id(&avatar_id).unwrap_or(true) {
                continue;
            }

            vrc_log::print_colorized(&avatar_id);
            for (name, provider) in &providers {
                if provider.send_avatar_id(&avatar_id)? {
                    println!("^ Successfully Submitted to {name} ^");
                }
            }
        }
    }

    bail!("Channel Closed");
}
