use twilight_model::{
    channel::message::AllowedMentions,
    id::{Id, marker::ChannelMarker},
};

use super::Bot;
use crate::{
    discord::answer::Answer,
    event::request_view::{EventRequestOutcome, MessageView, event_request_card, event_request_host_view},
    records::{EventRequest, MessageAt},
};

async fn send_view(bot: &Bot, at: MessageAt, view: &MessageView) -> anyhow::Result<()> {
    bot.http
        .update_message(at.channel_id, at.message_id)
        .content(Some(&view.content))
        .components(Some(&view.components))
        .allowed_mentions(Some(&AllowedMentions::default()))
        .await?;
    Ok(())
}

pub async fn post_view(bot: &Bot, channel_id: Id<ChannelMarker>, view: &MessageView) -> anyhow::Result<MessageAt> {
    let message = bot
        .http
        .create_message(channel_id)
        .content(&view.content)
        .components(&view.components)
        .allowed_mentions(Some(&AllowedMentions::default()))
        .await?
        .model()
        .await?;
    Ok(MessageAt { channel_id, message_id: message.id })
}

pub async fn edit_card(bot: &Bot, request: &EventRequest, outcome: EventRequestOutcome) {
    let Some(message_id) = request.card_message_id else {
        return;
    };
    let at = MessageAt { channel_id: request.card_channel_id, message_id };
    if let Err(error) = send_view(bot, at, &event_request_card(request, outcome)).await {
        tracing::warn!(request = %request.id, ?error, "Event request card was not edited");
    }
}

pub async fn edit_host_dm(bot: &Bot, request: &EventRequest, outcome: EventRequestOutcome) {
    let Some(dm) = request.host_dm else {
        return;
    };
    let view = event_request_host_view(request, outcome, &bot.guild_name(request.guild_id));
    if let Err(error) = send_view(bot, dm, &view).await {
        tracing::warn!(request = %request.id, ?error, "Event request DM was not edited");
    }
}

pub async fn show_event_request(bot: &Bot, answer: &Answer<'_>, request: &EventRequest, outcome: EventRequestOutcome) -> anyhow::Result<()> {
    edit_card(bot, request, outcome).await;
    if request.host_dm.is_some() {
        edit_host_dm(bot, request, outcome).await;
    } else if answer.interaction.author_id() == Some(request.host_id) {
        let view = event_request_host_view(request, outcome, &bot.guild_name(request.guild_id));
        answer.edit_reply(view.into()).await?;
    }
    Ok(())
}
