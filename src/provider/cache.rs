use anyhow::Result;
use async_trait::async_trait;
use tokio_rusqlite_new::{Connection, Error};

use crate::{
    provider::{Provider, ProviderKind},
    vrchat::VRCHAT_LOW_PATH,
};

pub struct Cache {
    connection: Connection,
}

impl Cache {
    /// # Errors
    /// Will return `Err` if `sqlite::open` errors
    pub async fn new() -> Result<Self> {
        debug!("Trying to open SQLite cache database.");
        let path = VRCHAT_LOW_PATH.join("avatars.sqlite");
        let connection = Connection::open(path).await?;

        connection
            .call(|connection| {
                let query = "CREATE TABLE avatars (
                    id TEXT PRIMARY KEY,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
                )";

                debug!("Trying to create avatars table...");
                if connection.execute(query, []).is_err() {
                    debug!("The avatars table already exists.");

                    let mut statement = connection.prepare("PRAGMA table_info(avatars)")?;
                    let columns = statement
                        .query_map([], |row| row.get::<_, String>(1))?
                        .collect::<Result<Vec<_>, _>>()?;

                    if !columns.contains(&"created_at".to_string()) {
                        debug!("Trying to create the created_at column.");
                        #[rustfmt::skip]
                        connection.execute("
                            ALTER TABLE avatars
                            ADD COLUMN created_at DATETIME
                        ", [])?;
                    }

                    debug!("Updating all rows with missing created_at");
                    #[rustfmt::skip]
                    connection.execute("
                        UPDATE avatars
                        SET created_at = CURRENT_TIMESTAMP
                        WHERE created_at IS NULL
                    ", [])?;

                    if !columns.contains(&"updated_at".to_string()) {
                        debug!("Trying to create the updated_at column.");
                        #[rustfmt::skip]
                        connection.execute("
                            ALTER TABLE avatars
                            ADD COLUMN updated_at DATETIME
                        ", [])?;
                    }

                    debug!("Updating all rows with missing updated_at");
                    #[rustfmt::skip]
                    connection.execute("
                        UPDATE avatars
                        SET updated_at = datetime('now', '-31 days')
                        WHERE updated_at IS NULL
                    ", [])?;
                }

                debug!("Trying to create an updated_at index.");
                #[rustfmt::skip] // Speed up queries on large databases
                connection.execute("
                    CREATE INDEX IF NOT EXISTS idx_avatars_updated_at
                    ON avatars(updated_at)
                ", [])?;

                if let Ok(mut statement) = connection.prepare("SELECT COUNT(*) FROM avatars") {
                    if let Ok(count) = statement.query_row([], |row| row.get::<_, i64>(0)) {
                        info!("{} Cached Avatars", count);
                    }
                }

                Ok::<(), Error>(())
            })
            .await?;

        Ok(Self { connection })
    }
}

#[async_trait]
impl Provider for Cache {
    fn kind(&self) -> ProviderKind {
        ProviderKind::CACHE
    }

    async fn check_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let kind = self.kind();
        let id = avatar_id.to_string();
        let query = "
            SELECT EXISTS(
                SELECT 1 FROM avatars
                WHERE id = ? AND updated_at >= datetime('now', '-30 days')
            )
        ";

        let exists = self
            .connection
            .call(move |c| {
                let mut stmt = c.prepare(query)?;
                let result: i64 = stmt.query_row([id], |row| row.get(0))?;
                Ok::<bool, Error>(result == 1)
            })
            .await?;

        trace!("[{kind}] Checking {avatar_id} | Exists: {exists}");
        Ok(!exists)
    }

    async fn send_avatar_id(&self, avatar_id: &str) -> Result<bool> {
        let id = avatar_id.to_string();
        let query = "
            INSERT INTO avatars (id, created_at, updated_at)
            VALUES (?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT (id) DO UPDATE SET updated_at = CURRENT_TIMESTAMP
        ";

        let is_ok = self
            .connection
            .call(move |c| Ok::<bool, Error>(c.execute(query, [id]).is_ok()))
            .await?;

        Ok(is_ok)
    }
}
