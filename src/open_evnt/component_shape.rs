use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use super::checks::{
    COMPONENT_TYPE_PATTERN, Issues, LANGUAGE_TAG_PATTERN, ObjectShape, Path, absolute_uri, array_of, boolean, key,
    matching, number_where, object, report, string, translations,
};

const COORDINATES_TYPE: &str = "directory.evnt.component.coordinates#0";

static HEX_COLOUR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#[0-9A-Fa-f]{6}$").expect("hex colour pattern"));

pub fn shaped<'a>(shape: &'a ObjectShape<'a>) -> impl Fn(&Value, &Path, &mut Issues) + 'a {
    move |value, path, issues| object(shape, value, path, issues)
}

pub fn list(
    item: &'_ dyn Fn(&Value, &Path, &mut Issues),
    non_empty: bool,
) -> impl Fn(&Value, &Path, &mut Issues) + '_ {
    move |value, path, issues| array_of(item, non_empty, value, path, issues)
}

fn media(value: &Value, path: &Path, issues: &mut Issues) {
    let any_number = number_where(|_| true, "a number");
    let dimensions_shape = ObjectShape {
        required: &[("width", &any_number), ("height", &any_number)],
        ..Default::default()
    };
    let dimensions = shaped(&dimensions_shape);
    let source_shape = ObjectShape {
        required: &[("url", &absolute_uri)],
        optional: &[("mimeType", &string), ("dimensions", &dimensions)],
        ..Default::default()
    };
    let source = shaped(&source_shape);
    let sources = list(&source, true);
    let dominant_colour = matching(&HEX_COLOUR, "a hex colour such as #1a1a2e");
    let presentation_shape = ObjectShape {
        optional: &[("blurhash", &string), ("dominantColor", &dominant_colour)],
        ..Default::default()
    };
    let presentation = shaped(&presentation_shape);
    let shape = ObjectShape {
        required: &[("sources", &sources)],
        optional: &[("alt", &translations), ("presentation", &presentation)],
        ..Default::default()
    };
    object(&shape, value, path, issues);
}

fn known_component(component_type: &str, value: &Value, path: &Path, issues: &mut Issues) {
    let mut check = |shape: ObjectShape| object(&shape, value, path, issues);
    match component_type {
        "directory.evnt.text.markdown#0" => check(ObjectShape {
            required: &[("content", &translations)],
            optional: &[("flavor", &string)],
            ..Default::default()
        }),
        "directory.evnt.text.plain#0" => check(ObjectShape {
            required: &[("content", &translations)],
            ..Default::default()
        }),
        "directory.evnt.component.link#0" => check(ObjectShape {
            required: &[("url", &absolute_uri)],
            optional: &[("name", &translations), ("disabled", &boolean)],
            ..Default::default()
        }),
        "directory.evnt.component.source#0" => check(ObjectShape {
            required: &[("url", &absolute_uri)],
            ..Default::default()
        }),
        "directory.evnt.component.splashMedia#0" => check(ObjectShape {
            required: &[("media", &media)],
            optional: &[("roles", &list(&string, false))],
            ..Default::default()
        }),
        "directory.evnt.component.languages#0" => {
            let code = matching(&LANGUAGE_TAG_PATTERN, "a BCP 47 language tag such as en");
            let language_shape = ObjectShape {
                required: &[("code", &code)],
                ..Default::default()
            };
            let language = shaped(&language_shape);
            check(ObjectShape {
                required: &[("languages", &list(&language, false))],
                ..Default::default()
            })
        }
        "directory.evnt.component.placeId#0" => check(ObjectShape {
            required: &[("type", &string), ("id", &string)],
            ..Default::default()
        }),
        "directory.evnt.component.relation#0" => check(ObjectShape {
            required: &[("type", &string), ("target", &absolute_uri)],
            ..Default::default()
        }),
        COORDINATES_TYPE => {
            let latitude = number_where(|degrees| (-90.0..=90.0).contains(&degrees), "a number from -90 to 90");
            let longitude = number_where(|degrees| (-180.0..=180.0).contains(&degrees), "a number from -180 to 180");
            let uncertainty = number_where(|metres| metres > 0.0, "a number above 0");
            check(ObjectShape {
                required: &[("latitude", &latitude), ("longitude", &longitude)],
                optional: &[("uncertainty", &uncertainty)],
                ..Default::default()
            })
        }
        _ => {}
    }
}

fn component(value: &Value, path: &Path, issues: &mut Issues) {
    let Some(fields) = value.as_object() else {
        return report(issues, "must be a component object with a $type", path.clone());
    };
    let Some(component_type) = fields.get("$type") else {
        return report(issues, "is required", key(path, "$type"));
    };
    match component_type.as_str() {
        Some(text) if COMPONENT_TYPE_PATTERN.is_match(text) => {
            known_component(text, value, path, issues)
        }
        _ => report(
            issues,
            "must be a reverse domain name and a version, such as com.example.ticketTier#0",
            key(path, "$type"),
        ),
    }
}

pub fn components(value: &Value, path: &Path, issues: &mut Issues) {
    array_of(&component, false, value, path, issues);
}

pub fn venue_components(value: &Value, path: &Path, issues: &mut Issues) {
    components(value, path, issues);
    let coordinates = value.as_array().map_or(0, |items| {
        items
            .iter()
            .filter(|item| item.get("$type").and_then(Value::as_str) == Some(COORDINATES_TYPE))
            .count()
    });
    if coordinates > 1 {
        report(issues, "must not hold more than one coordinates component", path.clone());
    }
}
