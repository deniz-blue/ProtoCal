use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use super::{
    checks::{
        Issues, ObjectShape, Path, absolute_uri, key, matching, object, one_of, partial_date, report, string,
        translations,
    },
    component_shape::{components, list, venue_components},
    partial_date::PartialDate,
};

const EVENT_RECORD_TYPE: &str = "directory.evnt.event";

static COUNTRY_CODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z]{2}$").expect("country code pattern"));

fn venue(value: &Value, path: &Path, issues: &mut Issues) {
    let country_code = matching(&COUNTRY_CODE, "an uppercase ISO 3166-1 alpha-2 code such as LT");
    let shape = ObjectShape {
        optional: &[
            ("name", &translations),
            ("address", &string),
            ("countryCode", &country_code),
            ("components", &venue_components),
        ],
        closed: true,
        non_empty: true,
        ..Default::default()
    };
    object(&shape, value, path, issues);
}

fn activity(value: &Value, path: &Path, issues: &mut Issues) {
    let status = one_of(&["cancelled"]);
    let shape = ObjectShape {
        required: &[("name", &translations)],
        optional: &[("status", &status)],
        closed: true,
        ..Default::default()
    };
    object(&shape, value, path, issues);
}

fn occurrence(value: &Value, path: &Path, issues: &mut Issues) {
    let status = one_of(&["cancelled"]);
    let activities = list(&activity, false);
    let shape = ObjectShape {
        optional: &[
            ("start", &partial_date),
            ("end", &partial_date),
            ("status", &status),
            ("activities", &activities),
            ("components", &components),
        ],
        closed: true,
        ..Default::default()
    };
    object(&shape, value, path, issues);
    let date = |name: &str| value.get(name).and_then(Value::as_str).and_then(PartialDate::parse);
    if let (Some(start), Some(end)) = (date("start"), date("end"))
        && start.is_after(&end)
    {
        report(issues, "must not be after end", key(path, "start"));
    }
}

pub fn open_evnt(value: &Value, path: &Path, issues: &mut Issues) {
    let version = one_of(&["0.2"]);
    let record_type = one_of(&[EVENT_RECORD_TYPE]);
    let status = one_of(&["uncertain", "cancelled", "postponed"]);
    let occurrences = list(&occurrence, false);
    let shape = ObjectShape {
        required: &[("v", &version), ("id", &absolute_uri), ("name", &translations)],
        optional: &[
            ("$type", &record_type),
            ("status", &status),
            ("venue", &venue),
            ("occurrences", &occurrences),
            ("components", &components),
        ],
        closed: true,
        ..Default::default()
    };
    object(&shape, value, path, issues);
}
