use twilight_model::{
    application::interaction::modal::ModalInteractionData,
    id::{Id, marker::ChannelMarker},
};

use super::{
    Bot,
    access::member_roles_of_interaction,
    event_form::{ONLY_HOST_OR_STAFF, may_edit_event},
    event_form_modal::{fields_of_submitted_event_form, reject_draft},
    event_lifecycle::{post_new_event, save_event_document},
    event_request::{save_event_request_edit, send_event_request},
};
use crate::{
    discord::answer::{Answer, Reply},
    event::{
        document::new_event_document,
        fields::{EventFields, event_document_of_fields},
        post::EventFormTarget,
    },
};

pub struct SubmittedForm {
    pub target: EventFormTarget,
    pub fields: EventFields,
}

async fn submit_new_event(bot: &Bot, answer: &Answer<'_>, forum_id: Id<ChannelMarker>, form: SubmittedForm) -> anyhow::Result<()> {
    let base = new_event_document("urn:uuid:draft", &form.fields.name);
    let document = match event_document_of_fields(&form.fields, &base) {
        Ok(document) => document,
        Err(problem) => return reject_draft(bot, answer, &form.target, form.fields, &problem.message).await,
    };
    answer.defer_privately().await?;
    let open = match (answer.interaction.guild_id, member_roles_of_interaction(answer.interaction)) {
        (Some(guild_id), Some(member)) => bot
            .event_forum_open_to(guild_id, &member, forum_id)
            .await?
            .map(|forum| (member, forum)),
        _ => None,
    };
    let Some((member, forum)) = open else {
        return answer.edit_reply(Reply::text("That is not an event forum you can post in any more.")).await;
    };
    if bot.event_goes_to_review(&member, &forum) {
        return send_event_request(bot, answer, &forum, document, form).await;
    }
    bot.forget_rejected_draft(member.user_id, &form.target.key());
    let thread_id = post_new_event(bot, &forum.channel, vec![member.user_id], &document).await?;
    answer.edit_reply(Reply::text(format!("Posted: <#{thread_id}>"))).await
}

pub async fn submit_event_form(bot: &Bot, answer: &Answer<'_>, target: EventFormTarget, data: &ModalInteractionData) -> anyhow::Result<()> {
    let form = SubmittedForm { fields: fields_of_submitted_event_form(data), target: target.clone() };
    let thread_id = match target {
        EventFormTarget::New(forum_id) => return submit_new_event(bot, answer, forum_id, form).await,
        EventFormTarget::Request(request_id) => return save_event_request_edit(bot, answer, &request_id, form).await,
        EventFormTarget::Edit(thread_id) => thread_id,
    };
    let event = bot.db.event_of_thread(thread_id)?;
    let Some(event) = event.filter(|event| may_edit_event(answer, &event.organizers)) else {
        return answer.reply_privately(Reply::text(ONLY_HOST_OR_STAFF)).await;
    };
    let document = match event_document_of_fields(&form.fields, &event.document) {
        Ok(document) => document,
        Err(problem) => return reject_draft(bot, answer, &form.target, form.fields, &problem.message).await,
    };
    if let Some(user_id) = answer.interaction.author_id() {
        bot.forget_rejected_draft(user_id, &form.target.key());
    }
    answer.defer_privately().await?;
    let Some(thread) = bot.channel(thread_id).await.filter(|thread| thread.is_thread()) else {
        return answer.edit_reply(Reply::text("That event's post is gone.")).await;
    };
    save_event_document(bot, &thread, &event, document).await?;
    answer.edit_reply(Reply::text("Saved.")).await
}
