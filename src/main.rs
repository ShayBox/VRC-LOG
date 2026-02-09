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
use time::{UtcOffset, macros::format_description};
use tokio::signal;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt::time::OffsetTime};
use vrc_log::{
    CARGO_PKG_HOMEPAGE, provider,
    provider::{ProviderKind, avtrdb::AvtrDBActor, prelude::*},
    settings::Settings,
    vrchat::{VRCHAT_AMP_PATH, VRCHAT_LOW_PATH},
};

/* Watchers will stop working if they get dropped. */
static WATCHERS: OnceLock<Vec<PollWatcher>> = OnceLock::new();

#[allow(clippy::too_many_lines)]
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

    let mut args = std::env::args().collect::<Vec<_>>();
    let force_wizard = args.iter().any(|arg| arg == "--wizard" || arg == "-w");
    if force_wizard {
        args.retain(|arg| arg != "--wizard" && arg != "-w");
    }

    let settings = if force_wizard {
        info!("Setup wizard requested via flag");
        Settings::try_wizard().expect("Failed to setup wizard")
    } else {
        Settings::load().unwrap_or_else(|error| match error {
            ConfigError::Io(error) if error.kind() == ErrorKind::NotFound => {
                info!("Welcome to VRC-LOG! Please follow the setup wizard");
                Settings::try_wizard().expect("Failed to setup wizard")
            }
            error => {
                error!("There was an error loading the settings: {error}");
                error!("Most likely an update. Please follow the setup wizard");
                Settings::try_wizard().expect("Failed to setup wizard")
            }
        })
    };

    let (tx, rx) = flume::unbounded();
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
    vrc_log::launch_game(&args)?;

    // This is a little wonky, but effecively we are creating a controlled memory leak
    // which will be static for the rest of the programms runtime
    let settings_static: &'static Settings = Box::leak(Box::new(settings));

    let (mut avtrdb_actor, avtrdb_sender) = AvtrDBActor::new(settings_static);

    let providers = settings_static
        .providers
        .iter()
        .map(|provider| match provider {
            #[cfg(feature = "nsvr")]
            ProviderKind::NSVR => provider!(NSVR::new(settings_static)),
            #[cfg(feature = "paw")]
            ProviderKind::PAW => provider!(Paw::new(settings_static)),
            #[cfg(feature = "vrcdb")]
            ProviderKind::VRCDB => provider!(VrcDB::new(settings_static)),
            #[cfg(feature = "vrcwb")]
            ProviderKind::VRCWB => provider!(VrcWB::new(settings_static)),
            #[cfg(feature = "avtrdb")]
            ProviderKind::AVTRDB => provider!(AvtrDB::new(avtrdb_sender.clone())),
        })
        .collect::<Vec<_>>();

    let avtrdb_handle = tokio::spawn(async move { avtrdb_actor.run().await });

    let handle = tokio::spawn(vrc_log::process_avatars(
        providers,
        settings_static.clear_amplitude,
        (tx, rx),
    ));

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            handle.abort();
            avtrdb_handle.abort();
        },
        () = terminate => {
            handle.abort();
            avtrdb_handle.abort();
        },
    }

    Ok(())
}
