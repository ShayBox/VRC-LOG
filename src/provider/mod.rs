use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[cfg(feature = "avtrdb")]
pub mod avtrdb;
#[cfg(feature = "cache")]
pub mod cache;
#[cfg(feature = "paw")]
pub mod paw;
#[cfg(feature = "vrcdb")]
pub mod vrcdb;
#[cfg(feature = "vrcds")]
pub mod vrcds;
#[cfg(feature = "vrcwb")]
pub mod vrcwb;

pub mod prelude;

#[derive(EnumIter, Display, Deserialize, Serialize)]
pub enum ProviderKind {
    #[cfg(feature = "cache")]
    CACHE,
    #[cfg(feature = "avtrdb")]
    #[strum(to_string = "avtrDB - Avatar Search")]
    AVTRDB,
    #[cfg(feature = "paw")]
    #[strum(to_string = "PAW - Puppy's Avatar World")]
    PAW,
    #[cfg(feature = "vrcdb")]
    #[strum(to_string = "VRCDB - Avatar Search")]
    VRCDB,
    #[cfg(feature = "vrcds")]
    #[strum(to_string = "VRCDS - Project Dark Star")]
    VRCDS,
    #[cfg(feature = "vrcwb")]
    #[strum(to_string = "VRCWB - World Balancer")]
    VRCWB,
}

#[async_trait]
pub trait Provider {
    /// # Return the `ProviderKind`
    fn kind(&self) -> ProviderKind;

    /// # Check if the avatar ID is unique or not.
    /// True: New/Unique | False: Duplicate/Existing.
    ///
    /// # Errors
    /// Will return `Err` if anything errors
    async fn check_avatar_id(&self, avatar_id: &str) -> Result<bool>;

    /// # Send avatar ID to the provider
    /// True: New/Unique | False: Duplicate/Existing.
    ///
    /// # Errors
    /// Will return `Err` if anything errors
    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool>;

    // TODO: Add send_avatar_ids for batching support, with fallback iteration to send_avatar_id.
}

// https://stackoverflow.com/a/72239266
#[macro_export]
macro_rules! provider {
    ($x:expr) => {
        Some(Box::new($x) as Box<dyn $crate::provider::Provider>)
    };
}
