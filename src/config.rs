use std::{
    fs::File,
    io::{Error, Read, Seek},
    path::PathBuf,
    str,
};

use lazy_static::lazy_static;
use serde::{Deserialize, Deserializer, Serialize};

const DEFAULT: &str = "%AppData%\\..\\LocalLow\\VRChat\\VRChat";

lazy_static! {
    pub static ref DEFAULT_PATH: PathBuf = crate::parse_path_env(DEFAULT).unwrap();
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VRChatConfig {
    #[serde(deserialize_with = "deserialize")]
    pub cache_directory: PathBuf,
}

impl Default for VRChatConfig {
    fn default() -> Self {
        Self {
            cache_directory: DEFAULT_PATH.join("Cache-WindowsPlayer"),
        }
    }
}

impl VRChatConfig {
    pub fn get_path() -> PathBuf {
        DEFAULT_PATH.join("config.json")
    }

    pub fn load() -> Result<Self, Error> {
        let path = Self::get_path();
        let mut file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let mut text = String::new();
        file.read_to_string(&mut text)?;
        file.rewind()?;

        match serde_json::from_str(&text) {
            Ok(config) => Ok(config),
            Err(_) => Ok(VRChatConfig::default()),
        }
    }
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
    let str = Deserialize::deserialize(deserializer).unwrap_or(DEFAULT);
    let path = crate::parse_path_env(str)
        .expect("Failed to parse the default path")
        .join("Cache-WindowsPlayer");

    Ok(path)
}
