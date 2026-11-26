use twilight_http::request::AuditLogReason as _;
use std::collections::BTreeSet;

use serde_json::Value;
use twilight_http::request::channel::reaction::RequestReactionType;
use twilight_model::{
    channel::{
        ChannelType,
        message::{AllowedMentions, MessageFlags},
    },
    id::{
        Id,
        marker::{ChannelMarker, EmojiMarker, UserMarker},
    },
};

use super::{
    Bot,
    access::OpenEventForum,
    event_post::{edit_event_post, update_event_post_if_outdated},
    event_roles::{RoleChange, change_event_role, create_event_role},
};
use crate::{
    discord::cache::CachedChannel,
    event::{
        attend_reaction::ReactionEmoji,
        dates::event_role_has_expired,
        document::new_event_document,
        post::thread_name_of,
    },
    grants::random_link_id,
    records::{EventForum, EventPost},
    time::now_unix_seconds,
};

pub use super::{
    event_lifecycle_edit::{rename_event_of_thread, save_event_document},
    event_post::{update_event_posts_of_organizer, update_outdated_event_posts},
};

pub struct EventAsPosted {
    pub organizers: Vec<Id<UserMarker>>,
    pub document: Value,
    pub link_id: String,
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
        return vec![];
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
    let posted = posted.unwrap_or_else(|| {
        let link_id = random_link_id();
        EventAsPosted {
            organizers: thread.owner_id.into_iter().collect(),
            document: new_event_document(&bot.links.event_document_url(&link_id), &thread.name),
            link_id,
        }
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
        link_id: posted.link_id,
    })?;
    tracing::info!(thread = %thread.id, role = %role_id, "Event started");

    let mut attendees: BTreeSet<Id<UserMarker>> = posted.organizers.into_iter().collect();
    if bot.message(thread.id, thread.id.cast()).await.is_some()
        && let Some(emoji) = request_reaction_of(attend_emoji)
    {
        attendees.extend(earlier_attendees(bot, thread, &emoji).await);
        bot.http.create_pin(thread.id, thread.id.cast()).await?;
    }
    for attendee in attendees {
        change_event_role(bot, thread, RoleChange::Add, attendee, role_id).await?;
    }

    if let Some(announce_role_id) = forum.announce_role_id {
        let mentions = AllowedMentions { roles: vec![announce_role_id], ..Default::default() };
        let (channel_id, content) = match forum.announce_channel_id {
            Some(channel_id) => (channel_id, format!("<@&{announce_role_id}> <#{}>", thread.id)),
            None => (thread.id, format!("<@&{announce_role_id}>")),
        };
        if let Err(error) = bot.http.create_message(channel_id).content(&content).allowed_mentions(Some(&mentions)).await {
            tracing::warn!(thread = %thread.id, channel = %channel_id, %error, "Could not announce the event");
        }
    }
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
    let attend_reaction = forum.default_reaction.as_ref().and_then(ReactionEmoji::key);
    let message = bot
        .event_post_message(forum.guild_id, document, &organizers, attend_reaction.as_deref(), now_unix_seconds())
        .await;
    let (components, attachments) = message.components_and_upload(&[]).await;
    let mentions = AllowedMentions::default();
    let posted = bot
        .http
        .create_forum_thread(forum.id, &thread_name_of(document))
        .message()
        .flags(MessageFlags::IS_COMPONENTS_V2)
        .components(&components)
        .attachments(&attachments)
        .allowed_mentions(Some(&mentions))
        .await?
        .model()
        .await?;
    let thread = CachedChannel::of(forum.guild_id, &posted.channel);
    bot.cache.insert_channel(thread.clone());
    let link_id = random_link_id();
    edit_event_post(bot, thread.id, &link_id, &message, &posted.message).await?;
    let mut document = document.clone();
    document["id"] = Value::String(bot.links.event_document_url(&link_id));
    match start_event(bot, &thread, Some(EventAsPosted { organizers, document, link_id })).await {
        Ok(EventStart::Started) => Ok(thread.id),
        outcome => {
            bot.http.delete_channel(thread.id).await?;
            anyhow::bail!("Event {} was not started: {outcome:?}", thread.id)
        }
    }
}

pub fn forum_has_attend_reaction(forum: &CachedChannel) -> bool {
    forum.default_reaction.as_ref().and_then(ReactionEmoji::key).is_some()
}

async fn delete_role_of_event(bot: &Bot, event: &EventPost, reason: &str) {
    match bot.http.delete_role(event.guild_id, event.role_id).reason(reason).await {
        Ok(_) => tracing::info!(thread = %event.thread_id, role = %event.role_id, "Event role deleted"),
        Err(error) => tracing::warn!(thread = %event.thread_id, ?error, "Event role was not deleted"),
    }
}

pub async fn delete_event(bot: &Bot, thread_id: Id<ChannelMarker>) -> anyhow::Result<()> {
    let Some(event) = bot.db.event_of_thread(thread_id)? else {
        return Ok(());
    };
    delete_role_of_event(bot, &event, &format!("Event {thread_id} was deleted")).await;
    bot.db.forget_event(thread_id)?;
    tracing::info!(thread = %thread_id, "Event deleted");
    Ok(())
}

pub async fn delete_event_role(bot: &Bot, thread_id: Id<ChannelMarker>) -> anyhow::Result<()> {
    if let Some(event) = bot.db.event_of_thread(thread_id)? {
        delete_role_of_event(bot, &event, &format!("Event {thread_id} was locked")).await;
    }
    Ok(())
}

pub async fn delete_expired_event_roles(bot: &Bot, now_unix_seconds: i64) -> anyhow::Result<()> {
    for event in bot.db.all_events()? {
        let role_exists = bot.cache.role(event.guild_id, event.role_id).is_some();
        if role_exists && event_role_has_expired(&event.document, now_unix_seconds) {
            delete_role_of_event(bot, &event, &format!("Event {} was a week ago", event.thread_id)).await;
            update_event_post_if_outdated(bot, &event).await;
        }
    }
    Ok(())
}
