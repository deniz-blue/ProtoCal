mod calendar_links;
mod meet_forums;
mod meet_requests;
mod meets;

use std::{fs, path::Path, str::FromStr, sync::Mutex};

use anyhow::Context as _;
use rusqlite::{Connection, Row, types::Type};

pub use meet_requests::{RequestEditor, RequestMatch};

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS `meet_forums` (
        `forum_id` text PRIMARY KEY NOT NULL,
        `guild_id` text NOT NULL,
        `announce_role_id` text,
        `review_channel_id` text
    );
    CREATE TABLE IF NOT EXISTS `meets` (
        `thread_id` text PRIMARY KEY NOT NULL,
        `guild_id` text NOT NULL,
        `forum_id` text NOT NULL,
        `role_id` text NOT NULL,
        `attend_reaction` text NOT NULL,
        `organizers` text NOT NULL,
        `document` text NOT NULL,
        `updated_at` integer NOT NULL
    );
    CREATE TABLE IF NOT EXISTS `calendar_links` (
        `user_id` text PRIMARY KEY NOT NULL,
        `token` text NOT NULL,
        `created_at` integer NOT NULL
    );
    CREATE UNIQUE INDEX IF NOT EXISTS `calendar_links_token_unique` ON `calendar_links` (`token`);
    CREATE TABLE IF NOT EXISTS `meet_requests` (
        `id` text PRIMARY KEY NOT NULL,
        `guild_id` text NOT NULL,
        `forum_id` text NOT NULL,
        `host_id` text NOT NULL,
        `document` text NOT NULL,
        `state` text NOT NULL,
        `handled_by` text,
        `decline_reason` text,
        `host_edited` integer NOT NULL,
        `moderators_edited` integer NOT NULL,
        `card_channel_id` text NOT NULL,
        `card_message_id` text NOT NULL,
        `dm_channel_id` text,
        `dm_message_id` text,
        `updated_at` integer NOT NULL
    );
";

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(directory) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            fs::create_dir_all(directory)
                .with_context(|| format!("creating {}", directory.display()))?;
        }
        let connection =
            Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        Self::of_connection(connection)
    }

    pub fn of_connection(connection: Connection) -> anyhow::Result<Self> {
        connection.execute_batch(SCHEMA)?;
        let has_review_channel: bool = connection.query_row(
            "SELECT count(*) > 0 FROM pragma_table_info('meet_forums') WHERE name = 'review_channel_id'",
            [],
            |row| row.get(0),
        )?;
        if !has_review_channel {
            connection.execute_batch("ALTER TABLE `meet_forums` ADD `review_channel_id` text")?;
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self::of_connection(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn with<T>(&self, query: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> anyhow::Result<T> {
        let connection = self.connection.lock().expect("database lock poisoned");
        Ok(query(&connection)?)
    }
}

fn conversion_failure(index: usize, error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
}

fn parsed<T: FromStr>(row: &Row, index: usize) -> rusqlite::Result<T>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    row.get::<_, String>(index)?
        .parse()
        .map_err(|error| conversion_failure(index, error))
}

fn parsed_optional<T: FromStr>(row: &Row, index: usize) -> rusqlite::Result<Option<T>>
where
    T::Err: std::error::Error + Send + Sync + 'static,
{
    row.get::<_, Option<String>>(index)?
        .filter(|text| !text.is_empty())
        .map(|text| text.parse().map_err(|error| conversion_failure(index, error)))
        .transpose()
}

fn json_of<T: serde::de::DeserializeOwned>(row: &Row, index: usize) -> rusqlite::Result<T> {
    serde_json::from_str(&row.get::<_, String>(index)?).map_err(|error| conversion_failure(index, error))
}

#[cfg(test)]
mod tests;
