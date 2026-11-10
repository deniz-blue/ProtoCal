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
    access::OpenEventForum,
    event_roles::{RoleChange, change_event_role, create_event_role},
};
use crate::{
    discord::cache::CachedChannel,
    event::{
        attend_reaction::ReactionEmoji,
        dates::event_is_over_at,
        document::new_event_document,
        post::{MESSAGE_MAX, event_post_buttons, event_post_content, thread_name_of},
    },
    records::{EventForum, EventPost},
    time::now_unix_seconds,
};

pub use super::event_lifecycle_edit::{rename_event_of_thread, save_event_document};

pub struct EventAsPosted {
    pub organizers: Vec<Id<UserMarker>>,
    pub document: Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventStart {
    Started,
    NotAnEventForum,
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

pub async fn start_event(bot: &Bot, thread: &CachedChannel, posted: Option<EventAsPosted>) -> anyhow::Result<EventStart> {
    let Some(forum_id) = thread.parent_id else {
        return Ok(EventStart::NotAnEventForum);
    };
    let Some(parent) = bot.channel(forum_id).await.filter(|parent| parent.kind == ChannelType::GuildForum) else {
        return Ok(EventStart::NotAnEventForum);
    };
    let Some(forum) = bot.db.event_forum_of(forum_id)? else {
        return Ok(EventStart::NotAnEventForum);
    };
    let attend = parent.default_reaction.as_ref();
    let (Some(attend_emoji), Some(attend_reaction)) = (attend, attend.and_then(ReactionEmoji::key)) else {
        bot.http
            .create_message(thread.id)
            .content("-# ❌ This forum has no default reaction, so there is nothing to react with to attend. Staff can set one in the forum's settings.")
            .await?;
        return Ok(EventStart::NoAttendReaction);
    };
    let posted = posted.unwrap_or_else(|| EventAsPosted {
        organizers: thread.owner_id.into_iter().collect(),
        document: new_event_document(&bot.links.event_document_url(thread.id), &thread.name),
    });
    let role_id = create_event_role(bot, thread).await?;
    bot.db.save_event(&EventPost {
        thread_id: thread.id,
        guild_id: thread.guild_id,
        forum_id: forum.forum_id,
        role_id,
        attend_reaction,
        organizers: posted.organizers.clone(),
        document: posted.document,
        updated_at: now_unix_seconds(),
    })?;
    tracing::info!(thread = %thread.id, role = %role_id, "Event started");

    let mut attendees: BTreeSet<Id<UserMarker>> = posted.organizers.into_iter().collect();
    if bot.message(thread.id, thread.id.cast()).await.is_some()
        && let Some(emoji) = request_reaction_of(attend_emoji)
    {
        attendees.extend(earlier_attendees(bot, thread, &emoji).await);
        bot.http.create_reaction(thread.id, thread.id.cast(), &emoji).await?;
        bot.http.create_pin(thread.id, thread.id.cast()).await?;
    }
    for attendee in attendees {
        change_event_role(bot, thread, RoleChange::Add, attendee, role_id).await?;
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
    Ok(EventStart::Started)
}

fn new_post_goes_to_review_note(register_command: &str) -> String {
    format!(
        "-# **Event not created**: A moderator must use {register_command} here to register it. Reviews are enabled for this forum but permissions were not set up correctly."
    )
}

async fn new_post_goes_to_review(bot: &Bot, thread: &CachedChannel) -> anyhow::Result<bool> {
    let Some(forum_id) = thread.parent_id else {
        return Ok(false);
    };
    let Some(settings) = bot.db.event_forum_of(forum_id)?.filter(EventForum::is_screened) else {
        return Ok(false);
    };
    let Some(channel) = bot.forum_of_guild(thread.guild_id, forum_id).await else {
        return Ok(false);
    };
    let poster = match thread.owner_id {
        Some(owner_id) => bot.member_roles(thread.guild_id, owner_id).await,
        None => None,
    };
    let forum = OpenEventForum { channel, settings };
    Ok(poster.is_none_or(|poster| bot.event_goes_to_review(&poster, &forum)))
}

pub async fn start_event_of_new_post(bot: &Bot, thread: &CachedChannel) -> anyhow::Result<()> {
    if new_post_goes_to_review(bot, thread).await? {
        let register_command = bot.command_mention(thread.guild_id, "event register").await;
        bot.http.create_message(thread.id).content(&new_post_goes_to_review_note(&register_command)).await?;
        return Ok(());
    }
    start_event(bot, thread, None).await.map(drop)
}

pub async fn post_new_event(
    bot: &Bot,
    forum: &CachedChannel,
    organizers: Vec<Id<UserMarker>>,
    document: &Value,
) -> anyhow::Result<Id<ChannelMarker>> {
    let content = event_post_content(document, &organizers, now_unix_seconds(), MESSAGE_MAX);
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
    let buttons = [event_post_buttons(&bot.links, thread.id, viewable)];
    bot.http
        .update_message(thread.id, posted.message.id)
        .components(Some(&buttons))
        .await?;
    let mut document = document.clone();
    document["id"] = Value::String(bot.links.event_document_url(thread.id));
    start_event(bot, &thread, Some(EventAsPosted { organizers, document })).await?;
    Ok(thread.id)
}

pub async fn end_event(bot: &Bot, guild_id: Id<GuildMarker>, thread_id: Id<ChannelMarker>) -> anyhow::Result<()> {
    let Some(event) = bot.db.event_of_thread(thread_id)? else {
        return Ok(());
    };
    let reason = format!("Event {thread_id} ended");
    if let Err(error) = bot.http.delete_role(guild_id, event.role_id).reason(&reason).await {
        tracing::warn!(thread = %thread_id, ?error, "Event role was not deleted");
    }
    bot.db.forget_event(thread_id)?;
    tracing::info!(thread = %thread_id, "Event ended");
    Ok(())
}

pub async fn end_events_that_are_over(bot: &Bot, now_unix_seconds: i64) -> anyhow::Result<()> {
    for event in bot.db.all_events()? {
        let over = event_is_over_at(&event.document).is_some_and(|over_at| over_at <= now_unix_seconds);
        if over && bot.cache.guild(event.guild_id).is_some() {
            end_event(bot, event.guild_id, event.thread_id).await?;
        }
    }
    Ok(())
}
