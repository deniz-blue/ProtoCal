use twilight_model::channel::message::{
    Component, EmojiReactionType, MessageFlags,
    component::{ActionRow, Button, ButtonStyle, Container, TextDisplay},
};

pub const COMPONENTS_MAX: usize = 40;

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

pub fn text_display(content: impl Into<String>) -> Component {
    Component::TextDisplay(TextDisplay { id: None, content: content.into() })
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
            Component::TextDisplay(text) => vec![text.content.as_str()],
            _ => vec![],
        })
        .collect()
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
