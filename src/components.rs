use twilight_model::channel::message::{
    Component,
    component::{ActionRow, Button, ButtonStyle},
};

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

pub fn row(components: Vec<Component>) -> Component {
    Component::ActionRow(ActionRow {
        id: None,
        components,
    })
}

#[cfg(test)]
pub fn button_labels(components: &[Component]) -> Vec<String> {
    components
        .iter()
        .flat_map(|component| match component {
            Component::ActionRow(row) => button_labels(&row.components),
            Component::Button(button) => button.label.clone().into_iter().collect(),
            _ => Vec::new(),
        })
        .collect()
}
