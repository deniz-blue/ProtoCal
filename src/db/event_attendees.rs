use twilight_model::id::{
    Id,
    marker::{ChannelMarker, UserMarker},
};

use super::{Database, parsed};

impl Database {
    pub fn add_event_attendee(&self, thread_id: Id<ChannelMarker>, user_id: Id<UserMarker>) -> anyhow::Result<()> {
        self.with(|db| {
            db.execute(
                "INSERT OR IGNORE INTO event_attendees (thread_id, user_id) VALUES (?1, ?2)",
                [thread_id.to_string(), user_id.to_string()],
            )
            .map(drop)
        })
    }

    pub fn remove_event_attendee(&self, thread_id: Id<ChannelMarker>, user_id: Id<UserMarker>) -> anyhow::Result<()> {
        self.with(|db| {
            db.execute(
                "DELETE FROM event_attendees WHERE thread_id = ?1 AND user_id = ?2",
                [thread_id.to_string(), user_id.to_string()],
            )
            .map(drop)
        })
    }

    pub fn events_attended_by(&self, user_id: Id<UserMarker>) -> anyhow::Result<Vec<Id<ChannelMarker>>> {
        self.with(|db| {
            let mut statement = db.prepare("SELECT thread_id FROM event_attendees WHERE user_id = ?1")?;
            let rows = statement.query_map([user_id.to_string()], |row| parsed(row, 0))?;
            rows.collect()
        })
    }
}
