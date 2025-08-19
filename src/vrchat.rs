use std::{
    fs::File,
    io::{Error, Read, Seek},
    path::PathBuf,
    sync::LazyLock,
};

use anyhow::Context;
use serde::{Deserialize, Deserializer, Serialize};

#[cfg(target_os = "windows")]
const AMP_PATH: &str = "%Temp%\\VRChat\\VRChat\\amplitude.cache";

#[cfg(target_os = "linux")]
const AMP_PATH: &str = "$HOME/.local/share/Steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/Local/Temp/VRChat/VRChat/amplitude.cache";

#[cfg(target_os = "windows")]
const LOW_PATH: &str = "%AppData%\\..\\LocalLow\\VRChat\\VRChat";

#[cfg(target_os = "linux")]
const LOW_PATH: &str = "$HOME/.local/share/Steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow/VRChat/VRChat";

pub static VRCHAT_AMP_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| crate::parse_path_env(AMP_PATH).expect("Failed to parse amplitude path"));

/// This is a static path and cannot be changed (without symlinks)
pub static VRCHAT_LOW_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| crate::parse_path_env(LOW_PATH).expect("Failed to parse local low path"));

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VRChat {
    /// This is actually the path to the cache parent directory
    /// `VRChat` doesn't allow you to change the cache directory name
    /// The `Cache-WindowsPlayer` path is appended during deserialization below
    /// Because this is how `VRChat` does it, it must not be in the config file
    #[serde(deserialize_with = "deserialize")]
    pub cache_directory: PathBuf,
}

/// Try to deserialize the `VRChat` `config.json` `cache_directory`, `parse_path_env`, and append `Cache-WindowsPlayer`
///
/// # Errors
/// Will return `Err` if `crate::parse_path_env` errors
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
    let haystack = String::deserialize(deserializer)?;
    let path = crate::parse_path_env(&haystack)
        .context("Failed to parse the default path")
        .map_err(serde::de::Error::custom)?
        .join("Cache-WindowsPlayer");

    Ok(path)
}

impl VRChat {
    #[must_use]
    pub fn get_path() -> PathBuf {
        VRCHAT_LOW_PATH.join("config.json")
    }

    /// Try to load the `VRChat` `config.json` file for the `cache_directory` field
    ///
    /// # Errors
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

        // Fallback to default below if config fails to deserialize
        serde_json::from_str(&text).map_or_else(|_| Ok(Self::default()), Ok)
    }
}

impl Default for VRChat {
    fn default() -> Self {
        Self {
            cache_directory: VRCHAT_LOW_PATH.join("Cache-WindowsPlayer"),
        }
    }
}
