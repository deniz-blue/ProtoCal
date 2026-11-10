use rusqlite::{OptionalExtension as _, Row, params};
use serde_json::Value;
use twilight_model::id::{
    Id,
    marker::{MessageMarker, UserMarker},
};

use super::{Database, json_of, parsed, parsed_optional};
use crate::records::{EventRequest, MessageAt, RequestState};

const COLUMNS: &str = "id, guild_id, forum_id, host_id, document, state, handled_by, decline_reason, \
    host_edited, moderators_edited, card_channel_id, card_message_id, dm_channel_id, dm_message_id, updated_at";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestMatch {
    AnyState,
    OnlyIfWaiting,
}

impl RequestMatch {
    fn condition(self) -> &'static str {
        match self {
            Self::AnyState => "id = ?1",
            Self::OnlyIfWaiting => "id = ?1 AND state = 'waiting'",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestEditor {
    Host,
    Moderator,
}

fn event_request_of_row(row: &Row) -> rusqlite::Result<EventRequest> {
    let state: String = row.get(5)?;
    let dm_channel_id = parsed_optional(row, 12)?;
    let dm_message_id = parsed_optional(row, 13)?;
    Ok(EventRequest {
        id: row.get(0)?,
        guild_id: parsed(row, 1)?,
        forum_id: parsed(row, 2)?,
        host_id: parsed(row, 3)?,
        document: json_of(row, 4)?,
        state: RequestState::of_str(&state).unwrap_or(RequestState::Waiting),
        handled_by: parsed_optional(row, 6)?,
        decline_reason: row.get(7)?,
        host_edited: row.get(8)?,
        moderators_edited: row.get(9)?,
        card_channel_id: parsed(row, 10)?,
        card_message_id: parsed_optional(row, 11)?,
        host_dm: dm_channel_id
            .zip(dm_message_id)
            .map(|(channel_id, message_id)| MessageAt { channel_id, message_id }),
        updated_at: row.get(14)?,
    })
}

impl Database {
    pub fn event_request_of(&self, id: &str) -> anyhow::Result<Option<EventRequest>> {
        self.with(|db| {
            db.query_row(&format!("SELECT {COLUMNS} FROM event_requests WHERE id = ?1"), [id], event_request_of_row)
                .optional()
        })
    }

    pub fn all_event_requests(&self) -> anyhow::Result<Vec<EventRequest>> {
        self.with(|db| {
            db.prepare(&format!("SELECT {COLUMNS} FROM event_requests"))?
                .query_map([], event_request_of_row)?
                .collect()
        })
    }

    pub fn save_event_request(&self, request: &EventRequest) -> anyhow::Result<()> {
        self.with(|db| {
            db.execute(
                &format!("INSERT INTO event_requests ({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"),
                params![
                    request.id,
                    request.guild_id.to_string(),
                    request.forum_id.to_string(),
                    request.host_id.to_string(),
                    request.document.to_string(),
                    request.state.as_str(),
                    request.handled_by.map(|id| id.to_string()),
                    request.decline_reason,
                    request.host_edited,
                    request.moderators_edited,
                    request.card_channel_id.to_string(),
                    request.card_message_id.map_or_else(String::new, |id| id.to_string()),
                    request.host_dm.map(|dm| dm.channel_id.to_string()),
                    request.host_dm.map(|dm| dm.message_id.to_string()),
                    request.updated_at,
                ],
            )
            .map(drop)
        })
    }

    pub fn decline_event_request(
        &self,
        id: &str,
        by: Id<UserMarker>,
        reason: Option<&str>,
    ) -> anyhow::Result<Option<EventRequest>> {
        self.with(|db| {
            db.query_row(
                &format!(
                    "UPDATE event_requests SET state = 'declined', handled_by = ?2, decline_reason = ?3
                     WHERE {} RETURNING {COLUMNS}",
                    RequestMatch::OnlyIfWaiting.condition()
                ),
                params![id, by.to_string(), reason],
                event_request_of_row,
            )
            .optional()
        })
    }

    pub fn edit_event_request(
        &self,
        id: &str,
        document: &Value,
        editor: RequestEditor,
        updated_at: i64,
    ) -> anyhow::Result<Option<EventRequest>> {
        let edited = match editor {
            RequestEditor::Host => "host_edited = 1, state = 'waiting', handled_by = NULL, decline_reason = NULL",
            RequestEditor::Moderator => "moderators_edited = 1",
        };
        self.with(|db| {
            db.query_row(
                &format!("UPDATE event_requests SET document = ?2, updated_at = ?3, {edited} WHERE id = ?1 RETURNING {COLUMNS}"),
                params![id, document.to_string(), updated_at],
                event_request_of_row,
            )
            .optional()
        })
    }

    pub fn move_event_request_card(&self, id: &str, card_message_id: Id<MessageMarker>) -> anyhow::Result<Option<EventRequest>> {
        self.with(|db| {
            db.query_row(
                &format!("UPDATE event_requests SET card_message_id = ?2 WHERE id = ?1 RETURNING {COLUMNS}"),
                params![id, card_message_id.to_string()],
                event_request_of_row,
            )
            .optional()
        })
    }

    pub fn forget_event_request(&self, id: &str, matching: RequestMatch) -> anyhow::Result<Option<EventRequest>> {
        self.with(|db| {
            db.query_row(
                &format!("DELETE FROM event_requests WHERE {} RETURNING {COLUMNS}", matching.condition()),
                [id],
                event_request_of_row,
            )
            .optional()
        })
    }
}
