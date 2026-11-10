pub mod access;
pub mod calendar;
pub mod clock;
pub mod commands;
mod event_edit;
mod event_form;
mod event_form_modal;
mod event_form_submit;
pub mod event_lifecycle;
mod event_lifecycle_edit;
mod event_request;
mod event_request_messages;
mod event_request_moderation;
mod event_roles;
pub mod guild_config;
mod interactions;
mod pins;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use twilight_gateway::{Event, Intents};
use twilight_http::Client;
use twilight_model::{
    channel::{Channel, message::Message},
    id::{
        Id,
        marker::{ApplicationMarker, ChannelMarker, GuildMarker, UserMarker},
    },
};

use crate::{
    config::Links,
    db::Database,
    discord::{
        bot_users::BotUsers,
        cache::{CachedChannel, GuildCache},
        permissions::MemberRoles,
    },
    grants::GrantStore,
    event::fields::EventFields,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigGrant {
    pub guild_id: Id<GuildMarker>,
    pub user_id: Id<UserMarker>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditGrant {
    pub thread_id: Id<ChannelMarker>,
    pub user_id: Id<UserMarker>,
}

pub struct Bot {
    pub http: Arc<Client>,
    pub application_id: Id<ApplicationMarker>,
    pub user_id: Id<UserMarker>,
    pub db: Database,
    pub cache: GuildCache,
    pub bot_users: BotUsers,
    pub event_clock: clock::EventClock,
    pub links: Links,
    pub config_grants: GrantStore<ConfigGrant>,
    pub edit_grants: GrantStore<EditGrant>,
    pending_thread_renames: event_lifecycle_edit::PendingThreadRenames,
    rejected_drafts: Mutex<HashMap<String, EventFields>>,
}

impl Bot {
    pub fn new(http: Client, application_id: Id<ApplicationMarker>, user_id: Id<UserMarker>, db: Database, links: Links) -> Self {
        Self {
            http: Arc::new(http),
            application_id,
            user_id,
            db,
            cache: GuildCache::default(),
            bot_users: BotUsers::default(),
            event_clock: clock::EventClock::default(),
            links,
            config_grants: GrantStore::default(),
            edit_grants: GrantStore::default(),
            pending_thread_renames: event_lifecycle_edit::PendingThreadRenames::default(),
            rejected_drafts: Mutex::default(),
        }
    }

    pub fn intents() -> Intents {
        Intents::GUILDS | Intents::GUILD_MESSAGE_REACTIONS | Intents::GUILD_MESSAGES
    }

    pub async fn channel(&self, channel_id: Id<ChannelMarker>) -> Option<CachedChannel> {
        if let Some(channel) = self.cache.channel(channel_id) {
            return Some(channel);
        }
        let channel: Channel = self.http.channel(channel_id).await.ok()?.model().await.ok()?;
        let cached = CachedChannel::of(channel.guild_id?, &channel);
        self.cache.insert_channel(cached.clone());
        Some(cached)
    }

    pub async fn member_roles(&self, guild_id: Id<GuildMarker>, user_id: Id<UserMarker>) -> Option<MemberRoles> {
        let member = self.http.guild_member(guild_id, user_id).await.ok()?.model().await.ok()?;
        Some(MemberRoles { user_id, role_ids: member.roles })
    }

    pub async fn message(&self, channel_id: Id<ChannelMarker>, message_id: Id<twilight_model::id::marker::MessageMarker>) -> Option<Message> {
        self.http.message(channel_id, message_id).await.ok()?.model().await.ok()
    }

    pub fn guild_name(&self, guild_id: Id<GuildMarker>) -> String {
        self.cache.guild(guild_id).map_or_else(|| "the server".to_owned(), |guild| guild.name)
    }

    pub async fn handle_event(&self, event: Event) -> anyhow::Result<()> {
        let thread_before = match &event {
            Event::ThreadUpdate(thread) => self.cache.channel(thread.id),
            _ => None,
        };
        self.cache.observe(&event);
        self.bot_users.observe(&event);
        match event {
            Event::InteractionCreate(interaction) => interactions::route(self, &interaction).await,
            Event::MessageCreate(message) => pins::pin_role_mention(self, &message).await,
            Event::ReactionAdd(reaction) => {
                let by_bot = reaction.member.as_ref().is_some_and(|member| member.user.bot);
                if by_bot || reaction.user_id == self.user_id {
                    return Ok(());
                }
                event_roles::change_attendance(self, event_roles::RoleChange::Add, &reaction).await
            }
            Event::ReactionRemove(reaction) => {
                if reaction.user_id == self.user_id || self.bot_users.is_bot(reaction.user_id) {
                    return Ok(());
                }
                event_roles::change_attendance(self, event_roles::RoleChange::Remove, &reaction).await
            }
            Event::ThreadCreate(thread) => {
                let Some(thread) = self.cache.channel(thread.id).filter(|_| thread.newly_created == Some(true)) else {
                    return Ok(());
                };
                if thread.owner_id == Some(self.user_id) {
                    return Ok(());
                }
                event_lifecycle::start_event_of_new_post(self, &thread).await
            }
            Event::ThreadDelete(thread) => event_lifecycle::end_event(self, thread.guild_id, thread.id).await,
            Event::ThreadUpdate(thread) => {
                let Some(after) = self.cache.channel(thread.id) else {
                    return Ok(());
                };
                let was_locked = thread_before.as_ref().is_some_and(|before| before.locked);
                if !was_locked && after.locked {
                    return event_lifecycle::end_event(self, after.guild_id, after.id).await;
                }
                if thread_before.is_none_or(|before| before.name != after.name) {
                    event_lifecycle::rename_event_of_thread(self, &after).await?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn rejected_draft_of(&self, user_id: Id<UserMarker>, target: &str) -> Option<EventFields> {
        self.drafts().get(&format!("{user_id}:{target}")).cloned()
    }

    pub fn reject_draft_of(&self, user_id: Id<UserMarker>, target: &str, fields: EventFields) {
        self.drafts().insert(format!("{user_id}:{target}"), fields);
    }

    pub fn forget_rejected_draft(&self, user_id: Id<UserMarker>, target: &str) {
        self.drafts().remove(&format!("{user_id}:{target}"));
    }

    fn drafts(&self) -> std::sync::MutexGuard<'_, HashMap<String, EventFields>> {
        self.rejected_drafts.lock().expect("draft lock poisoned")
    }
}
