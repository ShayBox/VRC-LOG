#[cfg(feature = "title")]
use crossterm::{execute, terminal::SetTitle};
use terminal_link::Link;
use vrc_log::{config::VRChat, CARGO_PKG_HOMEPAGE};

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "title")]
    execute!(std::io::stdout(), SetTitle("VRC-LOG"))?;

    if vrc_log::check_for_updates()? {
        let link = Link::new("An update is available", CARGO_PKG_HOMEPAGE);
        println!("{link}");
    }

    let args = std::env::args();
    let config = VRChat::load()?;
    let watcher = vrc_log::watch(config.cache_directory)?;

    vrc_log::launch_game(args)?;
    vrc_log::process_avatars(watcher)
}
