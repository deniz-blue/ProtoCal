use std::{
    collections::HashMap,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use serde_json::{Value, json};
use twilight_model::{
    channel::message::AllowedMentions,
    id::{Id, marker::ChannelMarker},
};

use super::Bot;
use crate::{
    discord::cache::CachedChannel,
    event::{
        attend_reaction::role_name_of_event,
        change_notice::event_change_notice,
        post::{MESSAGE_MAX, event_post_buttons, event_post_content, thread_name_of},
    },
    records::EventPost,
    time::now_unix_seconds,
};

pub async fn rename_event_of_thread(bot: &Bot, thread: &CachedChannel) -> anyhow::Result<()> {
    if bot.pending_thread_renames.is_pending(thread.id) {
        return Ok(());
    }
    let Some(event) = bot.db.event_of_thread(thread.id)? else {
        return Ok(());
    };
    if thread_name_of(&event.document) == thread.name {
        return Ok(());
    }
    let mut document = event.document.clone();
    document["name"] = json!({ "und": thread.name });
    save_event_document(bot, thread, &event, document).await
}

#[derive(Clone, Default)]
pub struct PendingThreadRenames(Arc<RwLock<HashMap<Id<ChannelMarker>, String>>>);

impl PendingThreadRenames {
    fn start(&self, thread_id: Id<ChannelMarker>, name: &str) {
        self.write().insert(thread_id, name.to_owned());
    }

    fn finish(&self, thread_id: Id<ChannelMarker>, name: &str) {
        let mut renames = self.write();
        if renames.get(&thread_id).is_some_and(|latest| latest == name) {
            renames.remove(&thread_id);
        }
    }

    fn is_pending(&self, thread_id: Id<ChannelMarker>) -> bool {
        self.read().contains_key(&thread_id)
    }

    fn read(&self) -> RwLockReadGuard<'_, HashMap<Id<ChannelMarker>, String>> {
        self.0.read().expect("thread rename lock poisoned")
    }

    fn write(&self) -> RwLockWriteGuard<'_, HashMap<Id<ChannelMarker>, String>> {
        self.0.write().expect("thread rename lock poisoned")
    }
}

// Discord allows two renames of a channel per ten minutes, and twilight holds the request until the limit resets.
fn rename_thread_in_background(bot: &Bot, thread_id: Id<ChannelMarker>, name: String) {
    bot.pending_thread_renames.start(thread_id, &name);
    let http = Arc::clone(&bot.http);
    let pending = bot.pending_thread_renames.clone();
    tokio::spawn(async move {
        if let Err(error) = http.update_thread(thread_id).name(&name).await {
            tracing::warn!(thread = %thread_id, ?error, "Thread was not renamed");
        }
        pending.finish(thread_id, &name);
    });
}

pub async fn save_event_document(bot: &Bot, thread: &CachedChannel, event: &EventPost, document: Value) -> anyhow::Result<()> {
    let updated_at = now_unix_seconds();
    bot.db.describe_event(thread.id, &document, updated_at)?;
    let starter = bot.message(thread.id, thread.id.cast()).await;
    if starter.is_some_and(|starter| starter.author.id == bot.user_id) {
        let content = event_post_content(&document, &event.organizers, updated_at, MESSAGE_MAX);
        let viewable = bot.everyone_can_view_forum(thread.guild_id, event.forum_id).await;
        let buttons = [event_post_buttons(&bot.links, thread.id, viewable)];
        bot.http
            .update_message(thread.id, thread.id.cast())
            .content(Some(&content))
            .components(Some(&buttons))
            .allowed_mentions(Some(&AllowedMentions::default()))
            .await?;
    }
    let name = thread_name_of(&document);
    if thread.name != name {
        rename_thread_in_background(bot, thread.id, name.clone());
    }
    let role_name = role_name_of_event(&name);
    if bot.cache.role(thread.guild_id, event.role_id).is_none_or(|role| role.name != role_name)
        && let Err(error) = bot.http.update_role(thread.guild_id, event.role_id).name(Some(&role_name)).await
    {
        tracing::warn!(thread = %thread.id, ?error, "Event role was not renamed");
    }
    let Some(notice) = event_change_notice(&event.document, &document) else {
        return Ok(());
    };
    let ping = notice.pings_role.then(|| format!("<@&{}>", event.role_id));
    let content: Vec<String> = ping.into_iter().chain(notice.lines).collect();
    let mentions = AllowedMentions {
        roles: if notice.pings_role { vec![event.role_id] } else { vec![] },
        ..Default::default()
    };
    bot.http
        .create_message(thread.id)
        .content(&content.join("\n"))
        .allowed_mentions(Some(&mentions))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use twilight_model::id::Id;

    use super::PendingThreadRenames;

    #[test]
    fn a_rename_stays_pending_until_the_latest_one_finishes() {
        let pending = PendingThreadRenames::default();
        let thread = Id::new(1);
        pending.start(thread, "first");
        pending.start(thread, "second");
        pending.finish(thread, "first");
        assert!(pending.is_pending(thread));
        pending.finish(thread, "second");
        assert!(!pending.is_pending(thread));
    }
}
