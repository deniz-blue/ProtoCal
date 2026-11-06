use rusqlite::{OptionalExtension as _, Row, params};
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker},
};

use super::{Database, parsed, parsed_optional};
use crate::records::MeetForum;

const COLUMNS: &str = "forum_id, guild_id, announce_role_id, review_channel_id";

fn meet_forum_of_row(row: &Row) -> rusqlite::Result<MeetForum> {
    Ok(MeetForum {
        forum_id: parsed(row, 0)?,
        guild_id: parsed(row, 1)?,
        announce_role_id: parsed_optional(row, 2)?,
        review_channel_id: parsed_optional(row, 3)?,
    })
}

impl Database {
    pub fn meet_forum_of(&self, forum_id: Id<ChannelMarker>) -> anyhow::Result<Option<MeetForum>> {
        self.with(|db| {
            db.query_row(
                &format!("SELECT {COLUMNS} FROM meet_forums WHERE forum_id = ?1"),
                [forum_id.to_string()],
                meet_forum_of_row,
            )
            .optional()
        })
    }

    pub fn meet_forums_of_guild(&self, guild_id: Id<GuildMarker>) -> anyhow::Result<Vec<MeetForum>> {
        self.with(|db| {
            db.prepare(&format!("SELECT {COLUMNS} FROM meet_forums WHERE guild_id = ?1"))?
                .query_map([guild_id.to_string()], meet_forum_of_row)?
                .collect()
        })
    }

    pub fn save_meet_forum(&self, forum: &MeetForum) -> anyhow::Result<()> {
        self.with(|db| {
            db.execute(
                &format!(
                    "INSERT INTO meet_forums ({COLUMNS}) VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT (forum_id) DO UPDATE SET guild_id = excluded.guild_id,
                     announce_role_id = excluded.announce_role_id, review_channel_id = excluded.review_channel_id"
                ),
                params![
                    forum.forum_id.to_string(),
                    forum.guild_id.to_string(),
                    forum.announce_role_id.map(|id| id.to_string()),
                    forum.review_channel_id.map(|id| id.to_string()),
                ],
            )
            .map(drop)
        })
    }

    pub fn forget_meet_forum(&self, forum_id: Id<ChannelMarker>) -> anyhow::Result<bool> {
        self.with(|db| {
            db.execute("DELETE FROM meet_forums WHERE forum_id = ?1", [forum_id.to_string()])
                .map(|deleted| deleted > 0)
        })
    }
}
