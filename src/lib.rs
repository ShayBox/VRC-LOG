use std::{
    fs::File,
    io::{Error, Read},
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
};

use lazy_static::lazy_static;
use notify::{Config, Event, PollWatcher, RecursiveMode, Watcher};
use regex::{Captures, Regex};

pub mod config;
pub mod provider;

pub fn watch<P: AsRef<Path>>(path: P) -> notify::Result<Receiver<PathBuf>> {
    let (tx_a, tx_b, rx) = {
        let (tx, rx) = std::sync::mpsc::channel();
        (tx.clone(), tx, rx)
    };

    let mut watcher = PollWatcher::with_initial_scan(
        move |watch_event: notify::Result<Event>| {
            let event = watch_event.unwrap();
            for path in event.paths {
                tx_a.send(path).unwrap();
            }
        },
        Config::default(),
        move |scan_event: notify::Result<PathBuf>| {
            let path = scan_event.unwrap();
            tx_b.send(path).unwrap();
        },
    )?;

    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    Ok(rx)
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
