use serde_json::{Value, json};
use twilight_model::channel::message::AllowedMentions;

use super::Bot;
use crate::{
    discord::cache::CachedChannel,
    meet::{
        attend_reaction::role_name_of_meet,
        change_notice::meet_change_notice,
        post::{MESSAGE_MAX, meet_post_buttons, meet_post_content, thread_name_of},
    },
    records::Meet,
    time::now_unix_seconds,
};

pub async fn rename_meet_of_thread(bot: &Bot, thread: &CachedChannel) -> anyhow::Result<()> {
    let Some(meet) = bot.db.meet_of_thread(thread.id)? else {
        return Ok(());
    };
    if thread_name_of(&meet.document) == thread.name {
        return Ok(());
    }
    let mut document = meet.document.clone();
    document["name"] = json!({ "und": thread.name });
    save_meet_document(bot, thread, &meet, document).await
}

pub async fn save_meet_document(bot: &Bot, thread: &CachedChannel, meet: &Meet, document: Value) -> anyhow::Result<()> {
    let updated_at = now_unix_seconds();
    bot.db.describe_meet(thread.id, &document, updated_at)?;
    let starter = bot.message(thread.id, thread.id.cast()).await;
    if starter.is_some_and(|starter| starter.author.id == bot.user_id) {
        let content = meet_post_content(&document, &meet.organizers, updated_at, MESSAGE_MAX);
        let viewable = bot.everyone_can_view_forum(thread.guild_id, meet.forum_id).await;
        let buttons = [meet_post_buttons(&bot.links, thread.id, viewable)];
        bot.http
            .update_message(thread.id, thread.id.cast())
            .content(Some(&content))
            .components(Some(&buttons))
            .allowed_mentions(Some(&AllowedMentions::default()))
            .await?;
    }
    let name = thread_name_of(&document);
    if thread.name != name {
        bot.http.update_thread(thread.id).name(&name).await?;
    }
    if thread_name_of(&meet.document) != name {
        let role_name = role_name_of_meet(&name);
        if let Err(error) = bot.http.update_role(thread.guild_id, meet.role_id).name(Some(&role_name)).await {
            tracing::warn!(thread = %thread.id, ?error, "Meet role was not renamed");
        }
    }
    let Some(notice) = meet_change_notice(&meet.document, &document) else {
        return Ok(());
    };
    let ping = notice.pings_role.then(|| format!("<@&{}>", meet.role_id));
    let content: Vec<String> = ping.into_iter().chain(notice.lines).collect();
    let mentions = AllowedMentions {
        roles: if notice.pings_role { vec![meet.role_id] } else { Vec::new() },
        ..Default::default()
    };
    bot.http
        .create_message(thread.id)
        .content(&content.join("\n"))
        .allowed_mentions(Some(&mentions))
        .await?;
    Ok(())
}
