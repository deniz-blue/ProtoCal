use twilight_model::{
    application::interaction::modal::{ModalInteractionComponent, ModalInteractionData},
    channel::message::{
        Component,
        component::{Label, TextInput, TextInputStyle},
    },
    http::interaction::InteractionResponseData,
};

pub struct TextField<'a> {
    pub custom_id: &'a str,
    pub label: &'a str,
    pub max_length: u16,
    pub required: bool,
    pub paragraph: bool,
    pub placeholder: Option<&'a str>,
    pub value: Option<String>,
}

#[allow(deprecated)]
fn labelled_input(field: TextField) -> Component {
    Component::Label(Label {
        id: None,
        label: field.label.to_owned(),
        description: None,
        component: Box::new(Component::TextInput(TextInput {
            id: None,
            custom_id: field.custom_id.to_owned(),
            label: None,
            max_length: Some(field.max_length),
            min_length: None,
            placeholder: field.placeholder.map(str::to_owned),
            required: Some(field.required),
            style: if field.paragraph { TextInputStyle::Paragraph } else { TextInputStyle::Short },
            value: field.value.filter(|value| !value.is_empty()),
        })),
    })
}

pub fn modal(custom_id: String, title: &str, fields: Vec<TextField>) -> InteractionResponseData {
    InteractionResponseData {
        custom_id: Some(custom_id),
        title: Some(title.to_owned()),
        components: Some(fields.into_iter().map(labelled_input).collect()),
        ..Default::default()
    }
}

fn submitted_text<'a>(components: &'a [ModalInteractionComponent], custom_id: &str) -> Option<&'a str> {
    components.iter().find_map(|component| match component {
        ModalInteractionComponent::Label(label) => {
            submitted_text(std::slice::from_ref(&*label.component), custom_id)
        }
        ModalInteractionComponent::ActionRow(row) => submitted_text(&row.components, custom_id),
        ModalInteractionComponent::TextInput(input) if input.custom_id == custom_id => {
            Some(input.value.as_str())
        }
        _ => None,
    })
}

pub fn submitted_value(data: &ModalInteractionData, custom_id: &str) -> Option<String> {
    submitted_text(&data.components, custom_id)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}
