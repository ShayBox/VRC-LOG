use anyhow::Result;
use sqlite::Connection;

use crate::{config::DEFAULT_PATH, provider::Provider};

pub struct Sqlite {
    connection: Connection,
}

impl Sqlite {
    /// # Errors
    ///
    /// Will return `Err` if `sqlite::open` errors
    pub fn new() -> Result<Self> {
        let path = DEFAULT_PATH.join("avatars.sqlite");
        let connection = sqlite::open(path)?;

        // Create the table if it doesn't exist
        let query = "CREATE TABLE avatars (id TEXT PRIMARY KEY)";
        let _ = connection.execute(query);

        Ok(Self { connection })
    }
}

impl Provider for Sqlite {
    fn check_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let query = "SELECT 1 FROM avatars WHERE id = (?) LIMIT 1";
        let mut statement = self.connection.prepare(query)?;
        statement.bind((1, avatar_id))?;

        Ok(matches!(statement.next()?, sqlite::State::Done))
    }

    fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let query = "INSERT INTO avatars VALUES (?)";
        let mut statement = self.connection.prepare(query)?;
        statement.bind((1, avatar_id))?;

        Ok(statement.next().is_ok())
    }
}
