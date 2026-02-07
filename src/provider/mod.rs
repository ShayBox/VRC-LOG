use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

#[cfg(feature = "avtrdb")]
pub mod avtrdb;
#[cfg(feature = "nsvr")]
pub mod nsvr;
#[cfg(feature = "paw")]
pub mod paw;
#[cfg(feature = "vrcdb")]
pub mod vrcdb;
#[cfg(feature = "vrcwb")]
pub mod vrcwb;

pub mod prelude;

#[derive(EnumIter, Display, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, PartialOrd)]
#[repr(u32)]
pub enum ProviderKind {
    #[cfg(feature = "avtrdb")]
    #[strum(to_string = "avtrDB - Avatar Search")]
    AVTRDB = 1 << 0,
    #[cfg(feature = "nsvr")]
    #[strum(to_string = "NSVR - NekoSune Community")]
    #[serde(alias = "VRCDS")]
    NSVR   = 1 << 1,
    #[cfg(feature = "paw")]
    #[strum(to_string = "PAW - Puppy's Avatar World")]
    PAW    = 1 << 2,
    #[cfg(feature = "vrcdb")]
    #[strum(to_string = "VRCDB - Avatar Search")]
    VRCDB  = 1 << 3,
    #[cfg(feature = "vrcwb")]
    #[strum(to_string = "VRCWB - World Balancer")]
    VRCWB  = 1 << 4,
}

#[async_trait]
pub trait Provider: Sync + Send {
    /// # Return the `ProviderKind`
    fn kind(&self) -> ProviderKind;

    /// # Send avatar ID to the provider
    ///
    /// True: New/Unique | False: Duplicate/Existing.
    ///
    /// # Errors
    /// Will return `Err` if anything errors
    async fn send_avatar_id(&self, avatar_id: &str) -> anyhow::Result<bool>;
}

// https://stackoverflow.com/a/72239266
#[macro_export]
macro_rules! provider {
    ($x:expr) => {
        std::sync::Arc::new(Box::new($x) as Box<dyn $crate::provider::Provider>)
    };
}
