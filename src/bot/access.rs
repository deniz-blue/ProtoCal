use twilight_model::{
    application::interaction::Interaction,
    channel::ChannelType,
    guild::Permissions,
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker},
    },
};

use super::Bot;
use crate::{
    discord::{
        cache::CachedChannel,
        permissions::{MemberRoles, everyone_permissions_in, member_permissions_in},
    },
    records::MeetForum,
};

#[derive(Clone, Debug)]
pub struct OpenMeetForum {
    pub channel: CachedChannel,
    pub settings: MeetForum,
}

pub fn member_roles_of_interaction(interaction: &Interaction) -> Option<MemberRoles> {
    let member = interaction.member.as_ref()?;
    Some(MemberRoles {
        user_id: interaction.author_id()?,
        role_ids: member.roles.clone(),
    })
}

pub fn interaction_has_permission(interaction: &Interaction, permission: Permissions) -> bool {
    interaction
        .member
        .as_ref()
        .and_then(|member| member.permissions)
        .is_some_and(|permissions| permissions.contains(permission))
}

impl Bot {
    pub async fn forum_of_guild(&self, guild_id: Id<GuildMarker>, forum_id: Id<ChannelMarker>) -> Option<CachedChannel> {
        self.channel(forum_id)
            .await
            .filter(|forum| forum.guild_id == guild_id && forum.kind == ChannelType::GuildForum)
    }

    pub fn permissions_in(&self, channel: &CachedChannel, member: &MemberRoles) -> Permissions {
        self.cache
            .guild(channel.guild_id)
            .map_or(Permissions::empty(), |guild| member_permissions_in(&guild, channel, member))
    }

    pub fn everyone_permissions_in(&self, channel: &CachedChannel) -> Permissions {
        self.cache
            .guild(channel.guild_id)
            .map_or(Permissions::empty(), |guild| everyone_permissions_in(&guild, channel))
    }

    pub async fn everyone_can_view_forum(&self, guild_id: Id<GuildMarker>, forum_id: Id<ChannelMarker>) -> bool {
        if self.cache.guild(guild_id).is_none() {
            return false;
        }
        match self.forum_of_guild(guild_id, forum_id).await {
            Some(forum) => self.everyone_permissions_in(&forum).contains(Permissions::VIEW_CHANNEL),
            None => false,
        }
    }

    pub fn is_meet_moderator(&self, member: &MemberRoles, forum: &CachedChannel) -> bool {
        self.permissions_in(forum, member).contains(Permissions::MANAGE_THREADS)
    }

    pub fn meet_goes_to_review(&self, member: &MemberRoles, forum: &OpenMeetForum) -> bool {
        forum.settings.is_screened() && !self.is_meet_moderator(member, &forum.channel)
    }

    fn member_may_post_meet_in(&self, member: &MemberRoles, forum: &OpenMeetForum) -> bool {
        let needed = if forum.settings.is_screened() {
            Permissions::VIEW_CHANNEL
        } else {
            Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES
        };
        self.permissions_in(&forum.channel, member).contains(needed)
    }

    pub async fn meet_forums_open_to(&self, guild_id: Id<GuildMarker>, member: &MemberRoles) -> anyhow::Result<Vec<OpenMeetForum>> {
        let mut forums = Vec::new();
        for settings in self.db.meet_forums_of_guild(guild_id)? {
            let Some(channel) = self.forum_of_guild(guild_id, settings.forum_id).await else {
                continue;
            };
            let forum = OpenMeetForum { channel, settings };
            if self.member_may_post_meet_in(member, &forum) {
                forums.push(forum);
            }
        }
        Ok(forums)
    }

    pub async fn meet_forum_open_to(
        &self,
        guild_id: Id<GuildMarker>,
        member: &MemberRoles,
        forum_id: Id<ChannelMarker>,
    ) -> anyhow::Result<Option<OpenMeetForum>> {
        let forums = self.meet_forums_open_to(guild_id, member).await?;
        Ok(forums.into_iter().find(|forum| forum.channel.id == forum_id))
    }
}
