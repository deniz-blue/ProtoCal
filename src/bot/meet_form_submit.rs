use twilight_model::{
    application::interaction::modal::ModalInteractionData,
    id::{Id, marker::ChannelMarker},
};

use super::{
    Bot,
    access::member_roles_of_interaction,
    meet_form::{ONLY_HOST_OR_STAFF, may_edit_meet},
    meet_form_modal::{fields_of_submitted_meet_form, reject_draft},
    meet_lifecycle::{post_new_meet, save_meet_document},
    meet_request::{save_meet_request_edit, send_meet_request},
};
use crate::{
    discord::answer::{Answer, Reply},
    meet::{
        document::new_meet_document,
        fields::{MeetFields, meet_document_of_fields},
        post::MeetFormTarget,
    },
};

pub struct SubmittedForm {
    pub target: MeetFormTarget,
    pub fields: MeetFields,
}

async fn submit_new_meet(bot: &Bot, answer: &Answer<'_>, forum_id: Id<ChannelMarker>, form: SubmittedForm) -> anyhow::Result<()> {
    let base = new_meet_document("urn:uuid:draft", &form.fields.name);
    let document = match meet_document_of_fields(&form.fields, &base) {
        Ok(document) => document,
        Err(problem) => return reject_draft(bot, answer, &form.target, form.fields, &problem.message).await,
    };
    answer.defer_privately().await?;
    let open = match (answer.interaction.guild_id, member_roles_of_interaction(answer.interaction)) {
        (Some(guild_id), Some(member)) => bot
            .meet_forum_open_to(guild_id, &member, forum_id)
            .await?
            .map(|forum| (member, forum)),
        _ => None,
    };
    let Some((member, forum)) = open else {
        return answer.edit_reply(Reply::text("That is not a meet forum you can post in any more.")).await;
    };
    if bot.meet_goes_to_review(&member, &forum) {
        return send_meet_request(bot, answer, &forum, document, form).await;
    }
    bot.forget_rejected_draft(member.user_id, &form.target.key());
    let thread_id = post_new_meet(bot, &forum.channel, vec![member.user_id], &document).await?;
    answer.edit_reply(Reply::text(format!("Posted: <#{thread_id}>"))).await
}

pub async fn submit_meet_form(bot: &Bot, answer: &Answer<'_>, target: MeetFormTarget, data: &ModalInteractionData) -> anyhow::Result<()> {
    let form = SubmittedForm { fields: fields_of_submitted_meet_form(data), target: target.clone() };
    let thread_id = match target {
        MeetFormTarget::New(forum_id) => return submit_new_meet(bot, answer, forum_id, form).await,
        MeetFormTarget::Request(request_id) => return save_meet_request_edit(bot, answer, &request_id, form).await,
        MeetFormTarget::Edit(thread_id) => thread_id,
    };
    let meet = bot.db.meet_of_thread(thread_id)?;
    let Some(meet) = meet.filter(|meet| may_edit_meet(answer, &meet.organizers)) else {
        return answer.reply_privately(Reply::text(ONLY_HOST_OR_STAFF)).await;
    };
    let document = match meet_document_of_fields(&form.fields, &meet.document) {
        Ok(document) => document,
        Err(problem) => return reject_draft(bot, answer, &form.target, form.fields, &problem.message).await,
    };
    if let Some(user_id) = answer.interaction.author_id() {
        bot.forget_rejected_draft(user_id, &form.target.key());
    }
    answer.defer_privately().await?;
    let Some(thread) = bot.channel(thread_id).await.filter(|thread| thread.is_thread()) else {
        return answer.edit_reply(Reply::text("That meet's post is gone.")).await;
    };
    save_meet_document(bot, &thread, &meet, document).await?;
    answer.edit_reply(Reply::text("Saved.")).await
}
