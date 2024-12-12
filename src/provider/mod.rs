use anyhow::Result;
use indexmap::IndexMap;
use strum::Display;

#[cfg(feature = "avtrdb")]
pub mod avtrdb;
#[cfg(feature = "cache")]
pub mod cache;
#[cfg(feature = "vrcdb")]
pub mod vrcdb;
#[cfg(feature = "vrcds")]
pub mod vrcds;
#[cfg(feature = "vrcwb")]
pub mod vrcwb;

pub mod prelude;

#[derive(Display, Eq, Hash, PartialEq)]
pub enum Type {
    #[cfg(feature = "cache")]
    CACHE,
    #[cfg(feature = "avtrdb")]
    #[strum(to_string = "avtrDB - Avatar Search")]
    AVTRDB,
    #[cfg(feature = "vrcdb")]
    #[strum(to_string = "VRCDB - Avatar Search")]
    VRCDB,
    #[cfg(feature = "vrcds")]
    #[strum(to_string = "VRCLogger - Project Dark Star")]
    VRCDS,
    #[cfg(feature = "vrcwb")]
    #[strum(to_string = "VRCWB - World Balancer")]
    VRCWB,
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
