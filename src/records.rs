use serde_json::Value;
use twilight_model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, MessageMarker, RoleMarker, UserMarker},
};

#[derive(Clone, Debug, PartialEq)]
pub struct MeetForum {
    pub forum_id: Id<ChannelMarker>,
    pub guild_id: Id<GuildMarker>,
    pub announce_role_id: Option<Id<RoleMarker>>,
    pub review_channel_id: Option<Id<ChannelMarker>>,
}

impl MeetForum {
    pub fn is_screened(&self) -> bool {
        self.review_channel_id.is_some()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Meet {
    pub thread_id: Id<ChannelMarker>,
    pub guild_id: Id<GuildMarker>,
    pub forum_id: Id<ChannelMarker>,
    pub role_id: Id<RoleMarker>,
    pub attend_reaction: String,
    pub organizers: Vec<Id<UserMarker>>,
    pub document: Value,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalendarLink {
    pub user_id: Id<UserMarker>,
    pub token: String,
    pub created_at: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestState {
    Waiting,
    Declined,
}

impl RequestState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Declined => "declined",
        }
    }

    pub fn of_str(text: &str) -> Option<Self> {
        match text {
            "waiting" => Some(Self::Waiting),
            "declined" => Some(Self::Declined),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MessageAt {
    pub channel_id: Id<ChannelMarker>,
    pub message_id: Id<MessageMarker>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeetRequest {
    pub id: String,
    pub guild_id: Id<GuildMarker>,
    pub forum_id: Id<ChannelMarker>,
    pub host_id: Id<UserMarker>,
    pub document: Value,
    pub state: RequestState,
    pub handled_by: Option<Id<UserMarker>>,
    pub decline_reason: Option<String>,
    pub host_edited: bool,
    pub moderators_edited: bool,
    pub card_channel_id: Id<ChannelMarker>,
    pub card_message_id: Option<Id<MessageMarker>>,
    pub host_dm: Option<MessageAt>,
    pub updated_at: i64,
}
