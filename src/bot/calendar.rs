use std::collections::BTreeMap;

use twilight_model::{
    guild::Permissions,
    id::{Id, marker::UserMarker},
};

use super::Bot;
use crate::{
    grants::random_token,
    records::{CalendarLink, EventPost},
    time::now_unix_seconds,
};

impl Bot {
    pub fn calendar_link_of(&self, user_id: Id<UserMarker>, replace: bool) -> anyhow::Result<CalendarLink> {
        if let Some(existing) = self.db.calendar_link_of_user(user_id)?.filter(|_| !replace) {
            return Ok(existing);
        }
        let link = CalendarLink { user_id, token: random_token(), created_at: now_unix_seconds() };
        self.db.save_calendar_link(&link)?;
        Ok(link)
    }

    pub async fn events_joined_by(&self, user_id: Id<UserMarker>) -> anyhow::Result<Vec<EventPost>> {
        let mut events_by_guild: BTreeMap<_, Vec<EventPost>> = BTreeMap::new();
        for event in self.db.all_events()? {
            events_by_guild.entry(event.guild_id).or_default().push(event);
        }
        let mut joined = vec![];
        for (guild_id, events) in events_by_guild {
            if self.cache.guild(guild_id).is_none() {
                continue;
            }
            let Some(member) = self.member_roles(guild_id, user_id).await else {
                continue;
            };
            for event in events {
                if !member.role_ids.contains(&event.role_id) {
                    continue;
                }
                let forum = self.forum_of_guild(guild_id, event.forum_id).await;
                if forum.is_some_and(|forum| self.permissions_in(&forum, &member).contains(Permissions::VIEW_CHANNEL)) {
                    joined.push(event);
                }
            }
        }
        Ok(joined)
    }
}
