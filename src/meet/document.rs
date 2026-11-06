use serde_json::{Value, json};

use crate::open_evnt::{self, Issue, PathSegment};

pub const DESCRIPTION_TYPE: &str = "directory.evnt.text.markdown#0";
const SHOWN_LANGUAGES: [&str; 4] = ["mul", "und", "en", "lt"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeetField {
    Name,
    Start,
    End,
    Place,
    Description,
}

impl MeetField {
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Start => "Start",
            Self::End => "End",
            Self::Place => "Place",
            Self::Description => "Description",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeetFieldProblem {
    pub field: Option<MeetField>,
    pub message: String,
}

pub fn new_meet_document(id: &str, name: &str) -> Value {
    json!({ "v": "0.2", "id": id, "name": { "und": name } })
}

pub fn text_of(translations: Option<&Value>) -> Option<String> {
    let texts = translations?.as_object()?;
    let language = SHOWN_LANGUAGES
        .iter()
        .find(|candidate| texts.contains_key(**candidate))
        .map(|language| language.to_string())
        .or_else(|| texts.keys().next().cloned())?;
    texts.get(&language)?.as_str().map(str::to_owned)
}

pub fn is_description(component: &Value) -> bool {
    component.get("$type").and_then(Value::as_str) == Some(DESCRIPTION_TYPE)
}

pub fn meet_description_of(document: &Value) -> Option<String> {
    let components = document.get("components")?.as_array()?;
    let description = components.iter().find(|component| is_description(component))?;
    text_of(description.get("content"))
}

pub fn meet_place_of(document: &Value) -> Option<String> {
    let venue = document.get("venue")?;
    match venue.get("address") {
        Some(Value::String(address)) => Some(address.clone()),
        _ => text_of(venue.get("name")),
    }
}

pub fn meet_name_of(document: &Value) -> String {
    text_of(document.get("name")).unwrap_or_default()
}

fn field_of_issue_path(path: &[PathSegment]) -> Option<MeetField> {
    let key = |position: usize| match path.get(position) {
        Some(PathSegment::Key(key)) => Some(key.as_str()),
        _ => None,
    };
    match key(0)? {
        "name" => Some(MeetField::Name),
        "venue" => Some(MeetField::Place),
        "components" => Some(MeetField::Description),
        "occurrences" if path.get(1) == Some(&PathSegment::Index(0)) => match key(2)? {
            "start" => Some(MeetField::Start),
            "end" => Some(MeetField::End),
            _ => None,
        },
        _ => None,
    }
}

fn problem_of_issue(issue: &Issue) -> MeetFieldProblem {
    let field = field_of_issue_path(&issue.path);
    let message = match field {
        Some(field) => format!("{} {}.", field.label(), issue.message),
        None => format!("{}.", issue.message),
    };
    MeetFieldProblem { field, message }
}

pub fn validate_meet_document(document: &Value) -> Result<(), MeetFieldProblem> {
    open_evnt::validate(document).map_err(|issues| problem_of_issue(&issues[0]))
}
