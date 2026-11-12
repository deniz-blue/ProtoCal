use serde_json::Value;
use twilight_model::id::{Id, marker::ChannelMarker};

use super::{dates::event_dates_as_discord, document::event_name_of};

pub const RELATION_TYPE: &str = "directory.evnt.component.relation#0";
const PARENT: &str = "parent";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParentLink {
    Thread(Id<ChannelMarker>),
    Outside { view_url: String, name: Option<String> },
}

impl ParentLink {
    pub fn outside(view_url: String, parent: Option<&Value>) -> Self {
        Self::Outside { view_url, name: parent.map(event_name_of).filter(|name| !name.is_empty()) }
    }

    pub fn markdown(&self) -> String {
        match self {
            Self::Thread(thread_id) => format!("<#{thread_id}>"),
            Self::Outside { view_url, name } => {
                let name = name.as_deref().unwrap_or("another event").replace(['[', ']'], "");
                format!("[{name}](<{view_url}>)")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartOf {
    pub link: ParentLink,
    pub dates: Option<String>,
}

impl PartOf {
    pub fn of_parent(link: ParentLink, parent: Option<&Value>) -> Self {
        Self { link, dates: parent.and_then(|parent| event_dates_as_discord(parent, false)) }
    }

    pub fn line(&self) -> String {
        let link = format!("🧩 **Part of:** {}", self.link.markdown());
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

pub fn parent_url_of(document: &Value) -> Option<String> {
    let components = document.get("components")?.as_array()?;
    let relation = components.iter().find(|component| is_parent_relation(component))?;
    relation.get("target")?.as_str().map(str::to_owned)
}
