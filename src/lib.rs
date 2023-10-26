use std::{
    fs::File,
    io::{Error, Read},
    path::{Path, PathBuf},
};

use colored::{Color, Colorize};
use crossbeam::channel::{Receiver, Sender};
use lazy_static::lazy_static;
use notify::{Config, Event, PollWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use regex::{Captures, Regex};

pub mod config;
pub mod provider;

pub fn watch<P: AsRef<Path>>(path: P) -> notify::Result<(Sender<PathBuf>, Receiver<PathBuf>)> {
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

    Ok((tx_a, rx_a))
}

pub fn parse_path_env(haystack: &str) -> Result<PathBuf, Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"%(\w+)%").unwrap();
    }

    let str = RE.replace_all(haystack, |captures: &Captures| {
        let key = &captures[1];
        std::env::var(key).unwrap()
    });
    let path = std::fs::canonicalize(str.as_ref())?;

    Ok(path)
}

pub fn parse_avatar_ids(path: PathBuf) -> Result<Vec<String>, Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"avtr_\w{8}-\w{4}-\w{4}-\w{4}-\w{12}").unwrap();
    }

    let mut file = File::open(path)?;
    let mut buf = vec![];
    file.read_to_end(&mut buf)?;

    let haystack = String::from_utf8_lossy(&buf);
    let avatar_ids = RE
        .find_iter(&haystack)
        .map(|m| m.as_str().into())
        .collect::<Vec<_>>();

    Ok(avatar_ids)
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
