use anyhow::bail;
#[cfg(feature = "title")]
use crossterm::{execute, terminal::SetTitle};
use vrc_log::{
    box_provider,
    config::VRChatConfig,
    provider::{prelude::*, Provider, ProviderType, Providers},
};

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "title")]
    execute!(std::io::stdout(), SetTitle("VRC-OSC"))?;

    let config = VRChatConfig::load()?;
    let providers = Providers::from([
        #[cfg(feature = "ravenwood")]
        (ProviderType::Ravenwood, box_provider!(Ravenwood::default())),
        #[cfg(feature = "sqlite")]
        (ProviderType::Sqlite, box_provider!(Sqlite::new()?)),
    ]);

    #[cfg(feature = "sqlite")]
    let cache = &providers[&ProviderType::Sqlite];

    let (_tx, rx, _pw) = vrc_log::watch(config.cache_directory)?;
    while let Ok(path) = rx.recv() {
        let Ok(avatar_ids) = vrc_log::parse_avatar_ids(path) else {
            continue;
        };

        for avatar_id in avatar_ids {
            #[cfg(feature = "sqlite")]
            if !cache.send_avatar_id(&avatar_id).unwrap_or(true) {
                continue;
            }

            vrc_log::print_colorized(&avatar_id);
            for (provider_type, provider) in &providers {
                #[cfg(feature = "sqlite")]
                if provider_type == &ProviderType::Sqlite {
                    continue;
                }

                if provider.send_avatar_id(&avatar_id)? {
                    println!("^ Successfully Submitted to {provider_type} ^");
                }
            }
        }
    }

    bail!("Channel Closed");
}
