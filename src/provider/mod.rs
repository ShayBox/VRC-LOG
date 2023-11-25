use indexmap::IndexMap;
use strum::Display;

pub mod prelude;
#[cfg(feature = "ravenwood")]
pub mod ravenwood;
#[cfg(feature = "sqlite")]
pub mod sqlite;

#[derive(Display, Eq, Hash, PartialEq)]
pub enum Type {
    #[cfg(feature = "cache")]
    Cache,
    #[cfg(feature = "ravenwood")]
    Ravenwood,
    #[cfg(feature = "sqlite")]
    Sqlite,
}

pub trait Provider {
    /// True: New/Unique | False: Duplicate/Existing
    ///
    /// # Errors
    ///
    /// Will return `Err` if anything errors
    fn check_avatar_id(&self, avatar_id: &str) -> anyhow::Result<bool>;

    /// True: New/Unique | False: Duplicate/Existing
    ///
    /// # Errors
    ///
    /// Will return `Err` if anything errors
    fn send_avatar_id(&self, avatar_id: &str) -> anyhow::Result<bool>;
}

pub type Providers = IndexMap<Type, Box<dyn Provider>>;

// https://stackoverflow.com/a/72239266
#[macro_export]
macro_rules! box_db {
    ($x:expr) => {
        Box::new($x) as Box<dyn $crate::provider::Provider>
    };
}
