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
        #[cfg(feature = "ravenwood")]
        (Type::Ravenwood, box_db!(Ravenwood::default())),
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
            #[cfg(feature = "cache")] // Check if the avatar is unique
            let mut unique_and_submitted = cache.check_avatar_id(&avatar_id).unwrap_or(true);

            #[cfg(feature = "cache")] // Skip if the avatar is not unique
            if !unique_and_submitted {
                continue;
            }

            vrc_log::print_colorized(&avatar_id); // Submit the avatar to providers
            for (provider_type, provider) in &providers {
                if let Err(error) = provider.send_avatar_id(&avatar_id) {
                    unique_and_submitted = false; // Failed to submit
                    eprintln!("^ Failed to submit to {provider_type}: {error}");
                } else {
                    println!("^ Successfully Submitted to {provider_type} ^");
                }
            }

            #[cfg(feature = "cache")]
            if unique_and_submitted {
                cache.send_avatar_id(&avatar_id)?;
            }
        }
    }

    anyhow::bail!("Channel Closed");
}
