use std::{
    fs::File,
    io::{Error, Read, Seek},
    path::PathBuf,
};

use serde::{Deserialize, Deserializer, Serialize};

const DEFAULT: &str = "%AppData%\\..\\LocalLow\\VRChat\\VRChat";

lazy_static::lazy_static! {
    pub static ref DEFAULT_PATH: PathBuf = crate::parse_path_env(DEFAULT).unwrap();
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VRChat {
    #[serde(deserialize_with = "deserialize")]
    pub cache_directory: PathBuf,
}

impl Default for VRChat {
    fn default() -> Self {
        Self {
            cache_directory: DEFAULT_PATH.join("Cache-WindowsPlayer"),
        }
    }
}

impl VRChat {
    #[must_use]
    pub fn get_path() -> PathBuf {
        DEFAULT_PATH.join("config.json")
    }

    /// # Errors
    ///
    /// Will return `Err` if `File::open`, `File::read_to_string`, or `File::rewind` errors
    pub fn load() -> Result<Self, Error> {
        let path = Self::get_path();
        let mut file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        let mut text = String::new();
        file.read_to_string(&mut text)?;
        file.rewind()?;

        serde_json::from_str(&text).map_or_else(|_| Ok(Self::default()), Ok)
    }
}

/// # Errors
///
/// Will never return `Err`
///
/// # Panics
///
/// Will panic if `crate::parse_path_env` errors
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
    let str = Deserialize::deserialize(deserializer).unwrap_or(DEFAULT);
    let path = crate::parse_path_env(str)
        .expect("Failed to parse the default path")
        .join("Cache-WindowsPlayer");

    Ok(path)
}
