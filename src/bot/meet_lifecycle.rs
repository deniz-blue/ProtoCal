use twilight_http::request::AuditLogReason as _;
use std::collections::BTreeSet;

use serde_json::Value;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::{
    channel::{ChannelType, message::AllowedMentions},
    id::{
        Id,
        marker::{ChannelMarker, EmojiMarker, GuildMarker, UserMarker},
    },
};

use super::{
    Bot,
    meet_roles::{RoleChange, change_meet_role, create_meet_role},
};
use crate::{
    discord::cache::CachedChannel,
    meet::{
        attend_reaction::ReactionEmoji,
        dates::meet_is_over_at,
        document::new_meet_document,
        post::{MESSAGE_MAX, meet_post_buttons, meet_post_content, thread_name_of},
    },
    records::Meet,
    time::now_unix_seconds,
};

pub use super::meet_lifecycle_edit::{rename_meet_of_thread, save_meet_document};

pub struct MeetAsPosted {
    pub organizers: Vec<Id<UserMarker>>,
    pub document: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeetStart {
    Started,
    NotAMeetForum,
    NoAttendReaction,
}

fn request_reaction_of(emoji: &ReactionEmoji) -> Option<RequestReactionType<'_>> {
    match (&emoji.id, &emoji.name) {
        (Some(id), _) => Some(RequestReactionType::Custom { id: id.parse::<Id<EmojiMarker>>().ok()?, name: None }),
        (None, Some(name)) => Some(RequestReactionType::Unicode { name }),
        (None, None) => None,
    }
}

async fn earlier_attendees(bot: &Bot, thread: &CachedChannel, emoji: &RequestReactionType<'_>) -> Vec<Id<UserMarker>> {
    let Ok(response) = bot.http.reactions(thread.id, thread.id.cast(), emoji).limit(100).await else {
        return Vec::new();
    };
    let users = response.models().await.unwrap_or_default();
    users.into_iter().filter(|user| !user.bot).map(|user| user.id).collect()
}

pub async fn start_meet(bot: &Bot, thread: &CachedChannel, posted: Option<MeetAsPosted>) -> anyhow::Result<MeetStart> {
    let Some(forum_id) = thread.parent_id else {
        return Ok(MeetStart::NotAMeetForum);
    };
    let Some(parent) = bot.channel(forum_id).await.filter(|parent| parent.kind == ChannelType::GuildForum) else {
        return Ok(MeetStart::NotAMeetForum);
    };
    let Some(forum) = bot.db.meet_forum_of(forum_id)? else {
        return Ok(MeetStart::NotAMeetForum);
    };
    let attend = parent.default_reaction.as_ref();
    let (Some(attend_emoji), Some(attend_reaction)) = (attend, attend.and_then(ReactionEmoji::key)) else {
        bot.http
            .create_message(thread.id)
            .content("-# ❌ This forum has no default reaction, so there is nothing to react with to attend. Staff can set one in the forum's settings.")
            .await?;
        return Ok(MeetStart::NoAttendReaction);
    };
    let posted = posted.unwrap_or_else(|| MeetAsPosted {
        organizers: thread.owner_id.into_iter().collect(),
        document: new_meet_document(&bot.links.meet_document_url(thread.id), &thread.name),
    });
    let role_id = create_meet_role(bot, thread).await?;
    bot.db.save_meet(&Meet {
        thread_id: thread.id,
        guild_id: thread.guild_id,
        forum_id: forum.forum_id,
        role_id,
        attend_reaction,
        organizers: posted.organizers.clone(),
        document: posted.document,
        updated_at: now_unix_seconds(),
    })?;
    tracing::info!(thread = %thread.id, role = %role_id, "Meet started");

    let mut attendees: BTreeSet<Id<UserMarker>> = posted.organizers.into_iter().collect();
    if bot.message(thread.id, thread.id.cast()).await.is_some()
        && let Some(emoji) = request_reaction_of(attend_emoji)
    {
        attendees.extend(earlier_attendees(bot, thread, &emoji).await);
        bot.http.create_reaction(thread.id, thread.id.cast(), &emoji).await?;
        bot.http.create_pin(thread.id, thread.id.cast()).await?;
    }
    for attendee in attendees {
        change_meet_role(bot, thread, RoleChange::Add, attendee, role_id).await?;
    }

    let announced: Vec<_> = forum.announce_role_id.into_iter().collect();
    let mut lines: Vec<String> = announced.iter().map(|role| format!("<@&{role}>")).collect();
    lines.push(format!("-# Role: <@&{role_id}>"));
    lines.push(format!(
        "-# React with {} on the first message if you are looking to attend, to get the role",
        attend_emoji.as_text()
    ));
    let mentions = AllowedMentions { roles: announced, ..Default::default() };
    bot.http
        .create_message(thread.id)
        .content(&lines.join("\n"))
        .allowed_mentions(Some(&mentions))
        .await?;
    Ok(MeetStart::Started)
}

pub async fn post_new_meet(
    bot: &Bot,
    forum: &CachedChannel,
    organizers: Vec<Id<UserMarker>>,
    document: &Value,
) -> anyhow::Result<Id<ChannelMarker>> {
    let content = meet_post_content(document, &organizers, now_unix_seconds(), MESSAGE_MAX);
    let mentions = AllowedMentions::default();
    let posted = bot
        .http
        .create_forum_thread(forum.id, &thread_name_of(document))
        .message()
        .content(&content)
        .allowed_mentions(Some(&mentions))
        .await?
        .model()
        .await?;
    let thread = CachedChannel::of(forum.guild_id, &posted.channel);
    bot.cache.insert_channel(thread.clone());
    let viewable = bot.everyone_can_view_forum(forum.guild_id, forum.id).await;
    let buttons = [meet_post_buttons(&bot.links, thread.id, viewable)];
    bot.http
        .update_message(thread.id, posted.message.id)
        .components(Some(&buttons))
        .await?;
    let mut document = document.clone();
    document["id"] = Value::String(bot.links.meet_document_url(thread.id));
    start_meet(bot, &thread, Some(MeetAsPosted { organizers, document })).await?;
    Ok(thread.id)
}

pub async fn end_meet(bot: &Bot, guild_id: Id<GuildMarker>, thread_id: Id<ChannelMarker>) -> anyhow::Result<()> {
    let Some(meet) = bot.db.meet_of_thread(thread_id)? else {
        return Ok(());
    };
    let reason = format!("Meet {thread_id} ended");
    if let Err(error) = bot.http.delete_role(guild_id, meet.role_id).reason(&reason).await {
        tracing::warn!(thread = %thread_id, ?error, "Meet role was not deleted");
    }
    bot.db.forget_meet(thread_id)?;
    tracing::info!(thread = %thread_id, "Meet ended");
    Ok(())
}

pub async fn end_meets_that_are_over(bot: &Bot, now_unix_seconds: i64) -> anyhow::Result<()> {
    for meet in bot.db.all_meets()? {
        let over = meet_is_over_at(&meet.document).is_some_and(|over_at| over_at <= now_unix_seconds);
        if over && bot.cache.guild(meet.guild_id).is_some() {
            end_meet(bot, meet.guild_id, meet.thread_id).await?;
        }
    }
    Ok(())
}
