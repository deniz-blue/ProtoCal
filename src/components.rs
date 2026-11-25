use twilight_model::channel::message::{
    Component, EmojiReactionType, MessageFlags,
    component::{ActionRow, Button, ButtonStyle, Container, Section, TextDisplay, Thumbnail, UnfurledMediaItem},
};

// Discord limits for a Components V2 message: 40 components, 4000 characters across its text displays.
pub const COMPONENTS_MAX: usize = 40;
pub const TEXT_DISPLAYS_MAX: usize = 4000;

pub fn emoji_of_text(text: &str) -> EmojiReactionType {
    let text = text.trim();
    let custom = text.strip_prefix('<').and_then(|rest| rest.strip_suffix('>')).and_then(|inner| {
        let (animated, inner) = match inner.strip_prefix("a:") {
            Some(rest) => (true, rest),
            None => (false, inner.strip_prefix(':')?),
        };
        let (name, id) = inner.rsplit_once(':')?;
        Some(EmojiReactionType::Custom { animated, id: id.parse().ok()?, name: Some(name.to_owned()) })
    });
    custom.unwrap_or_else(|| EmojiReactionType::Unicode { name: text.to_owned() })
}

pub fn button(custom_id: impl Into<String>, label: impl Into<String>, style: ButtonStyle) -> Component {
    Component::Button(Button {
        id: None,
        custom_id: Some(custom_id.into()),
        disabled: false,
        emoji: None,
        label: Some(label.into()),
        style,
        url: None,
        sku_id: None,
    })
}

pub fn link_button(label: impl Into<String>, url: impl Into<String>) -> Component {
    Component::Button(Button {
        id: None,
        custom_id: None,
        disabled: false,
        emoji: None,
        label: Some(label.into()),
        style: ButtonStyle::Link,
        url: Some(url.into()),
        sku_id: None,
    })
}

pub const fn row(components: Vec<Component>) -> Component {
    Component::ActionRow(ActionRow {
        id: None,
        components,
    })
}

// twilight refuses to create a message with a text display over 4000 bytes.
pub const TEXT_DISPLAY_BYTES_MAX: usize = 4000;

pub fn text_display(content: impl Into<String>) -> Component {
    let content = content.into();
    let content = if content.len() <= TEXT_DISPLAY_BYTES_MAX {
        content
    } else {
        format!("{}…", &content[..content.floor_char_boundary(TEXT_DISPLAY_BYTES_MAX - '…'.len_utf8())])
    };
    Component::TextDisplay(TextDisplay { id: None, content })
}

pub fn section_with_thumbnail(text: impl Into<String>, thumbnail_url: impl Into<String>) -> Component {
    let media = UnfurledMediaItem { url: thumbnail_url.into(), proxy_url: None, height: None, width: None, content_type: None };
    Component::Section(Section {
        id: None,
        components: vec![text_display(text)],
        accessory: Box::new(Component::Thumbnail(Thumbnail { id: None, media, description: None, spoiler: None })),
    })
}

pub const fn container(components: Vec<Component>) -> Component {
    Component::Container(Container { id: None, accent_color: None, spoiler: None, components })
}

pub fn message_flags_of(components: &[Component]) -> MessageFlags {
    if components.iter().all(|component| matches!(component, Component::ActionRow(_))) {
        MessageFlags::empty()
    } else {
        MessageFlags::IS_COMPONENTS_V2
    }
}

pub fn component_count(components: &[Component]) -> usize {
    components
        .iter()
        .map(|component| {
            1 + match component {
                Component::ActionRow(row) => component_count(&row.components),
                Component::Container(container) => component_count(&container.components),
                Component::Section(section) => component_count(&section.components) + component_count(std::slice::from_ref(&*section.accessory)),
                _ => 0,
            }
        })
        .sum()
}

pub fn text_displays(components: &[Component]) -> Vec<&str> {
    components
        .iter()
        .flat_map(|component| match component {
            Component::Container(container) => text_displays(&container.components),
            Component::Section(section) => text_displays(&section.components),
            Component::TextDisplay(text) => vec![text.content.as_str()],
            _ => vec![],
        })
        .collect()
}

pub fn thumbnail_url_of(components: &[Component]) -> Option<&str> {
    components.iter().find_map(|component| match component {
        Component::Container(container) => thumbnail_url_of(&container.components),
        Component::Section(section) => thumbnail_url_of(std::slice::from_ref(&*section.accessory)),
        Component::Thumbnail(thumbnail) => Some(thumbnail.media.url.as_str()),
        _ => None,
    })
}

#[cfg(test)]
pub fn button_labels(components: &[Component]) -> Vec<&str> {
    components
        .iter()
        .flat_map(|component| match component {
            Component::ActionRow(row) => button_labels(&row.components),
            Component::Button(button) => button.label.as_deref().into_iter().collect(),
            _ => vec![],
        })
        .collect()
}
