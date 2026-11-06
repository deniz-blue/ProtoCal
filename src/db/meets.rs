use rusqlite::{OptionalExtension as _, Row, params};
use serde_json::Value;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, RoleMarker},
};

use super::{Database, json_of, parsed};
use crate::records::Meet;

const COLUMNS: &str =
    "thread_id, guild_id, forum_id, role_id, attend_reaction, organizers, document, updated_at";

fn meet_of_row(row: &Row) -> rusqlite::Result<Meet> {
    Ok(Meet {
        thread_id: parsed(row, 0)?,
        guild_id: parsed(row, 1)?,
        forum_id: parsed(row, 2)?,
        role_id: parsed(row, 3)?,
        attend_reaction: row.get(4)?,
        organizers: json_of(row, 5)?,
        document: json_of(row, 6)?,
        updated_at: row.get(7)?,
    })
}

impl Database {
    pub fn meet_of_thread(&self, thread_id: Id<ChannelMarker>) -> anyhow::Result<Option<Meet>> {
        self.with(|db| {
            db.query_row(
                &format!("SELECT {COLUMNS} FROM meets WHERE thread_id = ?1"),
                [thread_id.to_string()],
                meet_of_row,
            )
            .optional()
        })
    }

    pub fn all_meets(&self) -> anyhow::Result<Vec<Meet>> {
        self.with(|db| {
            db.prepare(&format!("SELECT {COLUMNS} FROM meets"))?
                .query_map([], meet_of_row)?
                .collect()
        })
    }

    pub fn meets_of_forum(&self, forum_id: Id<ChannelMarker>) -> anyhow::Result<Vec<Meet>> {
        self.with(|db| {
            db.prepare(&format!("SELECT {COLUMNS} FROM meets WHERE forum_id = ?1"))?
                .query_map([forum_id.to_string()], meet_of_row)?
                .collect()
        })
    }

    pub fn save_meet(&self, meet: &Meet) -> anyhow::Result<()> {
        let organizers = serde_json::to_string(&meet.organizers)?;
        self.with(|db| {
            db.execute(
                &format!("INSERT INTO meets ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"),
                params![
                    meet.thread_id.to_string(),
                    meet.guild_id.to_string(),
                    meet.forum_id.to_string(),
                    meet.role_id.to_string(),
                    meet.attend_reaction,
                    organizers,
                    meet.document.to_string(),
                    meet.updated_at,
                ],
            )
            .map(drop)
        })
    }

    pub fn replace_meet_role(&self, thread_id: Id<ChannelMarker>, role_id: Id<RoleMarker>) -> anyhow::Result<()> {
        self.with(|db| {
            db.execute(
                "UPDATE meets SET role_id = ?1 WHERE thread_id = ?2",
                [role_id.to_string(), thread_id.to_string()],
            )
            .map(drop)
        })
    }

    pub fn describe_meet(&self, thread_id: Id<ChannelMarker>, document: &Value, updated_at: i64) -> anyhow::Result<()> {
        self.with(|db| {
            db.execute(
                "UPDATE meets SET document = ?1, updated_at = ?2 WHERE thread_id = ?3",
                params![document.to_string(), updated_at, thread_id.to_string()],
            )
            .map(drop)
        })
    }

    pub fn forget_meet(&self, thread_id: Id<ChannelMarker>) -> anyhow::Result<()> {
        self.with(|db| {
            db.execute("DELETE FROM meets WHERE thread_id = ?1", [thread_id.to_string()])
                .map(drop)
        })
    }
}
