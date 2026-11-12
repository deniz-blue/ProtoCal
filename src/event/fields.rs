use jiff::civil::Date;
use serde_json::{Map, Value, json};

use super::document::{
    DESCRIPTION_TYPE, EventField, EventFieldProblem, event_description_of,
    event_name_of, event_place_of, is_description, validate_event_document,
};
use super::parent::{is_parent_relation, parent_relation, parent_url_of};
use super::written_date::{partial_dates_of_written, written_of_partial_dates};
use crate::open_evnt::remote::https_url_of;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventFields {
    pub name: String,
    pub date: Option<String>,
    pub place: Option<String>,
    pub description: Option<String>,
    pub part_of: Option<String>,
}

pub fn fields_of_event_document(document: &Value) -> EventFields {
    let first = document.get("occurrences").and_then(|all| all.get(0));
    let partial_date = |name: &str| first.and_then(|occurrence| occurrence.get(name)).and_then(Value::as_str);
    EventFields {
        name: event_name_of(document),
        date: partial_date("start").map(|start| written_of_partial_dates(start, partial_date("end"))),
        place: event_place_of(document),
        description: event_description_of(document),
        part_of: parent_url_of(document),
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

fn components_with(components: Option<&Value>, shown: &EventFields, fields: &EventFields) -> Option<Value> {
    let description_changed = fields.description != shown.description;
    let part_of_changed = fields.part_of != shown.part_of;
    if !description_changed && !part_of_changed {
        return components.cloned();
    }
    let written = description_changed
        .then(|| fields.description.as_deref().map(|text| json!({ "$type": DESCRIPTION_TYPE, "content": { "und": text } })))
        .flatten();
    let kept = components.and_then(Value::as_array).into_iter().flatten().filter(|component| {
        !(description_changed && is_description(component)) && !(part_of_changed && is_parent_relation(component))
    });
    let parent = part_of_changed.then(|| fields.part_of.as_deref().map(parent_relation)).flatten();
    let changed: Vec<Value> = written.into_iter().chain(kept.cloned()).chain(parent).collect();
    (!changed.is_empty()).then_some(Value::Array(changed))
}

fn read_part_of(written: Option<&str>) -> Result<Option<String>, EventFieldProblem> {
    let Some(written) = written else {
        return Ok(None);
    };
    https_url_of(written).map(|url| Some(url.to_string())).map_err(|problem| EventFieldProblem {
        field: Some(EventField::PartOf),
        message: format!("Part of: `{written}` {problem}. Paste the link of an Open Evnt event."),
    })
}

fn read_dates(written: Option<&str>, today: Date) -> Result<(Option<String>, Option<String>), EventFieldProblem> {
    let Some(written) = written else {
        return Ok((None, None));
    };
    let dates = partial_dates_of_written(written, today).ok_or_else(|| EventFieldProblem {
        field: Some(EventField::Date),
        message: format!(
            "Date: `{written}` is not a date I can read. Try `12 October 18:00`, `12.10 18:00 - 22:00`, `5-7 December` or `2026-12`."
        ),
    })?;
    Ok((Some(dates.start), dates.end))
}

pub fn event_document_of_fields(fields: &EventFields, base: &Value, today: Date) -> Result<Value, EventFieldProblem> {
    let (start, end) = read_dates(fields.date.as_deref(), today)?;
    let fields = &EventFields { part_of: read_part_of(fields.part_of.as_deref())?, ..fields.clone() };
    let shown = fields_of_event_document(base);
    let mut document = base.as_object().cloned().unwrap_or_default();
    if fields.name != shown.name {
        document.insert("name".to_owned(), json!({ "und": fields.name }));
    }
    let venue = venue_with(base.get("venue"), shown.place.as_deref(), fields.place.as_deref());
    set_or_remove(&mut document, "venue", venue);
    let occurrences = occurrences_with(base.get("occurrences"), start, end);
    set_or_remove(&mut document, "occurrences", occurrences);
    let components = components_with(base.get("components"), &shown, fields);
    set_or_remove(&mut document, "components", components);
    document.retain(|_, value| !value.is_null());
    let document = Value::Object(document);
    validate_event_document(&document)?;
    Ok(document)
}
