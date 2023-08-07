use std::collections::HashMap;

pub mod prelude;
pub mod ravenwood;
pub mod sqlite;

pub trait Provider {
    fn send_avatar_id(&self, avatar_id: &str) -> anyhow::Result<bool>;
}

pub type Providers = HashMap<&'static str, Box<dyn Provider>>;

// https://stackoverflow.com/a/72239266
#[macro_export]
macro_rules! provider {
    ( $x:expr ) => {
        Box::new($x) as Box<dyn Provider>
    };
}
