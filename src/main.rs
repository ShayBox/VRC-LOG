use std::collections::HashMap;

use colored::{Color, Colorize};
use vrc_log::{
    config::VRChatConfig,
    provider,
    provider::{prelude::*, Provider, Providers},
};

const COLORS: [Color; 10] = [
    Color::Red,
    Color::Yellow,
    Color::BrightYellow,
    Color::Green,
    Color::BrightGreen,
    Color::Cyan,
    Color::BrightCyan,
    Color::BrightBlue,
    Color::Blue,
    Color::Magenta,
];

fn main() -> anyhow::Result<()> {
    let cache = Sqlite::new()?;
    let config = VRChatConfig::load()?;
    let receiver = vrc_log::watch(config.cache_directory)?;
    let providers: Providers = HashMap::from([
        // ("Sqlite", provider!(Sqlite::new()?)),
        ("Ravenwood", provider!(Ravenwood::default())),
    ]);

    let mut color_index = 0;
    loop {
        let Ok(path) = receiver.recv() else {
            continue;
        };

        let Ok(avatar_ids) = vrc_log::parse_avatar_ids(path) else {
            continue;
        };

        avatar_ids
            .iter()
            .filter(|avatar_id| cache.send_avatar_id(avatar_id).unwrap_or(false))
            .for_each(|avatar_id| {
                let color = COLORS[color_index];
                let text = format!("vrcx://avatar/{avatar_id}").color(color);
                color_index = (color_index + 1) % COLORS.len();
                println!("{text}");

                let _ = providers
                    .iter()
                    .map(|(avatar_id, provider)| provider.send_avatar_id(avatar_id));
            });
    }
}
