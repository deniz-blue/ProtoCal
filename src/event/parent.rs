use serde_json::{Value, json};

use super::{dates::event_dates_as_discord, document::event_name_of};

pub const RELATION_TYPE: &str = "directory.evnt.component.relation#0";
const PARENT: &str = "parent";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartOf {
    pub view_url: String,
    pub name: Option<String>,
    pub dates: Option<String>,
}

impl PartOf {
    pub fn of_parent(view_url: String, parent: Option<&Value>) -> Self {
        Self {
            view_url,
            name: parent.map(event_name_of).filter(|name| !name.is_empty()),
            dates: parent.and_then(|parent| event_dates_as_discord(parent, false)),
        }
    }

    pub fn line(&self) -> String {
        let name = self.name.as_deref().unwrap_or("another event").replace(['[', ']'], "");
        let link = format!("🧩 **Part of:** [{name}](<{}>)", self.view_url);
        match &self.dates {
            Some(dates) => format!("{link} · {dates}"),
            None => link,
        }
    }
}

pub fn is_parent_relation(component: &Value) -> bool {
    component.get("$type").and_then(Value::as_str) == Some(RELATION_TYPE)
        && component.get("type").and_then(Value::as_str) == Some(PARENT)
}

pub fn parent_relation(target: &str) -> Value {
    json!({ "$type": RELATION_TYPE, "type": PARENT, "target": target })
}

pub fn parent_url_of(document: &Value) -> Option<String> {
    let components = document.get("components")?.as_array()?;
    let relation = components.iter().find(|component| is_parent_relation(component))?;
    relation.get("target")?.as_str().map(str::to_owned)
}
