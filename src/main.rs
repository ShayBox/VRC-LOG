use anyhow::bail;
#[cfg(feature = "title")]
use crossterm::{execute, terminal::SetTitle};
use vrc_log::{
    box_provider,
    config::VRChatConfig,
    provider::{prelude::*, ProviderType, Providers},
};

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "title")]
    execute!(std::io::stdout(), SetTitle("VRC-LOG"))?;

    let config = VRChatConfig::load()?;
    let providers = Providers::from([
        #[cfg(all(feature = "cache", feature = "sqlite"))]
        (ProviderType::Cache, box_provider!(Sqlite::new()?)),
        #[cfg(feature = "ravenwood")]
        (ProviderType::Ravenwood, box_provider!(Ravenwood::default())),
        #[cfg(all(feature = "sqlite", not(feature = "cache")))]
        (ProviderType::Sqlite, box_provider!(Sqlite::new()?)),
    ]);

    #[cfg(feature = "cache")]
    let cache = &providers[&ProviderType::Cache];

    let (_tx, rx, _pw) = vrc_log::watch(config.cache_directory)?;
    while let Ok(path) = rx.recv() {
        let Ok(avatar_ids) = vrc_log::parse_avatar_ids(path) else {
            continue;
        };

        for avatar_id in avatar_ids {
            #[cfg(feature = "cache")]
            if !cache.send_avatar_id(&avatar_id).unwrap_or(true) {
                continue;
            }

            vrc_log::print_colorized(&avatar_id);
            for (provider_type, provider) in &providers {
                if provider.send_avatar_id(&avatar_id)? {
                    println!("^ Successfully Submitted to {provider_type} ^");
                }
            }
        }
    }

    bail!("Channel Closed");
}
