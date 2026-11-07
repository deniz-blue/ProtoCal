use twilight_model::id::{Id, marker::ChannelMarker};

use super::{
    Bot, EditGrant,
    event_form::{ONLY_HOST_OR_STAFF, may_edit_event},
};
use crate::{
    discord::answer::{Answer, Reply},
    event::post::event_edit_buttons,
};

pub async fn answer_edit_button(bot: &Bot, answer: &Answer<'_>, thread_id: Id<ChannelMarker>) -> anyhow::Result<()> {
    let event = bot.db.event_of_thread(thread_id)?;
    let editable = event.filter(|event| may_edit_event(answer, &event.organizers));
    let (Some(_), Some(user_id)) = (editable, answer.interaction.author_id()) else {
        return answer.reply_privately(Reply::text(ONLY_HOST_OR_STAFF)).await;
    };
    let token = bot.edit_grants.grant(EditGrant { thread_id, user_id });
    let edit_url = bot.links.edit_link_url(&token);
    let reply = Reply { content: String::new(), components: vec![event_edit_buttons(thread_id, edit_url)] };
    answer.reply_privately(reply).await
}
