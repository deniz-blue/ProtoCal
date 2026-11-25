use serde_json::json;
use twilight_model::{
    channel::message::{Component, MessageFlags},
    id::Id,
};

use super::{
    document::DESCRIPTION_TYPE,
    post::{event_post_buttons, event_post_text},
    post_message::{event_post_components, event_post_is_outdated},
};
use crate::{
    components::{TEXT_DISPLAY_BYTES_MAX, TEXT_DISPLAYS_MAX, row, text_display, text_displays, thumbnail_url_of},
    config::test_links,
    text::utf16_length,
};

const AVATAR: &str = "https://cdn.discordapp.com/embed/avatars/1.png";

fn components_of(description: &str) -> Vec<Component> {
    let text = json!({ "$type": DESCRIPTION_TYPE, "content": { "und": description } });
    let document = json!({ "name": { "und": "Kaunas Bar Meet" }, "components": [text] });
    event_post_components(&event_post_text(&document, None, &[Id::new(7)], None, 0), Some(AVATAR))
}

#[test]
fn a_post_is_the_head_beside_the_picture_then_the_description_then_the_foot() {
    let with = components_of("Drinks.");
    assert!(matches!(&with[0], Component::Section(section) if section.components.len() == 1));
    assert_eq!(thumbnail_url_of(&with), Some(AVATAR));
    assert_eq!(
        text_displays(&with),
        ["## Kaunas Bar Meet\n\n🪧 **Organized by** <@7>", "Drinks.", "-# Last updated at <t:0:R>"]
    );
    let without = components_of("");
    assert_eq!(without.len(), 2);
    assert_eq!(text_displays(&without), ["## Kaunas Bar Meet\n\n🪧 **Organized by** <@7>", "-# Last updated at <t:0:R>"]);
}

#[test]
fn the_description_is_cut_so_all_text_fits_in_4000_characters_and_each_text_in_4000_bytes() {
    let components = components_of(&"a".repeat(5000));
    let texts = text_displays(&components);
    assert_eq!(texts.iter().map(|text| utf16_length(text)).sum::<usize>(), TEXT_DISPLAYS_MAX);
    assert!(texts[1].ends_with("a…"));

    let components = components_of(&"ž".repeat(5000));
    let texts = text_displays(&components);
    assert!(texts.iter().all(|text| text.len() <= TEXT_DISPLAY_BYTES_MAX));
    assert!(texts[1].ends_with("ž…"));
}

#[test]
fn a_text_post_is_outdated_and_the_same_post_is_not() {
    let links = test_links();
    let buttons = event_post_buttons(&links, Id::new(10), "abc");
    let view_url = links.event_view_url("abc");
    let wanted = components_of("Drinks.");
    let posted = [wanted.clone(), vec![buttons.clone()]].concat();
    let v2 = Some(MessageFlags::IS_COMPONENTS_V2);
    assert!(!event_post_is_outdated(v2, &posted, &wanted, &view_url));
    assert!(event_post_is_outdated(None, std::slice::from_ref(&buttons), &wanted, &view_url));
    assert!(event_post_is_outdated(None, &posted, &wanted, &view_url));
    assert!(event_post_is_outdated(v2, &posted, &components_of("Snacks."), &view_url));
    assert!(event_post_is_outdated(v2, &posted, &wanted, &links.event_view_url("old")));
    let other_picture = [vec![text_display("x")], vec![row(vec![])]].concat();
    assert!(event_post_is_outdated(v2, &other_picture, &wanted, &view_url));
}
