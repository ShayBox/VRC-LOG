use indexmap::IndexMap;
use strum::Display;

pub mod prelude;
#[cfg(feature = "ravenwood")]
pub mod ravenwood;
#[cfg(feature = "sqlite")]
pub mod sqlite;

#[derive(Display, Eq, Hash, PartialEq)]
pub enum ProviderType {
    #[cfg(feature = "ravenwood")]
    Ravenwood,
    #[cfg(feature = "sqlite")]
    Sqlite,
}

pub trait Provider {
    fn send_avatar_id(&self, avatar_id: &str) -> anyhow::Result<bool>;
}

pub type Providers = IndexMap<ProviderType, Box<dyn Provider>>;

// https://stackoverflow.com/a/72239266
#[macro_export]
macro_rules! box_provider {
    ($x:expr) => {
        Box::new($x) as Box<dyn Provider>
    };
}
