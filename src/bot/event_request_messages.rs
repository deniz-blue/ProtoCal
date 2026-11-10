use twilight_http::{Client, request::channel::message::CreateMessage};
use twilight_model::{
    channel::message::AllowedMentions,
    id::{Id, marker::ChannelMarker},
};

use super::Bot;
use crate::{
    components::message_flags_of,
    discord::answer::Answer,
    event::request_view::{EventRequestOutcome, MessageView, event_request_card, event_request_host_view},
    records::{EventRequest, MessageAt},
};

async fn send_view(bot: &Bot, at: MessageAt, view: &MessageView) -> anyhow::Result<()> {
    bot.http
        .update_message(at.channel_id, at.message_id)
        .content(None)
        .components(Some(&view.components))
        .flags(message_flags_of(&view.components))
        .allowed_mentions(Some(&AllowedMentions::default()))
        .await?;
    Ok(())
}

fn new_view_message<'a>(http: &'a Client, channel_id: Id<ChannelMarker>, view: &'a MessageView) -> CreateMessage<'a> {
    http.create_message(channel_id).flags(message_flags_of(&view.components)).components(&view.components)
}

pub async fn post_view(bot: &Bot, channel_id: Id<ChannelMarker>, view: &MessageView) -> anyhow::Result<MessageAt> {
    let message = new_view_message(&bot.http, channel_id, view)
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

async fn repost_card(bot: &Bot, request: &EventRequest) -> anyhow::Result<()> {
    let card = event_request_card(request, EventRequestOutcome::Open);
    let posted = match post_view(bot, request.card_channel_id, &card).await {
        Ok(posted) => posted,
        Err(error) => {
            tracing::warn!(request = %request.id, ?error, "Event request card was not posted again");
            edit_card(bot, request, EventRequestOutcome::Open).await;
            return Ok(());
        }
    };
    if bot.db.move_event_request_card(&request.id, posted.message_id)?.is_none() {
        let _ = bot.http.delete_message(posted.channel_id, posted.message_id).await;
        return Ok(());
    }
    if let Some(old) = request.card_message_id
        && let Err(error) = bot.http.delete_message(request.card_channel_id, old).await
    {
        tracing::warn!(request = %request.id, ?error, "Old event request card was not deleted");
    }
    Ok(())
}

pub async fn show_host_edited_event_request(bot: &Bot, answer: &Answer<'_>, request: &EventRequest) -> anyhow::Result<()> {
    repost_card(bot, request).await?;
    show_to_host(bot, answer, request, EventRequestOutcome::Open).await
}

pub async fn show_event_request(bot: &Bot, answer: &Answer<'_>, request: &EventRequest, outcome: EventRequestOutcome) -> anyhow::Result<()> {
    edit_card(bot, request, outcome).await;
    show_to_host(bot, answer, request, outcome).await
}

async fn show_to_host(bot: &Bot, answer: &Answer<'_>, request: &EventRequest, outcome: EventRequestOutcome) -> anyhow::Result<()> {
    if request.host_dm.is_some() {
        edit_host_dm(bot, request, outcome).await;
    } else if answer.interaction.author_id() == Some(request.host_id) {
        let view = event_request_host_view(request, outcome, &bot.guild_name(request.guild_id));
        answer.edit_reply(view.into()).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use twilight_http::{Client, request::TryIntoRequest};
    use twilight_model::id::Id;

    use super::new_view_message;
    use crate::{
        components::{container, text_display},
        event::request_view::MessageView,
    };

    #[tokio::test]
    async fn a_view_with_a_container_passes_twilights_checks() {
        let http = Client::new(String::new());
        let view = MessageView { components: vec![text_display("head"), container(vec![text_display("event")])] };
        assert!(new_view_message(&http, Id::new(1), &view).try_into_request().is_ok());
    }
}
