#[macro_use]
extern crate tracing;

use std::{io::ErrorKind, sync::OnceLock};

use anyhow::Result;
use chrono::{Local, Offset};
#[cfg(feature = "title")]
use crossterm::{execute, terminal::SetTitle};
use derive_config::{ConfigError, DeriveTomlConfig};
use notify::PollWatcher;
use strum::IntoEnumIterator;
use terminal_link::Link;
use time::{macros::format_description, UtcOffset};
use tokio::signal;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt::time::OffsetTime, EnvFilter};
use vrc_log::{
    provider,
    provider::{
        avtrdb::AvtrDBActor, cutedb::CuteDBActor, kitsunedb::KitsuneDBActor, prelude::*,
        ProviderKind,
    },
    settings::Settings,
    vrchat::{VRCHAT_AMP_PATH, VRCHAT_LOW_PATH},
    CARGO_PKG_HOMEPAGE,
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
        .with_ansi(true)
        .with_ansi_sanitization(false)
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

    let mut settings = if force_wizard {
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

    if settings.providers.len() != ProviderKind::iter().count() {
        info!("Additional providers have been added, triggering setup wizard");
        settings = Settings::try_wizard().expect("Failed to setup wizard");
    }

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

    // This is a little wonky, but effectively we are creating a controlled memory leak,
    // which will be static for the rest of the programs runtime.
    let settings: &'static Settings = Box::leak(Box::new(settings));

    let (mut avtrdb_actor, avtrdb_sender) = AvtrDBActor::new(settings);
    let (mut cutedb_actor, cutedb_sender) = CuteDBActor::new();
    let (mut kitsunedb_actor, kitsunedb_sender) = KitsuneDBActor::new(settings);

    let providers = settings
        .providers
        .iter()
        .filter(|(_, enabled)| **enabled)
        .map(|(provider, _)| match provider {
            #[cfg(feature = "nsvr")]
            ProviderKind::NSVR => provider!(NSVR::new(settings)),
            #[cfg(feature = "paw")]
            ProviderKind::PAW => provider!(Paw::new(settings)),
            #[cfg(feature = "vrcdb")]
            ProviderKind::VRCDB => provider!(VrcDB::new(settings)),
            #[cfg(feature = "vrcwb")]
            ProviderKind::VRCWB => provider!(VrcWB::new(settings)),
            #[cfg(feature = "avtrdb")]
            ProviderKind::AVTRDB => provider!(AvtrDB::new(avtrdb_sender.clone())),
            #[cfg(feature = "avtrzip")]
            ProviderKind::AVTRZIP => provider!(AvtrZip::default()),
            #[cfg(feature = "kitsunedb")]
            ProviderKind::KITSUNEDB => provider!(KitsuneDB::new(kitsunedb_sender.clone())),
            #[cfg(feature = "cutedb")]
            ProviderKind::CUTEDB => provider!(CuteDB::new(cutedb_sender.clone())),
        })
        .collect::<Vec<_>>();

    let avtrdb_handle = tokio::spawn(async move { avtrdb_actor.run().await });
    let cutedb_handle = tokio::spawn(async move { cutedb_actor.run().await });
    let kitsunedb_handle = tokio::spawn(async move { kitsunedb_actor.run().await });

    let handle = tokio::spawn(vrc_log::process_avatars(providers, settings, (tx, rx)));

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

    // On Windows, closing the console window via the X button, logging off, or
    // a system shutdown deliver CTRL_CLOSE/LOGOFF/SHUTDOWN_EVENT, not Ctrl+C.
    // Without catching these too, the vast majority of users (who just click
    // the X button rather than pressing Ctrl+C) would skip the graceful
    // shutdown below entirely — Windows kills the process outright, silently
    // discarding whatever avatars are still buffered in the KitsuneDB/avtrDB
    // actors even though the local cache already marked them as sent.
    #[cfg(windows)]
    let terminate = async {
        let mut close =
            signal::windows::ctrl_close().expect("failed to install ctrl-close handler");
        let mut logoff =
            signal::windows::ctrl_logoff().expect("failed to install ctrl-logoff handler");
        let mut shutdown =
            signal::windows::ctrl_shutdown().expect("failed to install ctrl-shutdown handler");

        tokio::select! {
            _ = close.recv() => {},
            _ = logoff.recv() => {},
            _ = shutdown.recv() => {},
        }
    };

    #[cfg(not(any(unix, windows)))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    // Graceful shutdown: stop pulling in new avatar IDs, then let each actor
    // drain and flush whatever it already has buffered before we exit — see
    // the matching change in provider/kitsunedb.rs and provider/avtrdb.rs.
    handle.abort();
    drop(avtrdb_sender);
    drop(cutedb_sender);
    drop(kitsunedb_sender);

    let shutdown_timeout = std::time::Duration::from_secs(90);
    if tokio::time::timeout(shutdown_timeout, avtrdb_handle)
        .await
        .is_err()
    {
        error!("avtrDB actor did not finish flushing before shutdown timed out");
    }
    if tokio::time::timeout(shutdown_timeout, cutedb_handle)
        .await
        .is_err()
    {
        error!("CuteDB actor did not finish flushing before shutdown timed out");
    }
    if tokio::time::timeout(shutdown_timeout, kitsunedb_handle)
        .await
        .is_err()
    {
        error!("KitsuneDB actor did not finish flushing before shutdown timed out");
    }

    Ok(())
}
