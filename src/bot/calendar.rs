use std::collections::BTreeMap;

use twilight_model::{
    guild::Permissions,
    id::{Id, marker::UserMarker},
};

use super::Bot;
use crate::{
    grants::random_token,
    records::{CalendarLink, Meet},
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

    pub async fn meets_joined_by(&self, user_id: Id<UserMarker>) -> anyhow::Result<Vec<Meet>> {
        let mut meets_by_guild: BTreeMap<_, Vec<Meet>> = BTreeMap::new();
        for meet in self.db.all_meets()? {
            meets_by_guild.entry(meet.guild_id).or_default().push(meet);
        }
        let mut joined = Vec::new();
        for (guild_id, meets) in meets_by_guild {
            if self.cache.guild(guild_id).is_none() {
                continue;
            }
            let Some(member) = self.member_roles(guild_id, user_id).await else {
                continue;
            };
            for meet in meets {
                if !member.role_ids.contains(&meet.role_id) {
                    continue;
                }
                let forum = self.forum_of_guild(guild_id, meet.forum_id).await;
                if forum.is_some_and(|forum| self.permissions_in(&forum, &member).contains(Permissions::VIEW_CHANNEL)) {
                    joined.push(meet);
                }
            }
        }
        Ok(joined)
    }
}
