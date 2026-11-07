use twilight_model::{
    application::interaction::modal::ModalInteractionData, channel::message::component::ButtonStyle,
    http::interaction::InteractionResponseData,
};

use super::Bot;
use crate::{
    components::{button, row},
    discord::{
        answer::{Answer, Reply},
        modal::{TextField, modal, submitted_value},
    },
    event::{
        fields::EventFields,
        post::{EventFormTarget, THREAD_NAME_MAX},
    },
    text::cut_to_utf16_length,
};

pub fn event_form_modal(target: &EventFormTarget, filled: Option<EventFields>) -> InteractionResponseData {
    let filled = filled.unwrap_or(EventFields {
        name: String::new(),
        start: None,
        end: None,
        place: None,
        description: None,
    });
    let field = |custom_id, label, max_length: usize, value: Option<String>| TextField {
        custom_id,
        label,
        max_length: max_length as u16,
        required: false,
        paragraph: false,
        placeholder: None,
        value: value.map(|value| cut_to_utf16_length(&value, max_length).to_owned()),
    };
    let title = if matches!(target, EventFormTarget::New(_)) { "New event" } else { "Edit event" };
    modal(
        target.custom_id(),
        title,
        vec![
            TextField { required: true, ..field("name", "Name", THREAD_NAME_MAX, Some(filled.name)) },
            TextField {
                placeholder: Some("2026-12-05 18:00, or 2026-12-05"),
                ..field("start", "Start", 40, filled.start)
            },
            TextField {
                placeholder: Some("2026-12-06 02:00, or 2026-12-07"),
                ..field("end", "End", 40, filled.end)
            },
            field("place", "Place", 200, filled.place),
            TextField { paragraph: true, ..field("description", "Description", 1500, filled.description) },
        ],
    )
}

pub fn fields_of_submitted_event_form(data: &ModalInteractionData) -> EventFields {
    EventFields {
        name: submitted_value(data, "name").unwrap_or_default(),
        start: submitted_value(data, "start"),
        end: submitted_value(data, "end"),
        place: submitted_value(data, "place"),
        description: submitted_value(data, "description"),
    }
}

pub async fn reject_draft(
    bot: &Bot,
    answer: &Answer<'_>,
    target: &EventFormTarget,
    fields: EventFields,
    message: &str,
) -> anyhow::Result<()> {
    if let Some(user_id) = answer.interaction.author_id() {
        bot.reject_draft_of(user_id, &target.key(), fields);
    }
    let fix = button(target.custom_id(), "Fix it", ButtonStyle::Primary);
    answer
        .reply_or_edit(Reply { content: message.to_owned(), components: vec![row(vec![fix])] })
        .await
}
