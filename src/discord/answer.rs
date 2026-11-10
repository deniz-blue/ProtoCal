use std::sync::Mutex;

use twilight_http::Client;
use twilight_model::{
    application::interaction::Interaction,
    channel::message::{AllowedMentions, Component, MessageFlags},
    http::interaction::{InteractionResponse, InteractionResponseData, InteractionResponseType},
    id::{Id, marker::ApplicationMarker},
};

use crate::components::message_flags_of;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reply {
    pub content: String,
    pub components: Vec<Component>,
}

impl Reply {
    pub fn text(content: impl Into<String>) -> Self {
        Self { content: content.into(), components: vec![] }
    }
}

impl From<crate::event::request_view::MessageView> for Reply {
    fn from(view: crate::event::request_view::MessageView) -> Self {
        Self { content: String::new(), components: view.components }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnswerState {
    Unanswered,
    Replied,
    DeferredReply,
    DeferredUpdate,
}

pub struct Answer<'a> {
    http: &'a Client,
    application_id: Id<ApplicationMarker>,
    pub interaction: &'a Interaction,
    state: Mutex<AnswerState>,
}

impl<'a> Answer<'a> {
    pub fn new(http: &'a Client, application_id: Id<ApplicationMarker>, interaction: &'a Interaction) -> Self {
        Self { http, application_id, interaction, state: Mutex::new(AnswerState::Unanswered) }
    }

    fn state(&self) -> AnswerState {
        *self.state.lock().expect("answer state lock poisoned")
    }

    fn set_state(&self, state: AnswerState) {
        *self.state.lock().expect("answer state lock poisoned") = state;
    }

    async fn respond(&self, kind: InteractionResponseType, data: Option<InteractionResponseData>) -> anyhow::Result<()> {
        let response = InteractionResponse { kind, data };
        self.http
            .interaction(self.application_id)
            .create_response(self.interaction.id, &self.interaction.token, &response)
            .await?;
        Ok(())
    }

    pub async fn reply_privately(&self, reply: Reply) -> anyhow::Result<()> {
        let data = InteractionResponseData {
            content: Some(reply.content).filter(|content| !content.is_empty()),
            flags: Some(MessageFlags::EPHEMERAL | message_flags_of(&reply.components)),
            components: Some(reply.components),
            allowed_mentions: Some(AllowedMentions::default()),
            ..Default::default()
        };
        self.respond(InteractionResponseType::ChannelMessageWithSource, Some(data)).await?;
        self.set_state(AnswerState::Replied);
        Ok(())
    }

    pub async fn defer_privately(&self) -> anyhow::Result<()> {
        let data = InteractionResponseData { flags: Some(MessageFlags::EPHEMERAL), ..Default::default() };
        self.respond(InteractionResponseType::DeferredChannelMessageWithSource, Some(data)).await?;
        self.set_state(AnswerState::DeferredReply);
        Ok(())
    }

    pub async fn defer_update(&self) -> anyhow::Result<()> {
        self.respond(InteractionResponseType::DeferredUpdateMessage, None).await?;
        self.set_state(AnswerState::DeferredUpdate);
        Ok(())
    }

    pub async fn show_modal(&self, modal: InteractionResponseData) -> anyhow::Result<()> {
        self.respond(InteractionResponseType::Modal, Some(modal)).await?;
        self.set_state(AnswerState::Replied);
        Ok(())
    }

    pub async fn edit_reply(&self, reply: Reply) -> anyhow::Result<()> {
        let mentions = AllowedMentions::default();
        let interaction = self.http.interaction(self.application_id);
        let update = interaction
            .update_response(&self.interaction.token)
            .content(Some(reply.content.as_str()).filter(|content| !content.is_empty()))
            .components(Some(&reply.components))
            .allowed_mentions(Some(&mentions));
        let flags = message_flags_of(&reply.components);
        if flags.is_empty() { update.await? } else { update.flags(flags).await? };
        Ok(())
    }

    pub async fn follow_up_privately(&self, reply: Reply) -> anyhow::Result<()> {
        let mentions = AllowedMentions::default();
        let interaction = self.http.interaction(self.application_id);
        let followup = interaction
            .create_followup(&self.interaction.token)
            .flags(MessageFlags::EPHEMERAL | message_flags_of(&reply.components))
            .components(&reply.components)
            .allowed_mentions(Some(&mentions));
        if reply.content.is_empty() { followup.await? } else { followup.content(&reply.content).await? };
        Ok(())
    }

    pub async fn tell(&self, content: impl Into<String>) -> anyhow::Result<()> {
        let reply = Reply::text(content);
        if self.state() == AnswerState::Unanswered {
            self.reply_privately(reply).await
        } else {
            self.follow_up_privately(reply).await
        }
    }

    pub async fn reply_or_edit(&self, reply: Reply) -> anyhow::Result<()> {
        if self.state() == AnswerState::DeferredReply {
            self.edit_reply(reply).await
        } else {
            self.reply_privately(reply).await
        }
    }

    pub async fn answer_failure(&self) -> anyhow::Result<()> {
        let content = "❌ An unexpected error occurred.";
        match self.state() {
            AnswerState::Unanswered => self.reply_privately(Reply::text(content)).await,
            AnswerState::DeferredReply => self.edit_reply(Reply::text(content)).await,
            AnswerState::Replied | AnswerState::DeferredUpdate => {
                self.follow_up_privately(Reply::text(content)).await
            }
        }
    }
}
