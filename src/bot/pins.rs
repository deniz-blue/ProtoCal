use std::sync::LazyLock;

use regex::Regex;
use twilight_model::{channel::message::Message, gateway::payload::incoming::MessageCreate};

use super::Bot;

static ONLY_ROLE_MENTIONS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\s*<@&\d+>)+\s*$").expect("role mention pattern"));

async fn replied_to_message(bot: &Bot, message: &Message) -> Option<Message> {
    if let Some(replied_to) = &message.referenced_message {
        return Some(replied_to.as_ref().clone());
    }
    let reference = message.reference.as_ref()?;
    bot.message(reference.channel_id?, reference.message_id?).await
}

pub async fn pin_role_mention(bot: &Bot, message: &MessageCreate) -> anyhow::Result<()> {
    if message.author.bot || message.mention_roles.is_empty() {
        return Ok(());
    }
    let Some(event) = bot.db.event_of_thread(message.channel_id)? else {
        return Ok(());
    };
    if !message.mention_roles.contains(&event.role_id) {
        return Ok(());
    }
    let only_the_ping = ONLY_ROLE_MENTIONS.is_match(&message.content);
    if let Some(replied_to) = replied_to_message(bot, message).await
        && (only_the_ping || replied_to.poll.is_some() || replied_to.author.bot)
    {
        bot.http.create_pin(replied_to.channel_id, replied_to.id).await?;
        if only_the_ping {
            return Ok(());
        }
    }
    bot.http.create_pin(message.channel_id, message.id).await?;
    Ok(())
}
