use anyhow::Result;
use async_trait::async_trait;
use strum::Display;

use crate::provider::prelude::*;

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
#[cfg(feature = "paw")]
pub mod paw;

pub mod prelude;

#[derive(Display)]
pub enum Type<'a> {
    #[cfg(feature = "cache")]
    CACHE(&'a Cache),
    #[cfg(feature = "avtrdb")]
    #[strum(to_string = "avtrDB - Avatar Search")]
    AVTRDB(&'a AvtrDB),
    #[cfg(feature = "vrcdb")]
    #[strum(to_string = "VRCDB - Avatar Search")]
    VRCDB(&'a VrcDB),
    #[cfg(feature = "vrcds")]
    #[strum(to_string = "VRCDS - Project Dark Star")]
    VRCDS(&'a VrcDS),
    #[cfg(feature = "vrcwb")]
    #[strum(to_string = "VRCWB - World Balancer")]
    VRCWB(&'a VrcWB),
    #[cfg(feature = "paw")]
    #[strum(to_string = "PAW - Puppy's Avatar World")]
    PAW(&'a Paw),
}

#[async_trait]
pub trait Provider {
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
}

#[async_trait]
impl Provider for Type<'_> {
    async fn check_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        match self {
            #[cfg(feature = "cache")]
            Type::CACHE(p) => p.check_avatar_id(avatar_id).await,
            _ => Ok(true),
        }
    }

    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        match self {
            #[cfg(feature = "cache")]
            Type::CACHE(p) => p.send_avatar_id(avatar_id).await,
            #[cfg(feature = "avtrdb")]
            Type::AVTRDB(p) => p.send_avatar_id(avatar_id).await,
            #[cfg(feature = "vrcdb")]
            Type::VRCDB(p) => p.send_avatar_id(avatar_id).await,
            #[cfg(feature = "vrcds")]
            Type::VRCDS(p) => p.send_avatar_id(avatar_id).await,
            #[cfg(feature = "vrcwb")]
            Type::VRCWB(p) => p.send_avatar_id(avatar_id).await,
            #[cfg(feature = "paw")]
            Type::PAW(p) => p.send_avatar_id(avatar_id).await,
        }
    }
}
