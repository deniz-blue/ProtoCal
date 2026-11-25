use std::sync::Arc;

use serde_json::Value;
use twilight_model::{
    channel::message::{AllowedMentions, Component, Message, MessageFlags},
    guild::Member,
    http::attachment::Attachment,
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker, UserMarker},
    },
};

use super::Bot;
use crate::{
    event::{
        dates::event_role_has_expired,
        organizer_picture::{OrganizerPicture, avatar_url_of, thumbnail_shows},
        post::{EventPostText, event_post_buttons, event_post_text},
        post_message::{event_post_components, event_post_is_outdated},
    },
    components::thumbnail_url_of,
    open_evnt::remote::fetch_image,
    records::EventPost,
    rgba_image::RgbaImage,
    time::now_unix_seconds,
};

pub struct EventPostMessage {
    pub text: EventPostText,
    pub picture: OrganizerPicture,
}

impl EventPostMessage {
    pub async fn components_and_upload(&self, posted: &[Component]) -> (Vec<Component>, Vec<Attachment>) {
        let wanted = self.picture.thumbnail_url();
        let with_thumbnail = |url: Option<&str>| event_post_components(&self.text, url);
        let Some(file_name) = self.picture.stack_file_name() else {
            return (with_thumbnail(wanted.as_deref()), vec![]);
        };
        let already_posted = thumbnail_url_of(posted).zip(wanted.as_deref()).is_some_and(|(posted, wanted)| thumbnail_shows(posted, wanted));
        if already_posted {
            return (with_thumbnail(wanted.as_deref()), vec![]);
        }
        match drawn_organizer_stack(&self.picture).await {
            Ok(png) => (with_thumbnail(wanted.as_deref()), vec![Attachment::from_bytes(file_name, png, 0)]),
            Err(error) => {
                tracing::warn!(?error, "Organizer picture was not drawn");
                (with_thumbnail(self.picture.avatar_urls.first().map(String::as_str)), vec![])
            }
        }
    }
}

async fn drawn_organizer_stack(picture: &OrganizerPicture) -> anyhow::Result<Vec<u8>> {
    let layout = picture.stack_layout().ok_or_else(|| anyhow::anyhow!("one organizer has no stack"))?;
    let mut avatars = Vec::new();
    for index in layout.shown_avatars() {
        let png = fetch_image(&picture.avatar_urls[index]).await.map_err(|problem| anyhow::anyhow!("avatar {problem}"))?;
        avatars.push(RgbaImage::of_png(&png)?);
    }
    layout.png_of(&avatars)
}

impl Bot {
    async fn member(&self, guild_id: Id<GuildMarker>, user_id: Id<UserMarker>) -> Option<Member> {
        self.http.guild_member(guild_id, user_id).await.ok()?.model().await.ok()
    }

    async fn organizer_picture(&self, guild_id: Id<GuildMarker>, organizers: &[Id<UserMarker>]) -> OrganizerPicture {
        let mut avatar_urls = Vec::new();
        for &user_id in organizers {
            avatar_urls.push(avatar_url_of(guild_id, user_id, self.member(guild_id, user_id).await.as_ref()));
        }
        OrganizerPicture { avatar_urls }
    }

    pub async fn event_post_message(
        &self,
        guild_id: Id<GuildMarker>,
        document: &Value,
        organizers: &[Id<UserMarker>],
        attend_reaction: Option<&str>,
        updated_at: i64,
    ) -> EventPostMessage {
        let part_of = self.part_of(document).await;
        let attend_reaction = attend_reaction.filter(|_| !event_role_has_expired(document, now_unix_seconds()));
        EventPostMessage {
            text: event_post_text(document, part_of.as_ref(), organizers, attend_reaction, updated_at),
            picture: self.organizer_picture(guild_id, organizers).await,
        }
    }

    async fn event_post_message_of(&self, event: &EventPost) -> EventPostMessage {
        self.event_post_message(event.guild_id, &event.document, &event.organizers, Some(&event.attend_reaction), event.updated_at).await
    }
}

async fn thread_is_archived(bot: &Bot, thread_id: Id<ChannelMarker>) -> bool {
    let Ok(response) = bot.http.channel(thread_id).await else {
        return false;
    };
    let channel = response.model().await.ok();
    channel.and_then(|channel| channel.thread_metadata).is_some_and(|metadata| metadata.archived)
}

// Discord refuses edits to messages in an archived thread.
pub async fn edit_event_post(bot: &Bot, thread_id: Id<ChannelMarker>, link_id: &str, message: &EventPostMessage, posted: &Message) -> anyhow::Result<()> {
    let (mut components, attachments) = message.components_and_upload(&posted.components).await;
    components.push(event_post_buttons(&bot.links, thread_id, link_id));
    let archived = thread_is_archived(bot, thread_id).await;
    if archived {
        bot.http.update_thread(thread_id).archived(false).await?;
    }
    let mentions = AllowedMentions::default();
    let edit = bot
        .http
        .update_message(thread_id, posted.id)
        .flags(MessageFlags::IS_COMPONENTS_V2)
        .content(Some(""))
        .components(Some(&components))
        .allowed_mentions(Some(&mentions));
    // Discord keeps a message's attachments when an edit leaves them out.
    let edited = match (attachments.is_empty(), message.picture.stack_file_name()) {
        (false, _) => edit.attachments(&attachments).await,
        (true, None) if !posted.attachments.is_empty() => edit.keep_attachment_ids(&[]).await,
        (true, _) => edit.await,
    };
    if archived {
        bot.http.update_thread(thread_id).archived(true).await?;
    }
    edited?;
    Ok(())
}

pub async fn update_event_post_if_outdated(bot: &Bot, event: &EventPost) {
    let Some(posted) = bot.message(event.thread_id, event.thread_id.cast()).await else {
        return;
    };
    if posted.author.id != bot.user_id {
        return;
    }
    let message = bot.event_post_message_of(event).await;
    let wanted = event_post_components(&message.text, message.picture.thumbnail_url().as_deref());
    if !event_post_is_outdated(posted.flags, &posted.components, &wanted, &bot.links.event_view_url(&event.link_id)) {
        return;
    }
    match edit_event_post(bot, event.thread_id, &event.link_id, &message, &posted).await {
        Ok(()) => tracing::info!(thread = %event.thread_id, "Event post updated"),
        Err(error) => tracing::warn!(thread = %event.thread_id, ?error, "Event post was not updated"),
    }
}

pub async fn update_outdated_event_posts(bot: Arc<Bot>) {
    let events = match bot.db.all_events() {
        Ok(events) => events,
        Err(error) => return tracing::error!(?error, "Events were not read to update their posts"),
    };
    for event in events {
        update_event_post_if_outdated(&bot, &event).await;
    }
}

pub async fn update_event_posts_of_organizer(bot: &Bot, guild_id: Id<GuildMarker>, user_id: Id<UserMarker>) -> anyhow::Result<()> {
    let events = bot.db.events_of_guild(guild_id)?;
    for event in events.iter().filter(|event| event.organizers.contains(&user_id)) {
        update_event_post_if_outdated(bot, event).await;
    }
    Ok(())
}
