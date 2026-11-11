use std::collections::{HashMap, HashSet};

use twilight_model::{
    channel::ChannelType,
    guild::Permissions,
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker},
    },
};

use super::{Bot, ConfigGrant};
use crate::{
    discord::{
        cache::{CachedChannel, CachedGuild, CachedRole},
        permissions::guild_permissions,
    },
    records::EventForum,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ForumConfig {
    pub forum_id: Id<ChannelMarker>,
    pub name: Option<String>,
    pub is_event_forum: bool,
    pub announce_role_id: Option<String>,
    pub review_channel_id: Option<String>,
    pub has_default_reaction: bool,
    pub everyone_can_view: bool,
    pub everyone_can_post: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GuildConfigChange {
    pub event_forum_ids: HashSet<String>,
    pub announce_role_id_of: HashMap<String, String>,
    pub review_channel_id_of: HashMap<String, String>,
}

pub fn may_configure_guild(permissions: Option<Permissions>) -> bool {
    permissions.is_some_and(|permissions| permissions.contains(Permissions::MANAGE_GUILD))
}

impl Bot {
    pub async fn guild_of_config_grant(&self, token: &str) -> Option<CachedGuild> {
        let ConfigGrant { guild_id, user_id } = self.config_grants.find(token)?;
        let guild = self.cache.guild(guild_id)?;
        let member = self.member_roles(guild_id, user_id).await?;
        may_configure_guild(Some(guild_permissions(&guild, &member))).then_some(guild)
    }

    pub fn review_channels_of(&self, guild_id: Id<GuildMarker>) -> Vec<CachedChannel> {
        self.cache.channels_of_guild(guild_id, ChannelType::GuildText)
    }

    pub fn forum_configs_of(&self, guild_id: Id<GuildMarker>) -> anyhow::Result<Vec<ForumConfig>> {
        let forums = self.cache.channels_of_guild(guild_id, ChannelType::GuildForum);
        let saved: HashMap<_, EventForum> = self
            .db
            .event_forums_of_guild(guild_id)?
            .into_iter()
            .map(|forum| (forum.forum_id, forum))
            .collect();
        let configs = forums.iter().map(|forum| {
            let everyone = self.everyone_permissions_in(forum);
            let settings = saved.get(&forum.id);
            ForumConfig {
                forum_id: forum.id,
                name: Some(forum.name.clone()),
                is_event_forum: settings.is_some(),
                announce_role_id: settings.and_then(|settings| settings.announce_role_id).map(|id| id.to_string()),
                review_channel_id: settings.and_then(|settings| settings.review_channel_id).map(|id| id.to_string()),
                has_default_reaction: forum.default_reaction.is_some(),
                everyone_can_view: everyone.contains(Permissions::VIEW_CHANNEL),
                everyone_can_post: everyone.contains(Permissions::SEND_MESSAGES),
            }
        });
        let mut deleted: Vec<&EventForum> = saved
            .values()
            .filter(|settings| !forums.iter().any(|forum| forum.id == settings.forum_id))
            .collect();
        deleted.sort_by_key(|settings| settings.forum_id);
        let deleted_configs = deleted.into_iter().map(|settings| ForumConfig {
            forum_id: settings.forum_id,
            name: None,
            is_event_forum: true,
            announce_role_id: settings.announce_role_id.map(|id| id.to_string()),
            review_channel_id: settings.review_channel_id.map(|id| id.to_string()),
            has_default_reaction: false,
            everyone_can_view: false,
            everyone_can_post: false,
        });
        Ok(configs.chain(deleted_configs).collect())
    }

    pub fn announceable_roles_of(&self, guild: &CachedGuild) -> anyhow::Result<Vec<CachedRole>> {
        let event_role_ids: HashSet<_> = self
            .db
            .all_events()?
            .into_iter()
            .filter(|event| event.guild_id == guild.id)
            .map(|event| event.role_id)
            .collect();
        let mut roles: Vec<CachedRole> = guild
            .roles
            .values()
            .filter(|role| role.id.cast() != guild.id && !role.managed && !event_role_ids.contains(&role.id))
            .cloned()
            .collect();
        roles.sort_by_key(|role| (std::cmp::Reverse(role.position), role.id));
        Ok(roles)
    }

    pub fn save_guild_config(&self, guild: &CachedGuild, change: &GuildConfigChange) -> anyhow::Result<()> {
        let role_ids: HashSet<String> = self.announceable_roles_of(guild)?.iter().map(|role| role.id.to_string()).collect();
        let review_channel_ids: HashSet<String> =
            self.review_channels_of(guild.id).iter().map(|channel| channel.id.to_string()).collect();
        for forum in self.forum_configs_of(guild.id)? {
            let key = forum.forum_id.to_string();
            if !change.event_forum_ids.contains(&key) {
                if forum.is_event_forum {
                    self.db.forget_event_forum(forum.forum_id)?;
                }
                continue;
            }

            fn chosen<'a>(of: &'a HashMap<String, String>, key: &String, allowed: &'a HashSet<String>) -> Option<&'a String> {
                of.get(key).filter(|&id| allowed.contains(id))
            }

            self.db.save_event_forum(&EventForum {
                forum_id: forum.forum_id,
                guild_id: guild.id,
                announce_role_id: chosen(&change.announce_role_id_of, &key, &role_ids).and_then(|id| id.parse().ok()),
                review_channel_id: chosen(&change.review_channel_id_of, &key, &review_channel_ids).and_then(|id| id.parse().ok()),
            })?;
        }
        Ok(())
    }
}
