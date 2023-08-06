use sqlite::Connection;

use crate::{config::DEFAULT_PATH, provider::Provider};

pub struct Sqlite {
    connection: Connection,
}

impl Sqlite {
    pub fn new() -> anyhow::Result<Self> {
        let path = DEFAULT_PATH.join("avatars.sqlite");
        let connection = sqlite::open(path)?;

        // Create the table if it doesn't exist
        let query = "CREATE TABLE avatars (id TEXT PRIMARY KEY)";
        let _ = connection.execute(query);

        Ok(Self { connection })
    }
}

impl Provider for Sqlite {
    fn send_avatar_id(&self, avatar_id: &str) -> anyhow::Result<bool> {
        let query = "INSERT INTO avatars VALUES (?)";
        let mut statement = self.connection.prepare(query)?;
        statement.bind((1, avatar_id))?;

        Ok(statement.next().is_ok())
    }
}
