use rusqlite::{OptionalExtension as _, Row, params};
use twilight_model::id::{Id, marker::UserMarker};

use super::{Database, parsed};
use crate::records::CalendarLink;

fn calendar_link_of_row(row: &Row) -> rusqlite::Result<CalendarLink> {
    Ok(CalendarLink {
        user_id: parsed(row, 0)?,
        token: row.get(1)?,
        created_at: row.get(2)?,
    })
}

impl Database {
    pub fn calendar_link_of_user(&self, user_id: Id<UserMarker>) -> anyhow::Result<Option<CalendarLink>> {
        self.with(|db| {
            db.query_row(
                "SELECT user_id, token, created_at FROM calendar_links WHERE user_id = ?1",
                [user_id.to_string()],
                calendar_link_of_row,
            )
            .optional()
        })
    }

    pub fn calendar_link_of_token(&self, token: &str) -> anyhow::Result<Option<CalendarLink>> {
        self.with(|db| {
            db.query_row(
                "SELECT user_id, token, created_at FROM calendar_links WHERE token = ?1",
                [token],
                calendar_link_of_row,
            )
            .optional()
        })
    }

    pub fn save_calendar_link(&self, link: &CalendarLink) -> anyhow::Result<()> {
        self.with(|db| {
            db.execute(
                "INSERT INTO calendar_links (user_id, token, created_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT (user_id) DO UPDATE SET token = excluded.token, created_at = excluded.created_at",
                params![link.user_id.to_string(), link.token, link.created_at],
            )
            .map(drop)
        })
    }
}
