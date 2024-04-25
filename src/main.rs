#[cfg(feature = "title")]
use crossterm::{execute, terminal::SetTitle};
use terminal_link::Link;
use vrc_log::{vrchat::VRChat, CARGO_PKG_HOMEPAGE};

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "title")]
    execute!(std::io::stdout(), SetTitle("VRC-LOG"))?;

    if vrc_log::check_for_updates()? {
        let text = "An update is available";
        let link = Link::new(text, CARGO_PKG_HOMEPAGE);
        println!("{link}");
    }

    let args = std::env::args();
    let vrchat = VRChat::load()?;
    let watcher = vrc_log::watch(vrchat.cache_directory)?;

    vrc_log::launch_game(args)?;
    vrc_log::process_avatars(watcher)
}
