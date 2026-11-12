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
    let filled = filled.unwrap_or_default();
    let field = |custom_id, label, max_length: usize, value: Option<String>| TextField {
        custom_id,
        label,
        max_length: max_length as u16,
        required: false,
        paragraph: false,
        placeholder: None,
        value: value.map(|value| cut_to_utf16_length(&value, max_length).to_owned()),
    };
    let creating = matches!(target, EventFormTarget::New(_));
    let mut fields = vec![
        TextField { required: true, ..field("name", "Name", THREAD_NAME_MAX, Some(filled.name)) },
        TextField { placeholder: Some("12 October 18:00 - 22:00"), ..field("date", "Date", 100, filled.date) },
        field("place", "Place", 200, filled.place),
        TextField { paragraph: true, ..field("description", "Description", 1500, filled.description) },
    ];
    if !creating {
        fields.push(TextField {
            placeholder: Some("Link to an Open Evnt event, like a convention"),
            ..field("part_of", "Part of", 500, filled.part_of)
        });
    }
    modal(target.custom_id(), if creating { "New event" } else { "Edit event" }, fields)
}

pub fn fields_of_submitted_event_form(data: &ModalInteractionData) -> EventFields {
    EventFields {
        name: submitted_value(data, "name").unwrap_or_default(),
        date: submitted_value(data, "date"),
        place: submitted_value(data, "place"),
        description: submitted_value(data, "description"),
        part_of: submitted_value(data, "part_of"),
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
