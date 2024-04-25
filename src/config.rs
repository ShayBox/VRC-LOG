use std::{
    fs::File,
    io::{Error, Read},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

#[cfg(target_os = "windows")]
const DEFAULT: &str = "%AppData%\\..\\LocalLow\\VRChat\\VRChat";
#[cfg(target_os = "linux")]
const DEFAULT: &str = "%HOME%/.local/share/Steam/steamapps/compatdata/438100/pfx/drive_c/users/steamuser/AppData/LocalLow\\VRChat\\VRChat";

lazy_static::lazy_static! {
    pub static ref DEFAULT_PATH: PathBuf = crate::parse_path_env(DEFAULT).unwrap();
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VRChat {
    #[serde(default = "default_cache_directory")]
    pub cache_directory: PathBuf,
}

impl Default for VRChat {
    fn default() -> Self {
        Self {
            cache_directory: default_cache_directory(),
        }
    }
}

impl VRChat {
    #[must_use]
    pub fn get_path() -> PathBuf {
        DEFAULT_PATH.join("config.json")
    }

    pub fn load() -> Result<Self, Error> {
        let path = Self::get_path();
        let file = File::options().read(true).open(path);

        let mut config = match file {
            Ok(mut f) => {
                let mut text = String::new();
                f.read_to_string(&mut text)?;
                serde_json::from_str(&text).unwrap_or_else(|_| VRChat::default())
            }
            Err(_) => VRChat::default(),
        };

        config.cache_directory = config.cache_directory.join("Cache-WindowsPlayer");
        Ok(config)
    }
}

fn default_cache_directory() -> PathBuf {
    DEFAULT_PATH.clone()
}
