use twilight_http::request::AuditLogReason as _;
use twilight_model::{
    channel::message::{AllowedMentions, EmojiReactionType},
    gateway::GatewayReaction,
    guild::Permissions,
    id::{
        Id,
        marker::{RoleMarker, UserMarker},
    },
};

use super::Bot;
use crate::{
    discord::{
        cache::CachedChannel,
        errors::{UNKNOWN_ROLE, is_discord_error},
    },
    meet::attend_reaction::{ReactionEmoji, role_name_of_meet},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoleChange {
    Add,
    Remove,
}

pub fn reaction_emoji_of(emoji: &EmojiReactionType) -> ReactionEmoji {
    match emoji {
        EmojiReactionType::Custom { id, name, .. } => ReactionEmoji {
            id: Some(id.to_string()),
            name: name.clone(),
        },
        EmojiReactionType::Unicode { name } => ReactionEmoji {
            id: None,
            name: Some(name.clone()),
        },
    }
}

pub async fn change_attendance(bot: &Bot, change: RoleChange, reaction: &GatewayReaction) -> anyhow::Result<()> {
    if reaction.message_id.get() != reaction.channel_id.get() {
        return Ok(());
    }
    let Some(meet) = bot.db.meet_of_thread(reaction.channel_id)? else {
        return Ok(());
    };
    if reaction_emoji_of(&reaction.emoji).key().as_deref() != Some(meet.attend_reaction.as_str()) {
        return Ok(());
    }
    let Some(thread) = bot.channel(reaction.channel_id).await else {
        return Ok(());
    };
    change_meet_role(bot, &thread, change, reaction.user_id, meet.role_id).await
}

pub async fn create_meet_role(bot: &Bot, thread: &CachedChannel) -> anyhow::Result<Id<RoleMarker>> {
    let name = role_name_of_meet(&thread.name);
    let reason = format!("Meet {}", thread.id);
    let role = bot
        .http
        .create_role(thread.guild_id)
        .name(&name)
        .mentionable(true)
        .permissions(Permissions::empty())
        .reason(&reason)
        .await?
        .model()
        .await?;
    Ok(role.id)
}

async fn add_meet_role(bot: &Bot, thread: &CachedChannel, user_id: Id<UserMarker>, role_id: Id<RoleMarker>) -> anyhow::Result<()> {
    let added = bot.http.add_guild_member_role(thread.guild_id, user_id, role_id).await;
    let Err(error) = added else {
        return Ok(());
    };
    let error = anyhow::Error::from(error);
    if !is_discord_error(&error, UNKNOWN_ROLE) {
        return Err(error);
    }
    let role = create_meet_role(bot, thread).await?;
    bot.db.replace_meet_role(thread.id, role)?;
    tracing::info!(thread = %thread.id, gone = %role_id, role = %role, "Meet role made again");
    bot.http.add_guild_member_role(thread.guild_id, user_id, role).await?;
    Ok(())
}

pub async fn change_meet_role(
    bot: &Bot,
    thread: &CachedChannel,
    change: RoleChange,
    user_id: Id<UserMarker>,
    role_id: Id<RoleMarker>,
) -> anyhow::Result<()> {
    let changed = match change {
        RoleChange::Add => add_meet_role(bot, thread, user_id, role_id).await,
        RoleChange::Remove => bot
            .http
            .remove_guild_member_role(thread.guild_id, user_id, role_id)
            .await
            .map(drop)
            .map_err(anyhow::Error::from),
    };
    let Err(error) = changed else {
        return Ok(());
    };
    if change == RoleChange::Remove && is_discord_error(&error, UNKNOWN_ROLE) {
        return Ok(());
    }
    tracing::error!(thread = %thread.id, user = %user_id, ?change, ?error, "Meet role was not changed");
    let verb = if change == RoleChange::Add { "give" } else { "take" };
    let content = format!("-# ❌ I could not {verb} the role for <@{user_id}>. Please tell staff.");
    bot.http
        .create_message(thread.id)
        .content(&content)
        .allowed_mentions(Some(&AllowedMentions::default()))
        .await?;
    Ok(())
}
