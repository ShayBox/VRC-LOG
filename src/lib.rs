#[macro_use]
extern crate tracing;

use std::{
    env::Args,
    fs::File,
    io::{BufRead, BufReader, Error},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::LazyLock,
    time::Duration,
};

use anyhow::{bail, Context};
use chrono::Local;
use colored::{Color, Colorize};
use crossbeam::channel::{Receiver, Sender};
use lazy_regex::{lazy_regex, regex_replace_all, Lazy, Regex};
use notify::{Config, Event, PollWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use terminal_link::Link;

use crate::provider::{prelude::*, Providers, Type};

#[cfg(feature = "discord")]
pub mod discord;
pub mod provider;
pub mod vrchat;

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
pub fn check_for_updates() -> reqwest::Result<bool> {
    let response = reqwest::blocking::get(CARGO_PKG_HOMEPAGE)?;
    if let Some(mut segments) = response.url().path_segments() {
        if let Some(remote_version) = segments.next_back() {
            return Ok(remote_version > CARGO_PKG_VERSION);
        }
    }

    Ok(false)
}

/// # Errors
/// Will return `Err` if `PollWatcher::watch` errors
pub fn watch<P: AsRef<Path>>(tx: Sender<PathBuf>, path: P) -> notify::Result<PollWatcher> {
    let tx_clone = tx.clone();
    let mut watcher = PollWatcher::with_initial_scan(
        move |watch_event: notify::Result<Event>| {
            if let Ok(event) = watch_event {
                for path in event.paths {
                    let _ = tx.send(path);
                }
            }
        },
        Config::default().with_poll_interval(Duration::from_secs(1)),
        move |scan_event: notify::Result<PathBuf>| {
            if let Ok(path) = scan_event {
                let _ = tx_clone.send(path);
            }
        },
    )?;

    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;

    Ok(watcher)
}

/// Steam Game Launch Options: `.../vrc-log(.exe) %command%`
///
/// # Errors
/// Will return `Err` if `Command::spawn` errors
/// # Panics
/// Will panic if `Child::wait` panics
pub fn launch_game(args: Args) -> anyhow::Result<()> {
    let args = args.collect::<Vec<_>>();
    if args.len() > 1 {
        let mut child = Command::new(&args[1])
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
pub fn process_avatars(rx: &Receiver<PathBuf>) -> anyhow::Result<()> {
    #[cfg_attr(not(feature = "cache"), allow(unused_mut))]
    let mut providers = Providers::from([
        #[cfg(feature = "cache")]
        (Type::CACHE, box_db!(Cache::new()?)),
        #[cfg(feature = "avtrdb")]
        (Type::AVTRDB, box_db!(AvtrDB::default())),
        #[cfg(feature = "vrcwb")]
        (Type::VRCWB, box_db!(VRCWB::default())),
        #[cfg(feature = "vrcds")]
        (Type::VRCDS, box_db!(VRCDS::default())),
        #[cfg(feature = "vrcdb")]
        (Type::VRCDB, box_db!(VRCDB::default())),
    ]);

    #[cfg(feature = "cache")]
    let cache = providers.shift_remove(&Type::CACHE).context("None")?;

    while let Ok(path) = rx.recv() {
        let avatar_ids = parse_avatar_ids(&path);
        for avatar_id in avatar_ids {
            #[cfg(feature = "cache")] // Avatar already in cache
            if !cache.check_avatar_id(&avatar_id).unwrap_or(true) {
                continue;
            }

            #[cfg(feature = "cache")] // Don't send to cache if sending failed
            let mut send_to_cache = true;

            print_colorized(&avatar_id);

            for (provider_type, provider) in &providers {
                match provider.send_avatar_id(&avatar_id) {
                    Ok(unique) => {
                        if unique {
                            info!("^ Successfully Submitted to {provider_type}");
                        }
                    }
                    Err(error) => {
                        send_to_cache = false;
                        error!("^ Failed to submit to {provider_type}: {error}");
                    }
                }
            }

            #[cfg(feature = "cache")]
            if send_to_cache {
                cache.send_avatar_id(&avatar_id)?;
            }
        }
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

    std::fs::canonicalize(path.as_ref())
}

#[must_use]
pub fn parse_avatar_ids(path: &PathBuf) -> Vec<String> {
    static RE: Lazy<Regex> = lazy_regex!(r"avtr_\w{8}-\w{4}-\w{4}-\w{4}-\w{12}");

    let Ok(file) = File::open(path) else {
        return Vec::new(); // Directory
    };

    let mut reader = BufReader::new(file);
    let mut avatar_ids = Vec::new();
    let mut buf = Vec::new();

    while reader.read_until(b'\n', &mut buf).unwrap_or(0) > 0 {
        let line = String::from_utf8_lossy(&buf);
        for mat in RE.find_iter(&line) {
            avatar_ids.push(mat.as_str().to_string());
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
