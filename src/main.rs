#[macro_use]
extern crate tracing;

use std::{io::ErrorKind, sync::OnceLock};

use anyhow::Result;
use chrono::{Local, Offset};
#[cfg(feature = "title")]
use crossterm::{execute, terminal::SetTitle};
use derive_config::{ConfigError, DeriveTomlConfig};
use notify::PollWatcher;
use terminal_link::Link;
use time::{macros::format_description, UtcOffset};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt::time::OffsetTime, EnvFilter};
use vrc_log::{
    settings::Settings,
    vrchat::{VRCHAT_AMP_PATH, VRCHAT_LOW_PATH},
    CARGO_PKG_HOMEPAGE,
};
/* Watchers will stop working if they get dropped. */
static WATCHERS: OnceLock<Vec<PollWatcher>> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<()> {
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

    if vrc_log::check_for_updates().await? {
        let text = "An update is available";
        let link = Link::new(text, CARGO_PKG_HOMEPAGE);
        info!("{link}");
    }

    let args = std::env::args();
    let settings = Settings::load().unwrap_or_else(|error| match error {
        ConfigError::Io(error) if error.kind() == ErrorKind::NotFound => {
            info!("Welcome to VRC-LOG! Please follow the setup wizard");
            Settings::try_wizard().expect("Failed to setup wizard")
        }
        error => {
            error!("There was an error loading the settings: {error}");
            Settings::try_wizard().expect("Failed to setup wizard")
        }
    });

    let (tx, rx) = crossbeam::channel::unbounded();
    let _ = WATCHERS.set(vec![
        vrc_log::watch(tx.clone(), VRCHAT_AMP_PATH.as_path(), 100)?,
        vrc_log::watch(tx.clone(), VRCHAT_LOW_PATH.as_path(), 1_000)?,
    ]);

    #[cfg(windows)]
    if vrc_log::windows::is_elevated()? {
        vrc_log::windows::spawn_procmon_watcher();
        info!("Running with elevated privileges.");
        info!("Starting Process Monitor for additional logging.");
        info!("Close Process Monitor manually to begin scans; it will reopen automatically.");
    }

    settings.save()?;
    vrc_log::launch_game(args)?;
    vrc_log::process_avatars(settings, (tx, rx)).await
}
