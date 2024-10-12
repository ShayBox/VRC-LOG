use anyhow::Result;
use indexmap::IndexMap;
use strum::Display;

#[cfg(feature = "avtrdb")]
pub mod avtrdb;
#[cfg(feature = "cache")]
pub mod cache;
#[cfg(feature = "doughnut")]
pub mod doughnut;
#[cfg(feature = "neko")]
pub mod neko;
#[cfg(feature = "vrcdb")]
pub mod vrcdb;

pub mod prelude;

#[derive(Display, Eq, Hash, PartialEq)]
pub enum Type {
    #[cfg(feature = "avtrdb")]
    AVTRDB,
    #[cfg(feature = "cache")]
    CACHE,
    #[cfg(feature = "doughnut")]
    DOUGHNUT,
    #[cfg(feature = "neko")]
    NEKO,
    #[cfg(feature = "vrcdb")]
    VRCDB,
}

pub trait Provider {
    /// # Check if the avatar ID is unique or not (Cache Only)
    /// True: New/Unique | False: Duplicate/Existing
    ///
    /// # Errors
    /// Will return `Err` if anything errors
    fn check_avatar_id(&self, avatar_id: &str) -> Result<bool>;

    /// # Send avatar ID to the provider
    /// True: New/Unique | False: Duplicate/Existing
    ///
    /// # Errors
    /// Will return `Err` if anything errors
    fn send_avatar_id(&self, avatar_id: &str) -> Result<bool>;
}

pub type Providers = IndexMap<Type, Box<dyn Provider>>;

// https://stackoverflow.com/a/72239266
#[macro_export]
macro_rules! box_db {
    ($x:expr) => {
        Box::new($x) as Box<dyn $crate::provider::Provider>
    };
}
