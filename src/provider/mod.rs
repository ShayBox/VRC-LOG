use indexmap::IndexMap;
use strum::Display;

pub mod prelude;
#[cfg(feature = "ravenwood")]
pub mod ravenwood;
#[cfg(feature = "sqlite")]
pub mod sqlite;

#[derive(Display, Eq, Hash, PartialEq)]
pub enum ProviderType {
    #[cfg(feature = "cache")]
    Cache,
    #[cfg(feature = "ravenwood")]
    Ravenwood,
    #[cfg(feature = "sqlite")]
    Sqlite,
}

pub trait Provider {
    /// True: New/Unique | False: Duplicate/Existing
    fn send_avatar_id(&self, avatar_id: &str) -> anyhow::Result<bool>;
}

pub type ProviderBox = Box<dyn Provider>;
pub type Providers = IndexMap<ProviderType, ProviderBox>;

// https://stackoverflow.com/a/72239266
#[macro_export]
macro_rules! box_provider {
    ($x:expr) => {
        Box::new($x) as $crate::provider::ProviderBox
    };
}
