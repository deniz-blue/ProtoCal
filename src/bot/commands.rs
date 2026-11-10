use twilight_interactions::command::{CommandInputData, CommandModel, CommandOption, CreateCommand, CreateOption};
use twilight_model::{
    application::{command::Command, interaction::application_command::CommandData},
    channel::message::component::ButtonStyle,
    guild::Permissions,
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker},
    },
};

use super::{
    Bot, ConfigGrant,
    access::interaction_has_permission,
    guild_config::may_configure_guild,
    event_form::offer_new_event_form,
    event_lifecycle::{EventStart, start_event},
};
use crate::{
    components::{emoji_of_text, link_button, row},
    discord::{
        answer::{Answer, Reply},
        errors::{INVALID_FORM_BODY, is_discord_error},
    },
    event::post::NewEventButton,
};

#[derive(CommandModel, CreateCommand)]
#[command(
    name = "config",
    desc = "Open this server's ProtoCal settings",
    default_permissions = "manage_guild",
    contexts = "guild"
)]
pub struct OpenConfig;

fn manage_guild() -> Permissions {
    Permissions::MANAGE_GUILD
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "event", desc = "Events", contexts = "guild")]
pub enum EventCommand {
    #[command(name = "new")]
    New(PostEvent),
    #[command(name = "register")]
    Register(RegisterPost),
    #[command(name = "send-new-button")]
    SendNewButton(SendNewEventButton),
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "new", desc = "Post an event")]
pub struct PostEvent {
    #[command(
        desc = "Where to post it; without it you pick from the forums you can post in",
        channel_types = "guild_forum"
    )]
    forum: Option<Id<ChannelMarker>>,
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "register", desc = "Make this post, made by hand, an event")]
pub struct RegisterPost;

#[derive(CommandModel, CreateCommand)]
#[command(name = "send-new-button", desc = "Post a button here that opens the event form")]
pub struct SendNewEventButton {
    #[command(desc = "The forum the button posts to; without it, people pick", channel_types = "guild_forum")]
    forum: Option<Id<ChannelMarker>>,
    #[command(desc = "Button text, New event if left out", min_length = 1, max_length = 80)]
    text: Option<String>,
    #[command(desc = "A standard emoji or one from this server")]
    emoji: Option<String>,
    #[command(desc = "Button color, blurple if left out")]
    color: Option<ButtonColor>,
}

#[derive(CommandOption, CreateOption)]
pub enum ButtonColor {
    #[option(name = "Blurple", value = "blurple")]
    Blurple,
    #[option(name = "Grey", value = "grey")]
    Grey,
    #[option(name = "Green", value = "green")]
    Green,
    #[option(name = "Red", value = "red")]
    Red,
}

impl ButtonColor {
    fn style(&self) -> ButtonStyle {
        match self {
            Self::Blurple => ButtonStyle::Primary,
            Self::Grey => ButtonStyle::Secondary,
            Self::Green => ButtonStyle::Success,
            Self::Red => ButtonStyle::Danger,
        }
    }
}

#[derive(CommandModel, CreateCommand)]
#[command(name = "mycalendar", desc = "Link to the events you joined", contexts = "guild bot_dm")]
pub struct MyCalendar {
    #[command(rename = "new", desc = "Replace your link")]
    replace: Option<bool>,
}

pub fn all_commands() -> Vec<Command> {
    vec![
        OpenConfig::create_command().into(),
        EventCommand::create_command().into(),
        MyCalendar::create_command().into(),
    ]
}

pub async fn run_command(bot: &Bot, answer: &Answer<'_>, data: &CommandData) -> anyhow::Result<()> {
    let input = CommandInputData::from(data.clone());
    match data.name.as_str() {
        "config" => open_config(bot, answer).await,
        "event" => match EventCommand::from_interaction(input)? {
            EventCommand::New(PostEvent { forum }) => offer_new_event_form(bot, answer, forum).await,
            EventCommand::Register(RegisterPost) => register_post(bot, answer).await,
            EventCommand::SendNewButton(options) => send_new_event_button(bot, answer, options).await,
        },
        "mycalendar" => open_calendar(bot, answer, MyCalendar::from_interaction(input)?).await,
        _ => answer.reply_privately(Reply::text("Command not found. Please report this.")).await,
    }
}

impl Bot {
    pub async fn command_mention(&self, guild_id: Id<GuildMarker>, command: &str) -> String {
        let name = command.split(' ').next().unwrap_or(command);
        let interaction = self.http.interaction(self.application_id);
        let mut published = Vec::new();
        if let Ok(response) = interaction.guild_commands(guild_id).await {
            published.extend(response.models().await.unwrap_or_default());
        }
        if let Ok(response) = interaction.global_commands().await {
            published.extend(response.models().await.unwrap_or_default());
        }
        match published.into_iter().find(|published| published.name == name).and_then(|published| published.id) {
            Some(id) => format!("</{command}:{id}>"),
            None => format!("/{command}"),
        }
    }
}

async fn open_config(bot: &Bot, answer: &Answer<'_>) -> anyhow::Result<()> {
    let interaction = answer.interaction;
    let permissions = interaction.member.as_ref().and_then(|member| member.permissions);
    let (true, Some(guild_id), Some(user_id)) = (may_configure_guild(permissions), interaction.guild_id, interaction.author_id())
    else {
        return answer
            .reply_privately(Reply::text("Only people who can manage this server can change its settings."))
            .await;
    };
    let token = bot.config_grants.grant(ConfigGrant { guild_id, user_id });
    let settings = link_button("Settings", bot.links.config_page_url(&token));
    let reply = Reply {
        content: "-# The link works for an hour, and only you have it.".to_owned(),
        components: vec![row(vec![settings])],
    };
    answer.reply_privately(reply).await
}

