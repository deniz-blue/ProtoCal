mod checks;
mod component_shape;
mod document_shape;
pub mod partial_date;
#[cfg(test)]
mod partial_date_tests;

use serde_json::Value;

pub use checks::{Issue, PathSegment};
pub use partial_date::{PartialDate, Precision};

pub fn validate(document: &Value) -> Result<(), Vec<Issue>> {
    let mut issues = vec![];
    document_shape::open_evnt(document, &vec![], &mut issues);
    if issues.is_empty() { Ok(()) } else { Err(issues) }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn first_issue(document: Value) -> (String, String) {
        let issue = validate(&document).unwrap_err().remove(0);
        let path: Vec<String> = issue
            .path
            .iter()
            .map(|segment| match segment {
                PathSegment::Key(key) => key.clone(),
                PathSegment::Index(index) => index.to_string(),
            })
            .collect();
        (path.join("."), issue.message)
    }

    #[test]
    fn a_small_document_is_valid_and_unknown_components_pass() {
        let document = json!({
            "v": "0.2",
            "id": "urn:uuid:5f0c6a52",
            "name": { "und": "Bar meet" },
            "venue": { "address": "Snekutis" },
            "occurrences": [{ "start": "2026-12-05T18:00[Europe/Vilnius]" }],
            "components": [{ "$type": "com.example.ticketTier#0", "price": 3 }],
        });
        assert_eq!(validate(&document), Ok(()));
    }

    #[test]
    fn issues_name_the_path_and_say_what_is_wrong() {
        let base = json!({ "v": "0.2", "id": "https://p.test/1", "name": { "und": "x" } });
        let with = |field: &str, value: Value| {
            let mut document = base.clone();
            document[field] = value;
            first_issue(document)
        };
        assert_eq!(
            with(
                "occurrences",
                json!([{ "start": "2026-12-05T18:00[UTC]", "end": "2026-12-05T17:00[UTC]" }])
            ),
            ("occurrences.0.start".to_owned(), "must not be after end".to_owned())
        );
        assert_eq!(
            with("name", json!({ "en": "a", "EN": "b" })),
            ("name.EN".to_owned(), "differs from en only in case".to_owned())
        );
        assert_eq!(
            with("venue", json!({})),
            ("venue".to_owned(), "must have at least one field".to_owned())
        );
        assert_eq!(
            with("status", json!("over")),
            (
                "status".to_owned(),
                "must be \"uncertain\" or \"cancelled\" or \"postponed\"".to_owned()
            )
        );
        assert_eq!(
            with("extra", json!(1)),
            ("extra".to_owned(), "is not a field of this object".to_owned())
        );
        assert_eq!(
            first_issue(json!({ "v": "0.2", "id": "nope", "name": { "und": "x" } })).0,
            "id"
        );
    }
}
