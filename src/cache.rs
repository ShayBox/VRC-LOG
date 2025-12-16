use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use itertools::Itertools;
use rusqlite::{Connection as RusqliteConnection, named_params, params_from_iter};
use tokio_rusqlite_new::Connection;

use crate::vrchat::VRCHAT_LOW_PATH;

pub struct Cache {
    connection: Connection,
}

pub type AvatarIDWithProvider<S> = (S, u32);

impl Cache {
    /// # Errors
    /// Will return `Err` if `sqlite::open` errors
    pub async fn new() -> Result<Self> {
        debug!("Trying to open SQLite cache database.");
        Self::new_at_location(&VRCHAT_LOW_PATH.join("avatars.sqlite")).await
    }

    /// # Errors
    /// Will return `Err` if `sqlite::open` errors
    pub async fn new_at_location(path: &PathBuf) -> Result<Self> {
        debug!("Trying to open SQLite cache database.");
        let connection = Connection::open(path).await?;

        connection
            .call(|connection| Self::setup_database(connection))
            .await?;

        Ok(Self { connection })
    }

    /// # Errors
    /// Will return `Err` if `sqlite::open` errors
    pub async fn new_in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory().await?;
        connection
            .call(|connection| Self::setup_database(connection))
            .await?;
        Ok(Self { connection })
    }

    fn setup_database(connection: &RusqliteConnection) -> Result<(), rusqlite::Error> {
        let query = "CREATE TABLE avatars (
                    id TEXT PRIMARY KEY,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    provider_bits INT DEFAULT 0
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
            if !columns.contains(&"provider_bits".to_string()) {
                debug!("Trying to create the provider_bits column.");
                #[rustfmt::skip]
                        connection.execute("
                            ALTER TABLE avatars
                            ADD COLUMN provider_bits INT DEFAULT 0
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

        // Speed up queries on large databases
        debug!("Trying to create an updated_at index.");
        #[rustfmt::skip]
                connection.execute("
                    CREATE INDEX IF NOT EXISTS idx_avatars_updated_at
                    ON avatars(updated_at)
                ", [])?;

        debug!("Trying to create an id index.");
        #[rustfmt::skip]
                connection.execute(
                    "
                    CREATE INDEX IF NOT EXISTS idx_avatars_id
                    ON avatars(id)
                ", [])?;

        if let Ok(mut statement) = connection.prepare("SELECT COUNT(*) FROM avatars")
            && let Ok(count) = statement.query_row([], |row| row.get::<_, i64>(0))
        {
            info!("{} Cached Avatars", count);
        }

        Ok(())
    }

    /// # Errors
    /// Will return `Err` if `Connection::call(...)` errors
    pub async fn store_avatar_ids_with_providers<
        S: ToString,
        I: IntoIterator<Item = AvatarIDWithProvider<S>>,
    >(
        &self,
        insertables: I,
    ) -> Result<()> {
        let query = "
            INSERT INTO avatars (id, provider_bits, created_at, updated_at)
            VALUES (:id, :providers, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT (id) DO UPDATE
                SET updated_at = CURRENT_TIMESTAMP,
                    provider_bits = :providers
        ";

        let insertables: Vec<_> = insertables
            .into_iter()
            .map(|a| (a.0.to_string(), a.1))
            .collect();

        self.connection
            .call(|c| -> Result<(), rusqlite::Error> {
                let tx = c.transaction()?;
                for (id, providers) in insertables {
                    let _ = tx.execute(
                        query,
                        named_params! {
                            ":id": id,
                            ":providers": providers
                        },
                    );
                }
                tx.commit()
            })
            .await
            .map_err(anyhow::Error::from)
    }

    const CHUNK_SIZE: usize = 950; // sqlite parameter limit is 999

    /// # Errors
    /// Will return `Err` if `Connection::call(...)` errors
    pub async fn check_all_ids<I: IntoIterator<Item = String>>(
        &self,
        ids: I,
    ) -> Result<HashMap<String, u32>> {
        let ids: Vec<_> = ids.into_iter().collect();
        self.connection
            .call(|c| -> Result<_, rusqlite::Error> {
                let mut output = HashMap::new();

                for chunk in &ids.into_iter().chunks(Self::CHUNK_SIZE) {
                    let chunk: Vec<String> = chunk.collect();
                    for id in &chunk {
                        output.insert(id.clone(), 0);
                    }
                    let found_ids = Self::check_batch_ids(c, chunk.into_iter())?;
                    output.extend(found_ids);
                }

                Ok(output)
            })
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    fn check_batch_ids<I: Iterator<Item = String>>(
        conn: &RusqliteConnection,
        chunk: I,
    ) -> std::result::Result<HashMap<String, u32>, rusqlite::Error> {
        let chunk: Vec<_> = chunk.collect();
        assert!(chunk.len() <= Self::CHUNK_SIZE);

        let placeholders = std::iter::repeat_n("?", chunk.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT id, provider_bits FROM avatars WHERE id IN ({placeholders}) AND updated_at >= datetime('now', '-30 days')"
        );

        let mut stmt = conn.prepare(&sql)?;
        stmt.query_map(params_from_iter(chunk.iter()), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?
        .collect::<Result<HashMap<_, _>, _>>()
    }
}

// #[cfg(test)]
mod tests {
    use super::Cache;
    /// Helper to create a cache for tests
    #[allow(dead_code)]
    async fn cache() -> Cache {
        Cache::new_in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn creates_database_and_table() {
        let cache = cache().await;

        // Simple sanity check: storing should not fail
        cache
            .store_avatar_ids_with_providers(vec![("avatar_1", 1u32)].into_iter())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn inserts_and_reads_avatar_ids() {
        let cache = cache().await;

        cache
            .store_avatar_ids_with_providers(
                vec![("avatar_a", 1u32), ("avatar_b", 2u32)].into_iter(),
            )
            .await
            .unwrap();

        let result = cache
            .check_all_ids(vec!["avatar_a".into(), "avatar_b".into()])
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result["avatar_a"], 1);
        assert_eq!(result["avatar_b"], 2);
    }

    #[tokio::test]
    async fn returns_none_for_missing_ids() {
        let cache = cache().await;

        let result = cache
            .check_all_ids(vec!["missing_avatar".into()])
            .await
            .unwrap();

        assert_eq!(result["missing_avatar"], 0);
    }

    #[tokio::test]
    async fn updates_provider_bits_on_conflict() {
        let cache = cache().await;

        cache
            .store_avatar_ids_with_providers(vec![("avatar_x", 1u32)].into_iter())
            .await
            .unwrap();

        cache
            .store_avatar_ids_with_providers(vec![("avatar_x", 42u32)].into_iter())
            .await
            .unwrap();

        let result = cache.check_all_ids(vec!["avatar_x".into()]).await.unwrap();

        assert_eq!(result["avatar_x"], 42);
    }

    #[tokio::test]
    async fn respects_chunking_limits() {
        let cache = cache().await;

        #[allow(clippy::cast_possible_truncation)]
        let ids: Vec<(String, u32)> = (0..(Cache::CHUNK_SIZE + 10))
            .map(|i| (format!("avatar_{i}"), i as u32))
            .collect();

        cache
            .store_avatar_ids_with_providers(ids.iter().map(|(id, p)| (id.as_str(), *p)))
            .await
            .unwrap();

        let result = cache
            .check_all_ids(ids.iter().map(|(id, _)| id.clone()))
            .await
            .unwrap();

        assert_eq!(result.len(), ids.len());

        for (id, provider) in ids {
            assert_eq!(result[&id], provider);
        }
    }

    #[tokio::test]
    async fn ignores_entries_older_than_30_days() {
        let cache = cache().await;

        // Insert normally
        cache
            .store_avatar_ids_with_providers(vec![("old_avatar", 1u32)].into_iter())
            .await
            .unwrap();

        // Manually age the entry
        cache
            .connection
            .call(|c| {
                c.execute(
                    "UPDATE avatars
                     SET updated_at = datetime('now', '-31 days')
                     WHERE id = 'old_avatar'",
                    [],
                )
            })
            .await
            .unwrap();

        let result = cache
            .check_all_ids(vec!["old_avatar".into()])
            .await
            .unwrap();

        // Exists, but filtered out by age
        assert_eq!(result["old_avatar"], 0);
    }
}
