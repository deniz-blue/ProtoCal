use serde_json::{Map, Value, json};

use super::document::{
    DESCRIPTION_TYPE, EventField, EventFieldProblem, event_description_of,
    event_name_of, event_place_of, is_description, validate_event_document,
};
use super::written_date::{partial_date_of_written, written_of_partial_date};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventFields {
    pub name: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub place: Option<String>,
    pub description: Option<String>,
}

pub fn fields_of_event_document(document: &Value) -> EventFields {
    let first = document.get("occurrences").and_then(|all| all.get(0));
    let written = |name: &str| {
        first
            .and_then(|occurrence| occurrence.get(name))
            .and_then(Value::as_str)
            .map(written_of_partial_date)
    };
    EventFields {
        name: event_name_of(document),
        start: written("start"),
        end: written("end"),
        place: event_place_of(document),
        description: event_description_of(document),
    }
}

fn set_or_remove(fields: &mut Map<String, Value>, name: &str, value: Option<Value>) {
    match value {
        Some(value) => {
            fields.insert(name.to_owned(), value);
        }
        None => {
            fields.shift_remove(name);
        }
    }
}

fn without_null(mut fields: Map<String, Value>) -> Option<Value> {
    fields.retain(|_, value| !value.is_null());
    (!fields.is_empty()).then_some(Value::Object(fields))
}

fn occurrences_with(occurrences: Option<&Value>, start: Option<String>, end: Option<String>) -> Option<Value> {
    let all = occurrences.and_then(Value::as_array).cloned().unwrap_or_default();
    let first = all.first().and_then(Value::as_object);
    let first_date = |name: &str| first.and_then(|first| first.get(name)).and_then(Value::as_str);
    if start.as_deref() == first_date("start") && end.as_deref() == first_date("end") {
        return occurrences.cloned();
    }
    let mut changed = first.cloned().unwrap_or_default();
    changed.insert("start".to_owned(), start.map_or(Value::Null, Value::String));
    changed.insert("end".to_owned(), end.map_or(Value::Null, Value::String));
    let rest = all.into_iter().skip(1);
    let changed: Vec<Value> = without_null(changed).into_iter().chain(rest).collect();
    (!changed.is_empty()).then_some(Value::Array(changed))
}

fn venue_with(venue: Option<&Value>, shown_place: Option<&str>, place: Option<&str>) -> Option<Value> {
    if place == shown_place {
        return venue.cloned();
    }
    let mut rest = venue.and_then(Value::as_object).cloned().unwrap_or_default();
    let address = rest.shift_remove("address");
    let name = rest.shift_remove("name");
    if address.is_some()
        && let Some(name) = name
    {
        rest.insert("name".to_owned(), name);
    }
    if let Some(place) = place {
        rest.insert("address".to_owned(), json!(place));
    }
    without_null(rest)
}

fn components_with(
    components: Option<&Value>,
    shown_description: Option<&str>,
    description: Option<&str>,
) -> Option<Value> {
    if description == shown_description {
        return components.cloned();
    }
    let others = components
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|component| !is_description(component))
        .cloned();
    let written = description.map(|text| json!({ "$type": DESCRIPTION_TYPE, "content": { "und": text } }));
    let changed: Vec<Value> = written.into_iter().chain(others).collect();
    (!changed.is_empty()).then_some(Value::Array(changed))
}

fn unreadable_date(field: EventField, written: &str) -> EventFieldProblem {
    EventFieldProblem {
        field: Some(field),
        message: format!(
            "{}: `{written}` is not a date I can read. Write it as `2026-12-05 18:00`, `2026-12-05` for a whole day, or `2026-12` for a month.",
            field.label()
        ),
    }
}

fn read_date(field: EventField, written: Option<&str>) -> Result<Option<String>, EventFieldProblem> {
    match written {
        None => Ok(None),
        Some(written) => partial_date_of_written(written)
            .map(Some)
            .ok_or_else(|| unreadable_date(field, written)),
    }
}

pub fn event_document_of_fields(fields: &EventFields, base: &Value) -> Result<Value, EventFieldProblem> {
    let start = read_date(EventField::Start, fields.start.as_deref())?;
    let end = read_date(EventField::End, fields.end.as_deref())?;
    if end.is_some() && start.is_none() {
        return Err(EventFieldProblem {
            field: Some(EventField::Start),
            message: "An end needs a start as well.".to_owned(),
        });
    }
    let shown = fields_of_event_document(base);
    let mut document = base.as_object().cloned().unwrap_or_default();
    if fields.name != shown.name {
        document.insert("name".to_owned(), json!({ "und": fields.name }));
    }
    let venue = venue_with(base.get("venue"), shown.place.as_deref(), fields.place.as_deref());
    set_or_remove(&mut document, "venue", venue);
    let occurrences = occurrences_with(base.get("occurrences"), start, end);
    set_or_remove(&mut document, "occurrences", occurrences);
    let components = components_with(
        base.get("components"),
        shown.description.as_deref(),
        fields.description.as_deref(),
    );
    set_or_remove(&mut document, "components", components);
    document.retain(|_, value| !value.is_null());
    let document = Value::Object(document);
    validate_event_document(&document)?;
    Ok(document)
}
