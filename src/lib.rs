use std::{
    env::Args,
    fs::File,
    io::{BufRead, BufReader, Error},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use anyhow::{bail, Context};
use chrono::Local;
use colored::{Color, Colorize};
use crossbeam::channel::{Receiver, Sender};
use lazy_static::lazy_static;
use notify::{Config, Event, PollWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use regex::{Captures, Regex};

use crate::provider::{prelude::*, Providers, Type};

#[cfg(feature = "discord")]
pub mod discord;
pub mod provider;
pub mod vrchat;

pub const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CARGO_PKG_HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");

pub type WatchResponse = (Sender<PathBuf>, Receiver<PathBuf>, PollWatcher);

#[must_use]
pub fn get_local_time() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// # Errors
///
/// Will return `Err` if couldn't get the GitHub repository
pub fn check_for_updates() -> reqwest::Result<bool> {
    let response = reqwest::blocking::get(CARGO_PKG_HOMEPAGE)?;
    let Some(segments) = response.url().path_segments() else {
        return Ok(false);
    };

    let Some(remote_version) = segments.last() else {
        return Ok(false);
    };

    Ok(remote_version > CARGO_PKG_VERSION)
}

/// # Errors
///
/// Will return `Err` if `PollWatcher::watch` errors
pub fn watch<P: AsRef<Path>>(path: P) -> notify::Result<WatchResponse> {
    let (tx_a, rx_a) = crossbeam::channel::unbounded();
    let (tx_b, tx_c) = (tx_a.clone(), tx_a.clone());

    let mut watcher = PollWatcher::with_initial_scan(
        move |watch_event: notify::Result<Event>| {
            if let Ok(event) = watch_event {
                for path in event.paths {
                    let _ = tx_c.send(path);
                }
            }
        },
        Config::default(),
        move |scan_event: notify::Result<PathBuf>| {
            if let Ok(path) = scan_event {
                let _ = tx_b.send(path);
            }
        },
    )?;

    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    Ok((tx_a, rx_a, watcher))
}

/// Steam Game Launch Options: `.../vrc-log(.exe) %command%`
///
/// # Errors
///
/// Will return `Err` if `Command::spawn` errors
///
/// # Panics
///
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
///
/// Will return `Err` if `Sqlite::new` or `Provider::send_avatar_id` errors
pub fn process_avatars((_tx, rx, _): WatchResponse) -> anyhow::Result<()> {
    #[cfg_attr(not(feature = "cache"), allow(unused_mut))]
    let mut providers = Providers::from([
        #[cfg(all(feature = "cache", feature = "sqlite"))]
        (Type::Cache, box_db!(Sqlite::new()?)),
        #[cfg(feature = "vrcdb")]
        (Type::VRCDB, box_db!(VRCDB::default())),
        #[cfg(all(feature = "sqlite", not(feature = "cache")))]
        (Type::Sqlite, box_db!(Sqlite::new()?)),
    ]);

    #[cfg(feature = "cache")]
    let cache = providers.shift_remove(&Type::Cache).context("None")?;

    while let Ok(path) = rx.recv() {
        let avatar_ids = self::parse_avatar_ids(&path);
        for avatar_id in avatar_ids {
            #[cfg(feature = "cache")] // Avatar is already in cache
            if !cache.check_avatar_id(&avatar_id).unwrap_or(true) {
                continue;
            };

            #[cfg(feature = "cache")] // Don't send to cache if sending failed
            let mut send_to_cache = true;
            let local_time = self::get_local_time();

            self::print_colorized(&avatar_id);
            std::thread::sleep(Duration::from_secs(3));

            for (provider_type, provider) in &providers {
                match provider.send_avatar_id(&avatar_id) {
                    Ok(unique) => {
                        if unique {
                            println!("^ {local_time}: Successfully Submitted to {provider_type}");
                        }
                    }
                    Err(error) => {
                        send_to_cache = false;
                        eprintln!("^ {local_time}: Failed to submit to {provider_type}: {error}");
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
///
/// Will return `Err` if `std::fs::canonicalize` errors
///
/// # Panics
///
/// Will panic if an environment variable doesn't exist
pub fn parse_path_env(haystack: &str) -> Result<PathBuf, Error> {
    lazy_static! { // This is the best regex I could come up with
        static ref RE: Regex = Regex::new(r"(\$|%)(\w+)%?").unwrap();
    }

    let str = RE.replace_all(haystack, |captures: &Captures| {
        let key = &captures[2];
        std::env::var(key).unwrap_or_else(|_| panic!("Environment Variable not found: {key}"))
    });
    let path = std::fs::canonicalize(str.as_ref())?;

    Ok(path)
}

#[must_use]
pub fn parse_avatar_ids(path: &PathBuf) -> Vec<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"avtr_\w{8}-\w{4}-\w{4}-\w{4}-\w{12}").unwrap();
    }

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

pub fn print_colorized(avatar_id: &str) {
    lazy_static! {
        static ref INDEX: RwLock<usize> = RwLock::new(0);
        static ref COLORS: [Color; 12] = [
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
        ];
    }

    let index = *INDEX.read();
    let color = COLORS[index];
    *INDEX.write() = (index + 1) % COLORS.len();

    let text = format!("vrcx://avatar/{avatar_id}").color(color);
    println!("{text}");
}
