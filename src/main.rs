#[macro_use]
extern crate tracing;

use chrono::{Local, Offset};
#[cfg(feature = "title")]
use crossterm::{execute, terminal::SetTitle};
use terminal_link::Link;
use time::{macros::format_description, UtcOffset};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt::time::OffsetTime, EnvFilter};
use vrc_log::{
    vrchat::{VRCHAT_AMP_PATH, VRCHAT_LOW_PATH},
    CARGO_PKG_HOMEPAGE,
};

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "title")]
    execute!(std::io::stdout(), SetTitle("VRC-LOG"))?;

    /* Debugging: RUST_LOG=vrc_log=debug */
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with_target(false)
        .with_timer(OffsetTime::new(
            UtcOffset::from_whole_seconds(Local::now().offset().fix().local_minus_utc())?,
            format_description!("[hour repr:12]:[minute]:[second] [period]"),
        ))
        .init();

    if vrc_log::check_for_updates()? {
        let text = "An update is available";
        let link = Link::new(text, CARGO_PKG_HOMEPAGE);
        info!("{link}");
    }

    let args = std::env::args();
    let (tx, rx) = crossbeam::channel::unbounded();
    let _ = vrc_log::watch(tx.clone(), VRCHAT_AMP_PATH.as_path())?;
    let _ = vrc_log::watch(tx.clone(), VRCHAT_LOW_PATH.as_path())?;

    vrc_log::launch_game(args)?;
    vrc_log::process_avatars((tx, rx))
}
