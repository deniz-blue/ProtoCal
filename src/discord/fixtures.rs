use std::collections::HashMap;

use twilight_model::{
    channel::{
        ChannelType,
        permission_overwrite::{PermissionOverwrite, PermissionOverwriteType},
    },
    guild::Permissions,
    id::{Id, marker::RoleMarker},
};

use super::cache::{CachedChannel, CachedGuild, CachedRole};
use crate::meet::attend_reaction::ReactionEmoji;

pub const GUILD: u64 = 1;
pub const OWNER: u64 = 100;
pub const STAFF_ROLE: u64 = 50;
pub const MEMBER_ROLE: u64 = 51;

fn role(id: u64, name: &str, permissions: Permissions, position: i64) -> (Id<RoleMarker>, CachedRole) {
    let id = Id::new(id);
    (id, CachedRole { id, name: name.to_owned(), permissions, position, managed: false })
}

pub fn guild() -> CachedGuild {
    CachedGuild {
        id: Id::new(GUILD),
        name: "haj house".to_owned(),
        owner_id: Id::new(OWNER),
        roles: HashMap::from([
            role(GUILD, "@everyone", Permissions::VIEW_CHANNEL | Permissions::SEND_MESSAGES, 0),
            role(STAFF_ROLE, "Staff", Permissions::MANAGE_THREADS, 2),
            role(MEMBER_ROLE, "Member", Permissions::empty(), 1),
        ]),
    }
}

pub fn overwrite(id: u64, kind: PermissionOverwriteType, allow: Permissions, deny: Permissions) -> PermissionOverwrite {
    PermissionOverwrite { allow, deny, id: Id::new(id), kind }
}

pub fn forum(id: u64, name: &str, overwrites: Vec<PermissionOverwrite>) -> CachedChannel {
    CachedChannel {
        id: Id::new(id),
        guild_id: Id::new(GUILD),
        kind: ChannelType::GuildForum,
        name: name.to_owned(),
        position: 0,
        parent_id: None,
        owner_id: None,
        locked: false,
        overwrites,
        default_reaction: Some(ReactionEmoji { id: None, name: Some("👋".to_owned()) }),
    }
}

pub fn hidden_forum(id: u64, name: &str) -> CachedChannel {
    let hidden = overwrite(GUILD, PermissionOverwriteType::Role, Permissions::empty(), Permissions::VIEW_CHANNEL);
    let members = overwrite(MEMBER_ROLE, PermissionOverwriteType::Role, Permissions::VIEW_CHANNEL, Permissions::empty());
    forum(id, name, vec![hidden, members])
}
