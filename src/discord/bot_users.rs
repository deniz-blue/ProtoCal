use std::{
    collections::HashSet,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use twilight_gateway::Event;
use twilight_model::{
    gateway::payload::incoming::GuildCreate,
    id::{Id, marker::UserMarker},
};

#[derive(Default)]
pub struct BotUsers {
    seen: RwLock<HashSet<Id<UserMarker>>>,
}

impl BotUsers {
    pub fn observe(&self, event: &Event) {
        match event {
            Event::GuildCreate(guild) => {
                if let GuildCreate::Available(guild) = guild.as_ref() {
                    let bots = guild.members.iter().map(|member| &member.user).filter(|user| user.bot);
                    self.get_seen_mut().extend(bots.map(|user| user.id));
                }
            }
            Event::ReactionAdd(reaction) => {
                if let Some(member) = reaction.member.as_ref().filter(|member| member.user.bot) {
                    self.get_seen_mut().insert(member.user.id);
                }
            }
            Event::MessageCreate(message) if message.author.bot => {
                self.get_seen_mut().insert(message.author.id);
            }
            _ => {}
        }
    }

    pub fn is_bot(&self, user_id: Id<UserMarker>) -> bool {
        self.get_seen().contains(&user_id)
    }

    fn get_seen(&self) -> RwLockReadGuard<'_, HashSet<Id<UserMarker>>> {
        self.seen.read().expect("bot user lock poisoned")
    }

    fn get_seen_mut(&self) -> RwLockWriteGuard<'_, HashSet<Id<UserMarker>>> {
        self.seen.write().expect("bot user lock poisoned")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use twilight_gateway::Event;
    use twilight_model::{
        gateway::{GatewayReaction, payload::incoming::ReactionAdd},
        id::Id,
    };

    use super::BotUsers;

    fn reaction_add(user_id: u64, bot: bool) -> Event {
        let reaction: GatewayReaction = serde_json::from_value(json!({
            "burst": false,
            "burst_colors": [],
            "channel_id": "10",
            "emoji": { "name": "👋" },
            "guild_id": "1",
            "message_id": "10",
            "user_id": user_id.to_string(),
            "type": 0,
            "member": {
                "deaf": false,
                "flags": 0,
                "joined_at": "2026-01-01T00:00:00.000000+00:00",
                "mute": false,
                "roles": [],
                "user": { "id": user_id.to_string(), "username": "someone", "discriminator": "0", "avatar": null, "bot": bot }
            }
        }))
        .unwrap();
        Event::ReactionAdd(Box::new(ReactionAdd(reaction)))
    }

    #[test]
    fn bots_seen_reacting_are_remembered() {
        let users = BotUsers::default();
        users.observe(&reaction_add(7, true));
        users.observe(&reaction_add(8, false));
        assert!(users.is_bot(Id::new(7)));
        assert!(!users.is_bot(Id::new(8)));
        assert!(!users.is_bot(Id::new(9)));
    }
}
