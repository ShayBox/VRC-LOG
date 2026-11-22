#[macro_use]
extern crate tracing;

use std::{
    collections::HashSet,
    fmt::Display,
    fs::{File, create_dir_all},
    io::{BufRead, BufReader, Error},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, LazyLock},
    time::Duration,
};

use anyhow::Result;
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
use crate::{provider::Provider, settings::Settings};

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

const MAX_TIME_SINCE_MODIFY: Duration = Duration::from_hours(48); // 2 days

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

fn process_paths(
    filter: fn(&Path) -> bool,
    paths: impl IntoIterator<Item = PathBuf>,
    tx: &Sender<PathBuf>,
) {
    for p in paths
        .into_iter()
        .filter(|p| p.exists() && !p.is_dir() && filter(p))
    {
        if let Ok(meta) = p.metadata()
            && let Ok(modified) = meta.modified()
        {
            let time_since_last_modified = modified.elapsed().unwrap_or_default();
            if time_since_last_modified < MAX_TIME_SINCE_MODIFY {
                let _ = tx.send(p);
            }
        } else {
            debug!("did not process path: {:?}", p);
        }
    }
}

/// # Errors
/// Will return `Err` if `PollWatcher::watch` errors
pub fn watch<P: AsRef<Path>>(
    tx: &Sender<PathBuf>,
    path: P,
    millis: u64,
    filter: fn(&Path) -> bool,
) -> notify::Result<PollWatcher> {
    let path = path.as_ref();
    debug!("Watching {path:?}");

    let watch_tx = tx.clone();
    let scan_tx = tx.clone();

    let mut watcher = PollWatcher::with_initial_scan(
        move |watch_event: notify::Result<Event>| match watch_event {
            Ok(event) => {
                process_paths(filter, event.paths, &watch_tx);
            }
            Err(err) => {
                error!("failed to process watch event: {:?}", err);
            }
        },
        Config::default()
            .with_compare_contents(true)
            .with_poll_interval(Duration::from_millis(millis)),
        move |scan_event: notify::Result<PathBuf>| match scan_event {
            Ok(path) => process_paths(filter, vec![path], &scan_tx),
            Err(err) => {
                error!("failed to process scan event: {:?}", err);
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
    settings: &Settings,
    (_tx, rx): (Sender<PathBuf>, Receiver<PathBuf>),
) -> Result<()> {
    #[cfg(feature = "cache")]
    let cache = cache::Cache::new().await?;

    while let Ok(path) = rx.recv_async().await {
        if !path.is_file() {
            continue;
        }
        let avatar_ids = match parse_avatar_ids(&path) {
            Ok(ids) => ids,
            Err(error) => {
                error!(
                    "failed to parse avatar ids for path {:?} Error: {:?}",
                    path, error
                );
                continue;
            }
        };

        // Clear amplitude file after reading if enabled and it's an amplitude file
        if settings.clear_amplitude
            && path.file_name().and_then(|n| n.to_str()) == Some("amplitude.cache")
        {
            match std::fs::write(&path, "") {
                Ok(()) => debug!("Cleared amplitude file: {path:?}"),
                Err(error) => warn!("Failed to clear amplitude file: {error}"),
            }
        }

        #[cfg(feature = "cache")]
        process_with_cache(
            providers.clone(),
            settings.print_scanned,
            &cache,
            avatar_ids,
        )
        .await?;
        #[cfg(not(feature = "cache"))]
        process_without_cache(providers.clone(), settings.print_scanned, avatar_ids).await?;
    }

    debug!("Channel closed, stopping avatar processing");
    Ok(())
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

/// # Errors
///
/// Returns an [`std::io::Error`] if
/// opening `path` or reading from the underlying file fails.
pub fn parse_avatar_ids(path: &Path) -> Result<impl IntoIterator<Item = String>> {
    #[allow(clippy::non_std_lazy_statics)]
    static RE: Lazy<Regex> = lazy_regex!(r"avtr_\w{8}-\w{4}-\w{4}-\w{4}-\w{12}");

    let file = File::open(path)?;

    let mut reader = BufReader::new(file);
    let mut avatar_ids = HashSet::new(); // Filter out duplicates
    let mut buf = Vec::new();

    while reader.read_until(b'\n', &mut buf)? > 0 {
        let line = String::from_utf8_lossy(&buf);
        for mat in RE.find_iter(&line) {
            avatar_ids.insert(mat.as_str().to_string());
        }
        buf.clear();
    }

    Ok(avatar_ids)
}

/// # Print with colorized rainbow rows for separation
pub fn print_colorized(avatar_id: impl Display) {
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

    let mut idx = INDEX.write();
    let color = COLORS[*idx];
    *idx = (*idx + 1) % COLORS.len();
    drop(idx);

    let text = format!("vrcx://avatar/{avatar_id}");
    let link = Link::new(&text, &text).to_string().color(color);
    info!("{link}");
}

// These tests where generated by Claude Opus 5.5
#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use tempfile::TempDir;

    use super::*;

    const NONE: [PathBuf; 0] = [];
    const HOUR: Duration = Duration::from_hours(1);

    /// Creates `dir/name` with its modified time set `age` in the past.
    fn file_with_age(dir: &Path, name: &str, age: Duration) -> PathBuf {
        let path = dir.join(name);
        let file = File::create(&path).unwrap();
        file.set_modified(SystemTime::now() - age).unwrap();
        path
    }

    /// Runs `process_paths` and returns everything it sent.
    fn run(filter: fn(&Path) -> bool, paths: Vec<PathBuf>) -> Vec<PathBuf> {
        let (tx, rx) = flume::unbounded();
        process_paths(filter, paths, &tx);
        drop(tx);
        rx.drain().collect()
    }

    fn allow_all(_: &Path) -> bool {
        true
    }

    fn is_txt(p: &Path) -> bool {
        p.extension().is_some_and(|e| e == "txt")
    }

    #[test]
    fn sends_recent_file() {
        let dir = TempDir::new().unwrap();
        let file = file_with_age(dir.path(), "a.txt", Duration::ZERO);
        assert_eq!(run(allow_all, vec![file.clone()]), vec![file]);
    }

    #[test]
    fn skips_file_older_than_max_age() {
        let dir = TempDir::new().unwrap();
        let file = file_with_age(dir.path(), "old.txt", MAX_TIME_SINCE_MODIFY + HOUR);
        assert_eq!(run(allow_all, vec![file]), NONE);
    }

    #[test]
    fn sends_file_just_under_max_age() {
        let dir = TempDir::new().unwrap();
        let age = MAX_TIME_SINCE_MODIFY.saturating_sub(HOUR);
        let file = file_with_age(dir.path(), "edge.txt", age);
        assert_eq!(run(allow_all, vec![file.clone()]), vec![file]);
    }

    #[test]
    fn skips_directories() {
        let dir = TempDir::new().unwrap();
        assert_eq!(run(allow_all, vec![dir.path().to_path_buf()]), NONE);
    }

    #[test]
    fn skips_missing_paths() {
        let dir = TempDir::new().unwrap();
        assert_eq!(run(allow_all, vec![dir.path().join("nope.txt")]), NONE);
    }

    #[test]
    fn skips_paths_rejected_by_filter() {
        let dir = TempDir::new().unwrap();
        let file = file_with_age(dir.path(), "a.log", Duration::ZERO);
        assert_eq!(run(is_txt, vec![file]), NONE);
    }

    #[test]
    fn filters_mixed_input_and_keeps_order() {
        let dir = TempDir::new().unwrap();
        let first = file_with_age(dir.path(), "1.txt", Duration::ZERO);
        let wrong_ext = file_with_age(dir.path(), "2.log", Duration::ZERO);
        let stale = file_with_age(dir.path(), "3.txt", MAX_TIME_SINCE_MODIFY + HOUR);
        let second = file_with_age(dir.path(), "4.txt", HOUR);
        let missing = dir.path().join("5.txt");

        let sent = run(
            is_txt,
            vec![
                first.clone(),
                wrong_ext,
                stale,
                dir.path().to_path_buf(),
                second.clone(),
                missing,
            ],
        );
        assert_eq!(sent, vec![first, second]);
    }

    #[test]
    fn empty_input_sends_nothing() {
        assert_eq!(run(allow_all, Vec::new()), NONE);
    }
}
