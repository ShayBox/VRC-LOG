use std::time::Duration;

#[cfg(feature = "title")]
use crossterm::{execute, terminal::SetTitle};
use vrc_log::{
    box_db,
    config::VRChat,
    provider::{prelude::*, Providers, Type},
};

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "title")]
    execute!(std::io::stdout(), SetTitle("VRC-LOG"))?;

    let config = VRChat::load()?;
    #[cfg_attr(not(feature = "cache"), allow(unused_mut))]
    let mut providers = Providers::from([
        #[cfg(all(feature = "cache", feature = "sqlite"))]
        (Type::Cache, box_db!(Sqlite::new()?)),
        #[cfg(feature = "vrcdb")]
        (Type::VRCDB, box_db!(VRCDB::default())),
        #[cfg(all(feature = "sqlite", not(feature = "cache")))]
        (Type::Sqlite, box_db!(Sqlite::new()?)),
    ]);

    #[cfg(feature = "cache")]
    let cache = providers.shift_remove(&Type::Cache).unwrap();

    let (_tx, rx, _pw) = vrc_log::watch(config.cache_directory)?;
    while let Ok(path) = rx.recv() {
        let Ok(avatar_ids) = vrc_log::parse_avatar_ids(path) else {
            continue;
        };

        for avatar_id in avatar_ids {
            #[cfg(feature = "cache")] // Avatar is already in cache
            if !cache.check_avatar_id(&avatar_id).unwrap_or(true) {
                continue;
            };

            #[cfg(feature = "cache")] // Don't send to cache if sending failed
            let mut send_to_cache = true;
            let local_time = vrc_log::get_local_time();

            vrc_log::print_colorized(&avatar_id);
            std::thread::sleep(Duration::from_secs(1));

            for (provider_type, provider) in &providers {
                match provider.send_avatar_id(&avatar_id) {
                    Ok(unique) => {
                        if unique {
                            println!("^ {local_time}: Successfully Submitted to {provider_type}");
                        }
                    }
                    Err(error) => {
                        send_to_cache = false;
                        eprintln!("^ {local_time}: Failed to submit to {provider_type}: {error}");
                    }
                }
            }

            #[cfg(feature = "cache")]
            if send_to_cache {
                cache.send_avatar_id(&avatar_id)?;
            }
        }
    }

    anyhow::bail!("Channel Closed");
}
