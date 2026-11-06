use twilight_model::{
    channel::permission_overwrite::PermissionOverwriteType,
    guild::Permissions,
    id::{
        Id,
        marker::{RoleMarker, UserMarker},
    },
};

use super::cache::{CachedChannel, CachedGuild};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberRoles {
    pub user_id: Id<UserMarker>,
    pub role_ids: Vec<Id<RoleMarker>>,
}

fn role_permissions(guild: &CachedGuild, role_id: Id<RoleMarker>) -> Permissions {
    guild
        .roles
        .get(&role_id)
        .map_or(Permissions::empty(), |role| role.permissions)
}

fn everyone_role(guild: &CachedGuild) -> Id<RoleMarker> {
    guild.id.cast()
}

pub fn guild_permissions(guild: &CachedGuild, member: &MemberRoles) -> Permissions {
    if member.user_id == guild.owner_id {
        return Permissions::all();
    }
    let permissions = member
        .role_ids
        .iter()
        .fold(role_permissions(guild, everyone_role(guild)), |all, role_id| {
            all | role_permissions(guild, *role_id)
        });
    if permissions.contains(Permissions::ADMINISTRATOR) {
        Permissions::all()
    } else {
        permissions
    }
}

fn overwritten(base: Permissions, allow: Permissions, deny: Permissions) -> Permissions {
    (base - deny) | allow
}

fn overwrite_of(channel: &CachedChannel, id: u64, kind: PermissionOverwriteType) -> (Permissions, Permissions) {
    channel
        .overwrites
        .iter()
        .find(|overwrite| overwrite.id.get() == id && overwrite.kind == kind)
        .map_or((Permissions::empty(), Permissions::empty()), |overwrite| (overwrite.allow, overwrite.deny))
}

pub fn member_permissions_in(guild: &CachedGuild, channel: &CachedChannel, member: &MemberRoles) -> Permissions {
    let base = guild_permissions(guild, member);
    if base == Permissions::all() {
        return base;
    }
    let (allow, deny) = overwrite_of(channel, guild.id.get(), PermissionOverwriteType::Role);
    let base = overwritten(base, allow, deny);
    let (role_allow, role_deny) = member.role_ids.iter().fold(
        (Permissions::empty(), Permissions::empty()),
        |(all_allow, all_deny), role_id| {
            let (allow, deny) = overwrite_of(channel, role_id.get(), PermissionOverwriteType::Role);
            (all_allow | allow, all_deny | deny)
        },
    );
    let base = overwritten(base, role_allow, role_deny);
    let (allow, deny) = overwrite_of(channel, member.user_id.get(), PermissionOverwriteType::Member);
    overwritten(base, allow, deny)
}

pub fn everyone_permissions_in(guild: &CachedGuild, channel: &CachedChannel) -> Permissions {
    let base = role_permissions(guild, everyone_role(guild));
    if base.contains(Permissions::ADMINISTRATOR) {
        return Permissions::all();
    }
    let (allow, deny) = overwrite_of(channel, guild.id.get(), PermissionOverwriteType::Role);
    overwritten(base, allow, deny)
}

#[cfg(test)]
mod tests;