async fn open_calendar(bot: &Bot, answer: &Answer<'_>, options: MyCalendar) -> anyhow::Result<()> {
    let Some(user_id) = answer.interaction.author_id() else {
        return Ok(());
    };
    let link = bot.calendar_link_of(user_id, options.replace.unwrap_or(false))?;
    let index_url = bot.links.calendar_index_url(&link.token);
    let open = link_button("Open", bot.links.index_view_url(&index_url));
    answer.reply_privately(Reply { content: String::new(), components: vec![row(vec![open])] }).await
}

async fn send_new_event_button(bot: &Bot, answer: &Answer<'_>, options: SendNewEventButton) -> anyhow::Result<()> {
    answer.defer_privately().await?;
    let message = new_event_button_outcome(bot, answer, options).await?;
    answer.edit_reply(Reply::text(message)).await
}

async fn new_event_button_outcome(bot: &Bot, answer: &Answer<'_>, options: SendNewEventButton) -> anyhow::Result<String> {
    let forum_id = options.forum;
    let interaction = answer.interaction;
    if !interaction_has_permission(interaction, Permissions::MANAGE_THREADS) {
        return Ok("Only staff can post this button.".to_owned());
    }
    if let Some(forum_id) = forum_id
        && bot.db.event_forum_of(forum_id)?.is_none()
    {
        return Ok(format!("<#{forum_id}> is not an event forum."));
    }
    let Some(channel_id) = interaction.channel.as_ref().map(|channel| channel.id) else {
        return Ok("Run this in a channel.".to_owned());
    };
    let label = options.text.as_deref().unwrap_or("New event");
    let emoji = options.emoji.as_deref().map(emoji_of_text);
    let style = options.color.as_ref().map_or(ButtonStyle::Primary, ButtonColor::style);
    let buttons = [NewEventButton { forum_id }.component(label, emoji, style)];
    let Err(error) = bot.http.create_message(channel_id).components(&buttons).await else {
        return Ok("Posted.".to_owned());
    };
    let error = anyhow::Error::from(error);
    tracing::warn!(channel = %channel_id, ?error, "New event button was not posted");
    if options.emoji.is_some() && is_discord_error(&error, INVALID_FORM_BODY) {
        return Ok("Discord did not take that emoji. Use a standard one or one from this server.".to_owned());
    }
    Ok("I could not post in this channel.".to_owned())
}

async fn register_post(bot: &Bot, answer: &Answer<'_>) -> anyhow::Result<()> {
    answer.defer_privately().await?;
    let message = register_post_outcome(bot, answer).await?;
    answer.edit_reply(Reply::text(message)).await
}

async fn register_post_outcome(bot: &Bot, answer: &Answer<'_>) -> anyhow::Result<&'static str> {
    let interaction = answer.interaction;
    let channel_id = interaction.channel.as_ref().map(|channel| channel.id);
    let thread = match channel_id {
        Some(channel_id) => bot.channel(channel_id).await.filter(|channel| channel.is_thread()),
        None => None,
    };
    let Some(thread) = thread else {
        return Ok("Run this inside the post.");
    };
    let is_staff = interaction_has_permission(interaction, Permissions::MANAGE_THREADS);
    let forum = match thread.parent_id {
        Some(parent_id) => bot.db.event_forum_of(parent_id)?,
        None => None,
    };
    if forum.is_some_and(|forum| forum.is_screened()) && !is_staff {
        return Ok("Moderators check every event in this forum, so only staff can do this.");
    }
    if thread.owner_id != interaction.author_id() && !is_staff {
        return Ok("Only the person who posted it or staff can do this.");
    }
    if bot.db.event_of_thread(thread.id)?.is_some() {
        return Ok("This post is an event already.");
    }
    Ok(match start_event(bot, &thread, None).await? {
        EventStart::Started => "This post is an event now. Everyone who already reacted got the role.",
        EventStart::NoAttendReaction => {
            "This forum has no default reaction to attend with. Staff can set one in the forum's settings."
        }
        EventStart::NotAnEventForum => "This post is not in an event forum.",
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::all_commands;

    #[test]
    fn published_command_shapes() {
        let commands: Vec<Value> = all_commands().iter().map(|command| serde_json::to_value(command).unwrap()).collect();
        let [config, event, calendar] = commands.as_slice() else {
            panic!("three commands");
        };
        assert_eq!(config["name"], "config");
        assert_eq!(config["default_member_permissions"], "32");
        assert_eq!(config["contexts"], json!([0]));
        assert_eq!(event["contexts"], json!([0]));
        let forum = &event["options"][0]["options"][0];
        assert_eq!(event["options"][0]["name"], "new");
        assert_eq!(forum["name"], "forum");
        assert_eq!(forum["channel_types"], json!([15]));
        assert_ne!(forum["required"], json!(true));
        assert_eq!(event["options"][1]["name"], "register");
        assert_eq!(event["options"][2]["name"], "send-new-button");
        assert_eq!(event["options"][2]["options"][0]["channel_types"], json!([15]));
        assert_eq!(event["options"][2]["options"][1]["max_length"], 80);
        assert_eq!(event["options"][2]["options"][3]["choices"][0]["value"], "blurple");
        assert_eq!(calendar["contexts"], json!([0, 1]));
        assert_eq!(calendar["options"][0]["name"], "new");
        assert_eq!(calendar["options"][0]["type"], 5);
    }
}
