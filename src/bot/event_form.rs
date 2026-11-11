use twilight_model::{
    channel::message::component::{ButtonStyle, SelectMenu, SelectMenuOption, SelectMenuType},
    channel::message::Component,
    guild::Permissions,
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker, UserMarker},
    },
};

use super::{
    Bot,
    access::{OpenEventForum, interaction_has_permission, member_roles_of_interaction},
    event_form_modal::event_form_modal,
    event_request::event_request_to_edit,
};
use crate::{
    components::{button, row, text_display},
    discord::{
        answer::{Answer, Reply},
        permissions::MemberRoles,
    },
    event::{
        fields::fields_of_event_document,
        post::{EVENT_FORUM_PICK, EventFormTarget},
        request_view::host_requests_view,
    },
    records::EventRequest,
};

pub use super::event_form_submit::submit_event_form;

pub const NO_FORUM: &str = "There is no event forum here that you can post in.";
pub const ONLY_HOST_OR_STAFF: &str = "Only the host or staff can edit this event.";
const REVIEWED_HINT: &str = "The moderators check it before it is posted";
const WHERE_TO: &str = "Where should it go?";

pub fn may_edit_event(answer: &Answer<'_>, organizers: &[Id<UserMarker>]) -> bool {
    let interaction = answer.interaction;
    interaction.author_id().is_some_and(|user_id| organizers.contains(&user_id))
        || interaction_has_permission(interaction, Permissions::MANAGE_THREADS)
}

fn forum_picker(bot: &Bot, member: &MemberRoles, forums: &[OpenEventForum]) -> Component {
    let options = forums
        .iter()
        .take(25)
        .map(|forum| SelectMenuOption {
            default: false,
            description: bot.event_goes_to_review(member, forum).then(|| REVIEWED_HINT.to_owned()),
            emoji: None,
            label: forum.channel.name.clone(),
            value: forum.channel.id.to_string(),
        })
        .collect();
    row(vec![Component::SelectMenu(SelectMenu {
        id: None,
        channel_types: None,
        custom_id: EVENT_FORUM_PICK.to_owned(),
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

fn new_event_components(bot: &Bot, member: &MemberRoles, forums: &[OpenEventForum]) -> Vec<Component> {
    match forums {
        [] => vec![],
        [forum] => {
            let target = EventFormTarget::New(forum.channel.id);
            vec![row(vec![button(target.custom_id(), "New event", ButtonStyle::Primary)])]
        }
        _ => vec![text_display(WHERE_TO), forum_picker(bot, member, forums)],
    }
}

fn requests_only_followed_here(bot: &Bot, guild_id: Id<GuildMarker>, host_id: Id<UserMarker>) -> anyhow::Result<Vec<EventRequest>> {
    let mut requests = bot.db.event_requests_of_host(guild_id, host_id)?;
    requests.retain(|request| request.host_dm.is_none());
    Ok(requests)
}

pub async fn offer_new_event_form(bot: &Bot, answer: &Answer<'_>, picked: Option<Id<ChannelMarker>>) -> anyhow::Result<()> {
    let interaction = answer.interaction;
    let (Some(guild_id), Some(member), Some(user_id)) =
        (interaction.guild_id, member_roles_of_interaction(interaction), interaction.author_id())
    else {
        return Ok(());
    };
    let mut forums = bot.event_forums_open_to(guild_id, &member).await?;
    if let Some(picked) = picked {
        forums.retain(|forum| forum.channel.id == picked);
        if forums.is_empty() {
            return answer.reply_privately(Reply::text(format!("<#{picked}> is not an event forum you can post in."))).await;
        }
    }
    let pending = requests_only_followed_here(bot, guild_id, user_id)?;
    match (forums.as_slice(), pending.is_empty()) {
        ([], _) => answer.reply_privately(Reply::text(NO_FORUM)).await,
        ([forum], true) => answer.show_modal(event_form_modal(&EventFormTarget::New(forum.channel.id), None)).await,
        (_, true) => {
            let reply = Reply { content: WHERE_TO.to_owned(), components: vec![forum_picker(bot, &member, &forums)] };
            answer.reply_privately(reply).await
        }
        (_, false) => {
            let view = host_requests_view(&pending, &bot.guild_name(guild_id), new_event_components(bot, &member, &forums));
            answer.reply_privately(view.into()).await
        }
    }
}

pub async fn redraw_host_requests(bot: &Bot, answer: &Answer<'_>, guild_id: Id<GuildMarker>, host_id: Id<UserMarker>) -> anyhow::Result<()> {
    let Some(member) = member_roles_of_interaction(answer.interaction) else {
        return Ok(());
    };
    let forums = bot.event_forums_open_to(guild_id, &member).await?;
    let pending = requests_only_followed_here(bot, guild_id, host_id)?;
    let view = host_requests_view(&pending, &bot.guild_name(guild_id), new_event_components(bot, &member, &forums));
    answer.edit_reply(view.into()).await
}

pub async fn open_picked_event_form(bot: &Bot, answer: &Answer<'_>, picked: Option<&str>) -> anyhow::Result<()> {
    let forum_id = picked.and_then(|value| value.parse::<Id<ChannelMarker>>().ok());
    let open = match (forum_id, answer.interaction.guild_id, member_roles_of_interaction(answer.interaction)) {
        (Some(forum_id), Some(guild_id), Some(member)) => bot.event_forum_open_to(guild_id, &member, forum_id).await?,
        _ => None,
    };
    match open {
        Some(forum) => answer.show_modal(event_form_modal(&EventFormTarget::New(forum.channel.id), None)).await,
        None => answer.reply_privately(Reply::text(NO_FORUM)).await,
    }
}

pub async fn open_event_form(bot: &Bot, answer: &Answer<'_>, target: &EventFormTarget) -> anyhow::Result<()> {
    let Some(user_id) = answer.interaction.author_id() else {
        return Ok(());
    };
    let rejected = bot.rejected_draft_of(user_id, &target.key());
    match target {
        EventFormTarget::New(_) => answer.show_modal(event_form_modal(target, rejected)).await,
        EventFormTarget::Request(request_id) => {
            let Some(request) = event_request_to_edit(bot, answer, request_id).await? else {
                return Ok(());
            };
            let filled = rejected.unwrap_or_else(|| fields_of_event_document(&request.document));
            answer.show_modal(event_form_modal(target, Some(filled))).await
        }
        EventFormTarget::Edit(thread_id) => {
            let event = bot.db.event_of_thread(*thread_id)?;
            let Some(event) = event.filter(|event| may_edit_event(answer, &event.organizers)) else {
                return answer.reply_privately(Reply::text(ONLY_HOST_OR_STAFF)).await;
            };
            let filled = rejected.unwrap_or_else(|| fields_of_event_document(&event.document));
            answer.show_modal(event_form_modal(target, Some(filled))).await
        }
    }
}
