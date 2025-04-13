use anyhow::Result;
use sqlite::{Connection, State};

use crate::{
    provider::{Provider, Type},
    vrchat::VRCHAT_LOW_PATH,
};

pub struct Cache {
    connection: Connection,
}

impl Cache {
    /// # Errors
    /// Will return `Err` if `sqlite::open` errors
    pub fn new() -> Result<Self> {
        let path = VRCHAT_LOW_PATH.join("avatars.sqlite");
        let connection = sqlite::open(path)?;
        let query = "CREATE TABLE avatars (
            id TEXT PRIMARY KEY,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )";

        // Create the table if it doesn't exist
        if connection.execute(query).is_err() {
            // Ensure `created_at` and `updated_at` columns exist
            let mut statement = connection.prepare("PRAGMA table_info(avatars)")?;
            let mut columns = Vec::new();
            while statement.next()? == State::Row {
                let column_name: String = statement.read(1)?;
                columns.push(column_name);
            }

            if !columns.contains(&"created_at".to_string()) {
                #[rustfmt::skip]
                connection.execute("
                    ALTER TABLE avatars
                    ADD COLUMN created_at DATETIME
                ")?;
            }

            #[rustfmt::skip]
            connection.execute("
                UPDATE avatars
                SET created_at = CURRENT_TIMESTAMP
                WHERE created_at IS NULL
            ")?;

            if !columns.contains(&"updated_at".to_string()) {
                #[rustfmt::skip]
                connection.execute("
                    ALTER TABLE avatars
                    ADD COLUMN updated_at DATETIME
                ")?;
            }

            #[rustfmt::skip] // Prevent a large burst after updating
            connection.execute("
                UPDATE avatars
                SET updated_at = datetime('now', '-31 days')
                WHERE updated_at IS NULL
            ")?;

            // Print cache statistics
            if let Ok(statement) = connection.prepare("SELECT * FROM avatars") {
                let rows = statement.into_iter().filter_map(Result::ok);
                info!("[{}] {} Cached Avatars", Type::CACHE, rows.count());
            }
        }

        Ok(Self { connection })
    }
}

impl Provider for Cache {
    fn check_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let query = "
            SELECT 1 FROM avatars
            WHERE id = (?) AND updated_at >= datetime('now', '-30 days')
            LIMIT 1
        ";

        let mut statement = self.connection.prepare(query)?;
        statement.bind((1, avatar_id))?;

        Ok(matches!(statement.next()?, State::Done))
    }

    fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let query = "
            INSERT INTO avatars (id, created_at, updated_at)
            VALUES (?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT (id) DO UPDATE SET updated_at = CURRENT_TIMESTAMP
        ";

        let mut statement = self.connection.prepare(query)?;
        statement.bind((1, avatar_id))?;

        Ok(statement.next().is_ok())
    }
}
