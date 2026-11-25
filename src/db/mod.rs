mod calendar_links;
mod event_attendees;
mod event_forums;
mod event_requests;
mod events;

use std::{fs, path::Path, str::FromStr, sync::Mutex};

use anyhow::Context as _;
use rusqlite::{Connection, Row, types::Type};

pub use event_requests::{RequestEditor, RequestMatch};
use crate::grants::random_link_id;

const RENAMED_TABLES: [(&str, &str); 3] = [
    ("meets", "events"),
    ("meet_forums", "event_forums"),
    ("meet_requests", "event_requests"),
];

const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS `event_forums` (
        `forum_id` text PRIMARY KEY NOT NULL,
        `guild_id` text NOT NULL,
        `announce_role_id` text,
        `review_channel_id` text,
        `link_id` text NOT NULL
    );
    CREATE TABLE IF NOT EXISTS `events` (
        `thread_id` text PRIMARY KEY NOT NULL,
        `guild_id` text NOT NULL,
        `forum_id` text NOT NULL,
        `role_id` text NOT NULL,
        `attend_reaction` text NOT NULL,
        `organizers` text NOT NULL,
        `document` text NOT NULL,
        `updated_at` integer NOT NULL,
        `link_id` text NOT NULL
    );
    CREATE TABLE IF NOT EXISTS `event_attendees` (
        `thread_id` text NOT NULL,
        `user_id` text NOT NULL,
        PRIMARY KEY (`thread_id`, `user_id`)
    );
    CREATE TABLE IF NOT EXISTS `calendar_links` (
        `user_id` text PRIMARY KEY NOT NULL,
        `token` text NOT NULL,
        `created_at` integer NOT NULL
    );
    CREATE UNIQUE INDEX IF NOT EXISTS `calendar_links_token_unique` ON `calendar_links` (`token`);
    CREATE TABLE IF NOT EXISTS `event_requests` (
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

const LINK_ID_INDEXES: &str = "
    CREATE UNIQUE INDEX IF NOT EXISTS `event_forums_link_id_unique` ON `event_forums` (`link_id`);
    CREATE UNIQUE INDEX IF NOT EXISTS `events_link_id_unique` ON `events` (`link_id`);
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
        rename_tables_to_event_names(&connection)?;
        connection.execute_batch(SCHEMA)?;
        if !has_column(&connection, "event_forums", "review_channel_id")? {
            connection.execute_batch("ALTER TABLE `event_forums` ADD `review_channel_id` text")?;
        }
        add_link_ids_to_forums_and_events(&connection)?;
        connection.execute_batch(LINK_ID_INDEXES)?;
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

fn rename_tables_to_event_names(connection: &Connection) -> rusqlite::Result<()> {
    for (old, new) in RENAMED_TABLES {
        if table_exists(connection, old)? && !table_exists(connection, new)? {
            connection.execute_batch(&format!("ALTER TABLE `{old}` RENAME TO `{new}`"))?;
        }
    }
    Ok(())
}

fn table_exists(connection: &Connection, name: &str) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT count(*) > 0 FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [name],
        |row| row.get(0),
    )
}

fn has_column(connection: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT count(*) > 0 FROM pragma_table_info(?1) WHERE name = ?2",
        [table, column],
        |row| row.get(0),
    )
}

fn add_link_ids(connection: &Connection, table: &str, key: &str) -> rusqlite::Result<Vec<(String, String)>> {
    if has_column(connection, table, "link_id")? {
        return Ok(vec![]);
    }
    connection.execute_batch(&format!("ALTER TABLE `{table}` ADD `link_id` text"))?;
    let keys: Vec<String> = connection
        .prepare(&format!("SELECT `{key}` FROM `{table}`"))?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    keys.into_iter()
        .map(|key_value| {
            let link_id = random_link_id();
            connection.execute(&format!("UPDATE `{table}` SET `link_id` = ?1 WHERE `{key}` = ?2"), [&link_id, &key_value])?;
            Ok((key_value, link_id))
        })
        .collect()
}

fn add_link_ids_to_forums_and_events(connection: &Connection) -> rusqlite::Result<()> {
    let transaction = connection.unchecked_transaction()?;
    add_link_ids(&transaction, "event_forums", "forum_id")?;
    for (thread_id, link_id) in add_link_ids(&transaction, "events", "thread_id")? {
        let (old, new) = (format!("/events/{thread_id}.evnt.json"), format!("/events/{link_id}.evnt.json"));
        for table in ["events", "event_requests"] {
            transaction.execute(&format!("UPDATE `{table}` SET `document` = replace(`document`, ?1, ?2)"), [&old, &new])?;
        }
    }
    transaction.commit()
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
