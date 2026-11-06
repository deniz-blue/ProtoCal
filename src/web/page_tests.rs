use twilight_model::{channel::ChannelType, guild::Permissions, id::Id};

use super::{
    config_page::{ConfigPageOptions, config_page},
    edit_return_page::edit_return_page,
};
use crate::{
    bot::guild_config::ForumConfig,
    config::test_links,
    discord::{
        cache::{CachedChannel, CachedRole},
        fixtures::forum,
    },
};

fn forum_config(forum_id: u64, name: Option<&str>) -> ForumConfig {
    ForumConfig {
        forum_id: Id::new(forum_id),
        name: name.map(str::to_owned),
        is_meet_forum: true,
        announce_role_id: None,
        review_channel_id: None,
        has_default_reaction: true,
        everyone_can_view: true,
        everyone_can_post: false,
    }
}

fn role(id: u64, name: &str) -> CachedRole {
    CachedRole { id: Id::new(id), name: name.to_owned(), permissions: Permissions::empty(), position: 0, managed: false }
}

fn text_channel(id: u64, name: &str) -> CachedChannel {
    CachedChannel { kind: ChannelType::GuildText, ..forum(id, name, vec![]) }
}

#[test]
fn config_page_matches_the_typescript_page_byte_for_byte() {
    let forums = [
        ForumConfig {
            announce_role_id: Some("51".to_owned()),
            review_channel_id: Some("31".to_owned()),
            everyone_can_post: true,
            ..forum_config(20, Some("meets"))
        },
        ForumConfig {
            is_meet_forum: false,
            everyone_can_view: false,
            has_default_reaction: false,
            ..forum_config(21, Some("hidden <one>"))
        },
        ForumConfig { has_default_reaction: false, ..forum_config(22, Some("no reaction")) },
        ForumConfig {
            everyone_can_view: false,
            has_default_reaction: false,
            review_channel_id: Some("30".to_owned()),
            ..forum_config(23, None)
        },
    ];
    let roles = [role(51, "Pings & <news>"), role(52, "Member")];
    let review_channels = [text_channel(30, "review"), text_channel(31, "mod's \"desk\"")];
    let options = ConfigPageOptions {
        guild_name: "haj & <house>",
        forums: &forums,
        roles: &roles,
        review_channels: &review_channels,
        saved: false,
    };
    assert_eq!(config_page(&test_links(), &options), include_str!("reference/config_page_forums.html"));
}

#[test]
fn empty_saved_config_page_matches_the_typescript_page() {
    let options = ConfigPageOptions { guild_name: "empty", forums: &[], roles: &[], review_channels: &[], saved: true };
    assert_eq!(config_page(&test_links(), &options), include_str!("reference/config_page_empty.html"));
}

#[test]
fn edit_return_page_matches_the_typescript_page() {
    assert_eq!(edit_return_page(), include_str!("reference/edit_return_page.html"));
}
