use twilight_model::channel::message::{Component, MessageFlags};

use super::{
    organizer_picture::thumbnail_shows,
    post::{EventPostText, has_link_button},
};
use crate::{
    components::{TEXT_DISPLAYS_MAX, section_with_thumbnail, text_display, text_displays, thumbnail_url_of},
    text::utf16_length,
};

pub fn event_post_components(text: &EventPostText, thumbnail_url: Option<&str>) -> Vec<Component> {
    let room = TEXT_DISPLAYS_MAX.saturating_sub(utf16_length(&text.head) + utf16_length(&text.foot));
    let head = match thumbnail_url {
        Some(url) => section_with_thumbnail(text.head.clone(), url),
        None => text_display(text.head.clone()),
    };
    [Some(head), text.description_within(room).map(text_display), Some(text_display(text.foot.clone()))]
        .into_iter()
        .flatten()
        .collect()
}

pub fn event_post_is_outdated(posted_flags: Option<MessageFlags>, posted: &[Component], wanted: &[Component], view_url: &str) -> bool {
    let is_v2 = posted_flags.is_some_and(|flags| flags.contains(MessageFlags::IS_COMPONENTS_V2));
    let same_thumbnail = match (thumbnail_url_of(posted), thumbnail_url_of(wanted)) {
        (Some(posted), Some(wanted)) => thumbnail_shows(posted, wanted),
        (posted, wanted) => posted.is_none() && wanted.is_none(),
    };
    !is_v2 || !same_thumbnail || text_displays(posted) != text_displays(wanted) || !has_link_button(posted, view_url)
}
