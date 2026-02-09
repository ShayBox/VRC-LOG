#[macro_use]
extern crate tracing;

use std::{
    collections::HashSet,
    ffi::OsStr,
    fs::{File, create_dir_all},
    io::{BufRead, BufReader, Error},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, LazyLock},
    time::Duration,
};

use anyhow::{Result, bail};
use chrono::Local;
use colored::{Color, Colorize};
use flume::{Receiver, Sender};
use lazy_regex::{Lazy, Regex, lazy_regex, regex_replace_all};
use notify::{Config, Event, PollWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use terminal_link::Link;

#[cfg(feature = "cache")]
use crate::process::process_with_cache;
#[cfg(not(feature = "cache"))]
use crate::process::process_without_cache;
use crate::provider::Provider;

#[cfg(feature = "cache")]
pub mod cache;
#[cfg(feature = "discord")]
pub mod discord;
mod process;
pub mod provider;
pub mod settings;
pub mod vrchat;
#[cfg(windows)]
pub mod windows;

pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CARGO_PKG_HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");
pub const USER_AGENT: &str = concat!(
    "VRC-LOG/",
    env!("CARGO_PKG_VERSION"),
    " shaybox@shaybox.com"
);

#[must_use]
pub fn get_local_time() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// # Errors
/// Will return `Err` if it couldn't get the GitHub repository.
pub async fn check_for_updates() -> reqwest::Result<bool> {
    let response = reqwest::get(CARGO_PKG_HOMEPAGE).await?;
    if let Some(mut segments) = response.url().path_segments()
        && let Some(remote_version) = segments.next_back()
    {
        return Ok(remote_version > CARGO_PKG_VERSION);
    }

    Ok(false)
}

/// # Errors
/// Will return `Err` if `PollWatcher::watch` errors
pub fn watch<P: AsRef<Path>>(
    tx: Sender<PathBuf>,
    path: P,
    millis: u64,
) -> notify::Result<PollWatcher> {
    let path = path.as_ref();
    debug!("Watching {path:?}");

    let tx_clone = tx.clone();
    let mut watcher = PollWatcher::with_initial_scan(
        move |watch_event: notify::Result<Event>| {
            if let Ok(event) = watch_event {
                for path in event.paths {
                    if let Some(extension) = path.extension().and_then(OsStr::to_str)
                        && ["csv", "log", "txt"].contains(&extension)
                    {
                        let _ = tx.send(path.clone());
                    }
                    if let Some(filename) = path.file_name().and_then(OsStr::to_str)
                        && filename == "amplitude.cache"
                    {
                        let _ = tx.send(path);
                    }
                }
            }
        },
        Config::default()
            .with_compare_contents(true)
            .with_poll_interval(Duration::from_millis(millis)),
        move |scan_event: notify::Result<PathBuf>| {
            if let Ok(path) = scan_event {
                if let Some(extension) = path.extension().and_then(OsStr::to_str)
                    && ["csv", "log", "txt"].contains(&extension)
                {
                    let _ = tx_clone.send(path.clone());
                }
                if let Some(filename) = path.file_name().and_then(OsStr::to_str)
                    && filename == "amplitude.cache"
                {
                    let _ = tx_clone.send(path);
                }
            }
        },
    )?;

    watcher.watch(path, RecursiveMode::NonRecursive)?;

    Ok(watcher)
}

/// Steam Game Launch Options: `.../vrc-log(.exe) %command%`
///
/// # Errors
/// Will return `Err` if `Command::spawn` errors
/// # Panics
/// Will panic if `Child::wait` panics
pub fn launch_game(args: &[String]) -> Result<()> {
    if args.len() > 1 {
        let mut child = Command::new(args[1].clone())
            .args(args.iter().skip(2))
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .spawn()?;

        std::thread::spawn(move || {
            child.wait().unwrap();
            std::process::exit(0);
        });
    }

    Ok(())
}

/// # Errors
/// Will return `Err` if `Sqlite::new` or `Provider::send_avatar_id` errors
pub async fn process_avatars(
    providers: Vec<Arc<Box<dyn Provider>>>,
    clear_amplitude: bool,
    (_tx, rx): (Sender<PathBuf>, Receiver<PathBuf>),
) -> Result<()> {
    #[cfg(feature = "cache")]
    let cache = cache::Cache::new().await?;

    while let Ok(path) = rx.recv_async().await {
        let avatar_ids = parse_avatar_ids(&path);

        // Clear amplitude file after reading if enabled and it's an amplitude file
        if clear_amplitude && path.file_name().and_then(|n| n.to_str()) == Some("amplitude.cache") {
            match std::fs::write(&path, "") {
                Ok(()) => debug!("Cleared amplitude file: {path:?}"),
                Err(error) => warn!("Failed to clear amplitude file: {error}"),
            }
        }

        #[cfg(feature = "cache")]
        process_with_cache(providers.clone(), &cache, avatar_ids).await?;
        #[cfg(not(feature = "cache"))]
        process_without_cache(providers.clone(), avatar_ids).await?;
    }

    bail!("Channel Closed")
}

/// # Errors
/// Will return `Err` if `std::fs::canonicalize` errors
///
/// # Panics
/// Will panic if an environment variable doesn't exist
pub fn parse_path_env(path: &str) -> Result<PathBuf, Error> {
    let path = regex_replace_all!(r"(?:\$|%)(\w+)%?", path, |_, env| {
        std::env::var(env).unwrap_or_else(|_| panic!("Environment Variable not found: {env}"))
    });

    let path = Path::new(path.as_ref());
    if !path.exists() {
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }
        File::create(path)?;
    }

    std::fs::canonicalize(path)
}

#[must_use]
pub fn parse_avatar_ids(path: &PathBuf) -> impl IntoIterator<Item = String> {
    #[allow(clippy::non_std_lazy_statics)]
    static RE: Lazy<Regex> = lazy_regex!(r"avtr_\w{8}-\w{4}-\w{4}-\w{4}-\w{12}");

    let Ok(file) = File::open(path) else {
        return HashSet::new(); // Directory
    };

    let mut reader = BufReader::new(file);
    let mut avatar_ids = HashSet::new(); // Filter out duplicates
    let mut buf = Vec::new();

    while reader.read_until(b'\n', &mut buf).unwrap_or(0) > 0 {
        let line = String::from_utf8_lossy(&buf);
        for mat in RE.find_iter(&line) {
            avatar_ids.insert(mat.as_str().to_string());
        }
        buf.clear();
    }

    avatar_ids
}

/// # Print with colorized rainbow rows for separation
pub fn print_colorized(avatar_id: &str) {
    static INDEX: LazyLock<RwLock<usize>> = LazyLock::new(|| RwLock::new(0));
    static COLORS: LazyLock<[Color; 12]> = LazyLock::new(|| {
        [
            Color::Red,
            Color::BrightRed,
            Color::Yellow,
            Color::BrightYellow,
            Color::Green,
            Color::BrightGreen,
            Color::Blue,
            Color::BrightBlue,
            Color::Cyan,
            Color::BrightCyan,
            Color::Magenta,
            Color::BrightMagenta,
        ]
    });

    let index = *INDEX.read();
    let color = COLORS[index];
    *INDEX.write() = (index + 1) % COLORS.len();

    let text = format!("vrcx://avatar/{avatar_id}");
    let link = Link::new(&text, &text).to_string().color(color);
    info!("{link}");
}
