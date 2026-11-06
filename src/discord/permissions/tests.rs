use twilight_model::{channel::permission_overwrite::PermissionOverwriteType, guild::Permissions, id::Id};

use super::{MemberRoles, everyone_permissions_in, guild_permissions, member_permissions_in};
use crate::discord::fixtures::{MEMBER_ROLE, OWNER, STAFF_ROLE, forum, guild, hidden_forum, overwrite};

fn member(user: u64, roles: &[u64]) -> MemberRoles {
    MemberRoles { user_id: Id::new(user), role_ids: roles.iter().copied().map(Id::new).collect() }
}

#[test]
fn the_owner_has_everything_and_roles_add_up() {
    let guild = guild();
    assert_eq!(guild_permissions(&guild, &member(OWNER, &[])), Permissions::all());
    let staff = guild_permissions(&guild, &member(7, &[STAFF_ROLE]));
    assert!(staff.contains(Permissions::MANAGE_THREADS | Permissions::VIEW_CHANNEL));
    assert!(!staff.contains(Permissions::MANAGE_GUILD));
}

#[test]
fn a_hidden_forum_is_hidden_from_everyone_but_open_to_its_role() {
    let guild = guild();
    let forum = hidden_forum(20, "adult-meets");
    assert!(!everyone_permissions_in(&guild, &forum).contains(Permissions::VIEW_CHANNEL));
    assert!(!member_permissions_in(&guild, &forum, &member(7, &[])).contains(Permissions::VIEW_CHANNEL));
    assert!(member_permissions_in(&guild, &forum, &member(7, &[MEMBER_ROLE])).contains(Permissions::VIEW_CHANNEL));
}

#[test]
fn a_member_overwrite_wins_over_role_overwrites() {
    let guild = guild();
    let denied = overwrite(7, PermissionOverwriteType::Member, Permissions::empty(), Permissions::SEND_MESSAGES);
    let forum = forum(21, "meets", vec![denied]);
    assert!(!member_permissions_in(&guild, &forum, &member(7, &[])).contains(Permissions::SEND_MESSAGES));
    assert!(member_permissions_in(&guild, &forum, &member(8, &[])).contains(Permissions::SEND_MESSAGES));
    assert!(everyone_permissions_in(&guild, &forum).contains(Permissions::SEND_MESSAGES));
}
