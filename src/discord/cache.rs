use std::{
    collections::HashMap,
    sync::{RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use twilight_gateway::Event;
use twilight_model::{
    channel::{Channel, ChannelType, permission_overwrite::PermissionOverwrite},
    gateway::payload::incoming::GuildCreate,
    guild::{Permissions, Role},
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker, RoleMarker, UserMarker},
    },
};

use crate::event::attend_reaction::ReactionEmoji;

#[derive(Clone, Debug, PartialEq)]
pub struct CachedRole {
    pub id: Id<RoleMarker>,
    pub name: String,
    pub permissions: Permissions,
    pub position: i64,
    pub managed: bool,
}

impl CachedRole {
    fn of(role: &Role) -> Self {
        Self {
            id: role.id,
            name: role.name.clone(),
            permissions: role.permissions,
            position: role.position,
            managed: role.managed,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CachedGuild {
    pub id: Id<GuildMarker>,
    pub name: String,
    pub owner_id: Id<UserMarker>,
    pub roles: HashMap<Id<RoleMarker>, CachedRole>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CachedChannel {
    pub id: Id<ChannelMarker>,
    pub guild_id: Id<GuildMarker>,
    pub kind: ChannelType,
    pub name: String,
    pub position: i32,
    pub parent_id: Option<Id<ChannelMarker>>,
    pub owner_id: Option<Id<UserMarker>>,
    pub locked: bool,
    pub overwrites: Vec<PermissionOverwrite>,
    pub default_reaction: Option<ReactionEmoji>,
}

impl CachedChannel {
    pub fn of(guild_id: Id<GuildMarker>, channel: &Channel) -> Self {
        Self {
            id: channel.id,
            guild_id,
            kind: channel.kind,
            name: channel.name.clone().unwrap_or_default(),
            position: channel.position.unwrap_or(0),
            parent_id: channel.parent_id,
            owner_id: channel.owner_id,
            locked: channel.thread_metadata.as_ref().is_some_and(|metadata| metadata.locked),
            overwrites: channel.permission_overwrites.clone().unwrap_or_default(),
            default_reaction: channel.default_reaction_emoji.as_ref().map(|emoji| ReactionEmoji {
                id: emoji.emoji_id.map(|id| id.to_string()),
                name: emoji.emoji_name.clone(),
            }),
        }
    }

    pub const fn is_thread(&self) -> bool {
        self.kind.is_thread()
    }
}

#[derive(Default)]
struct Known {
    guilds: HashMap<Id<GuildMarker>, CachedGuild>,
    channels: HashMap<Id<ChannelMarker>, CachedChannel>,
}

#[derive(Default)]
pub struct GuildCache {
    known: RwLock<Known>,
}

impl GuildCache {
    pub fn observe(&self, event: &Event) {
        match event {
            Event::GuildCreate(guild) => {
                if let GuildCreate::Available(guild) = guild.as_ref() {
                    self.insert_guild(CachedGuild {
                        id: guild.id,
                        name: guild.name.clone(),
                        owner_id: guild.owner_id,
                        roles: guild.roles.iter().map(|role| (role.id, CachedRole::of(role))).collect(),
                    });
                    for channel in guild.channels.iter().chain(&guild.threads) {
                        self.insert_channel(CachedChannel::of(guild.id, channel));
                    }
                }
            }
            Event::GuildUpdate(update) => {
                if let Some(guild) = self.get_known_mut().guilds.get_mut(&update.id) {
                    guild.name = update.name.clone();
                    guild.owner_id = update.owner_id;
                }
            }
            Event::GuildDelete(guild) => {
                let mut known = self.get_known_mut();
                known.guilds.remove(&guild.id);
                known.channels.retain(|_, channel| channel.guild_id != guild.id);
            }
            Event::RoleCreate(created) => self.insert_role(created.guild_id, &created.role),
            Event::RoleUpdate(updated) => self.insert_role(updated.guild_id, &updated.role),
            Event::RoleDelete(deleted) => {
                if let Some(guild) = self.get_known_mut().guilds.get_mut(&deleted.guild_id) {
                    guild.roles.remove(&deleted.role_id);
                }
            }
            Event::ChannelCreate(channel) => self.insert_event_channel(event, &channel.0),
            Event::ChannelUpdate(channel) => self.insert_event_channel(event, &channel.0),
            Event::ThreadCreate(thread) => self.insert_event_channel(event, &thread.0),
            Event::ThreadUpdate(thread) => self.insert_event_channel(event, &thread.0),
            Event::ThreadListSync(sync) => {
                for thread in &sync.threads {
                    self.insert_channel(CachedChannel::of(sync.guild_id, thread));
                }
            }
            Event::ChannelDelete(channel) => {
                self.get_known_mut().channels.remove(&channel.0.id);
            }
            Event::ThreadDelete(thread) => {
                self.get_known_mut().channels.remove(&thread.id);
            }
            _ => {}
        }
    }

    pub fn insert_guild(&self, guild: CachedGuild) {
        self.get_known_mut().guilds.insert(guild.id, guild);
    }

    pub fn insert_channel(&self, channel: CachedChannel) {
        self.get_known_mut().channels.insert(channel.id, channel);
    }

    pub fn guild(&self, guild_id: Id<GuildMarker>) -> Option<CachedGuild> {
        self.get_known().guilds.get(&guild_id).cloned()
    }

    pub fn channel(&self, channel_id: Id<ChannelMarker>) -> Option<CachedChannel> {
        self.get_known().channels.get(&channel_id).cloned()
    }

    pub fn role(&self, guild_id: Id<GuildMarker>, role_id: Id<RoleMarker>) -> Option<CachedRole> {
        self.get_known().guilds.get(&guild_id)?.roles.get(&role_id).cloned()
    }

    pub fn channels_of_guild(&self, guild_id: Id<GuildMarker>, kind: ChannelType) -> Vec<CachedChannel> {
        let mut channels: Vec<CachedChannel> = self
            .get_known()
            .channels
            .values()
            .filter(|channel| channel.guild_id == guild_id && channel.kind == kind)
            .cloned()
            .collect();
        channels.sort_by_key(|channel| (channel.position, channel.id));
        channels
    }

    fn insert_role(&self, guild_id: Id<GuildMarker>, role: &Role) {
        if let Some(guild) = self.get_known_mut().guilds.get_mut(&guild_id) {
            guild.roles.insert(role.id, CachedRole::of(role));
        }
    }

    fn insert_event_channel(&self, event: &Event, channel: &Channel) {
        if let Some(guild_id) = event.guild_id().or(channel.guild_id) {
            self.insert_channel(CachedChannel::of(guild_id, channel));
        }
    }

    fn get_known(&self) -> RwLockReadGuard<'_, Known> {
        self.known.read().expect("guild cache lock poisoned")
    }

    fn get_known_mut(&self) -> RwLockWriteGuard<'_, Known> {
        self.known.write().expect("guild cache lock poisoned")
    }
}
