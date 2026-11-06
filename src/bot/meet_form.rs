use twilight_model::{
    channel::message::component::{SelectMenu, SelectMenuOption, SelectMenuType},
    channel::message::Component,
    guild::Permissions,
    id::{
        Id,
        marker::{ChannelMarker, UserMarker},
    },
};

use super::{
    Bot,
    access::{interaction_has_permission, member_roles_of_interaction},
    meet_form_modal::meet_form_modal,
    meet_request::meet_request_to_edit,
};
use crate::{
    discord::answer::{Answer, Reply},
    meet::{
        fields::fields_of_meet_document,
        post::{MEET_FORUM_PICK, MeetFormTarget},
    },
};

pub use super::meet_form_submit::submit_meet_form;

pub const NO_FORUM: &str = "There is no meet forum here that you can post in.";
pub const ONLY_HOST_OR_STAFF: &str = "Only the host or staff can edit this meet.";
const REVIEWED_HINT: &str = "The moderators check it before it is posted";

pub fn may_edit_meet(answer: &Answer<'_>, organizers: &[Id<UserMarker>]) -> bool {
    let interaction = answer.interaction;
    interaction.author_id().is_some_and(|user_id| organizers.contains(&user_id))
        || interaction_has_permission(interaction, Permissions::MANAGE_THREADS)
}

fn forum_picker(options: Vec<SelectMenuOption>) -> Component {
    crate::components::row(vec![Component::SelectMenu(SelectMenu {
        id: None,
        channel_types: None,
        custom_id: MEET_FORUM_PICK.to_owned(),
        default_values: None,
        disabled: false,
        kind: SelectMenuType::Text,
        max_values: None,
        min_values: None,
        options: Some(options),
        placeholder: Some("Pick a forum".to_owned()),
        required: None,
    })])
}

pub async fn offer_new_meet_form(bot: &Bot, answer: &Answer<'_>, picked: Option<Id<ChannelMarker>>) -> anyhow::Result<()> {
    let (Some(guild_id), Some(member)) = (answer.interaction.guild_id, member_roles_of_interaction(answer.interaction)) else {
        return Ok(());
    };
    let forums = bot.meet_forums_open_to(guild_id, &member).await?;
    if let Some(picked) = picked {
        return match forums.iter().find(|forum| forum.channel.id == picked) {
            Some(forum) => answer.show_modal(meet_form_modal(&MeetFormTarget::New(forum.channel.id), None)).await,
            None => answer.reply_privately(Reply::text(format!("<#{picked}> is not a meet forum you can post in."))).await,
        };
    }
    match forums.as_slice() {
        [] => answer.reply_privately(Reply::text(NO_FORUM)).await,
        [forum] => answer.show_modal(meet_form_modal(&MeetFormTarget::New(forum.channel.id), None)).await,
        _ => {
            let options = forums
                .iter()
                .take(25)
                .map(|forum| SelectMenuOption {
                    default: false,
                    description: bot.meet_goes_to_review(&member, forum).then(|| REVIEWED_HINT.to_owned()),
                    emoji: None,
                    label: forum.channel.name.clone(),
                    value: forum.channel.id.to_string(),
                })
                .collect();
            let reply = Reply { content: "Where should it go?".to_owned(), components: vec![forum_picker(options)] };
            answer.reply_privately(reply).await
        }
    }
}

pub async fn open_picked_meet_form(bot: &Bot, answer: &Answer<'_>, picked: Option<&str>) -> anyhow::Result<()> {
    let forum_id = picked.and_then(|value| value.parse::<Id<ChannelMarker>>().ok());
    let open = match (forum_id, answer.interaction.guild_id, member_roles_of_interaction(answer.interaction)) {
        (Some(forum_id), Some(guild_id), Some(member)) => bot.meet_forum_open_to(guild_id, &member, forum_id).await?,
        _ => None,
    };
    match open {
        Some(forum) => answer.show_modal(meet_form_modal(&MeetFormTarget::New(forum.channel.id), None)).await,
        None => answer.reply_privately(Reply::text(NO_FORUM)).await,
    }
}

pub async fn open_meet_form(bot: &Bot, answer: &Answer<'_>, target: &MeetFormTarget) -> anyhow::Result<()> {
    let Some(user_id) = answer.interaction.author_id() else {
        return Ok(());
    };
    let rejected = bot.rejected_draft_of(user_id, &target.key());
    match target {
        MeetFormTarget::New(_) => answer.show_modal(meet_form_modal(target, rejected)).await,
        MeetFormTarget::Request(request_id) => {
            let Some(request) = meet_request_to_edit(bot, answer, request_id).await? else {
                return Ok(());
            };
            let filled = rejected.unwrap_or_else(|| fields_of_meet_document(&request.document));
            answer.show_modal(meet_form_modal(target, Some(filled))).await
        }
        MeetFormTarget::Edit(thread_id) => {
            let meet = bot.db.meet_of_thread(*thread_id)?;
            let Some(meet) = meet.filter(|meet| may_edit_meet(answer, &meet.organizers)) else {
                return answer.reply_privately(Reply::text(ONLY_HOST_OR_STAFF)).await;
            };
            let filled = rejected.unwrap_or_else(|| fields_of_meet_document(&meet.document));
            answer.show_modal(meet_form_modal(target, Some(filled))).await
        }
    }
}
