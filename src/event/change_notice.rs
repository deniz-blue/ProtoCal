use serde_json::Value;

use super::{dates::event_dates_as_discord, document::event_place_of};

pub fn event_status_as_discord(document: &Value) -> Option<&'static str> {
    match document.get("status")?.as_str()? {
        "uncertain" => Some("⚠️ **May not happen as listed**"),
        "cancelled" => Some("❌ **Cancelled**"),
        "postponed" => Some("⏸️ **Postponed**, new dates not known yet"),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventChangeNotice {
    pub lines: Vec<String>,
    pub pings_role: bool,
}

fn change_line(icon: &str, before: Option<String>, after: Option<String>, unset: &str) -> String {
    match before.filter(|before| !before.is_empty()) {
        Some(before) => format!("{icon} ~~{before}~~ {}", after.as_deref().unwrap_or(unset)),
        None => format!("{icon} {}", after.unwrap_or_default()),
    }
}

pub fn event_change_notice(before: &Value, after: &Value) -> Option<EventChangeNotice> {
    let mut lines = Vec::new();
    if before.get("status") != after.get("status") {
        let status = event_status_as_discord(after).unwrap_or("✅ **On as listed again**");
        lines.push(status.to_owned());
    }
    let dates_before = event_dates_as_discord(before, false);
    let dates_after = event_dates_as_discord(after, false);
    if dates_before != dates_after {
        lines.push(change_line("📆", dates_before, dates_after, "no time yet"));
    }
    let place_before = event_place_of(before);
    let place_after = event_place_of(after);
    if place_before != place_after {
        lines.push(change_line("📍", place_before, place_after, "no place yet"));
    }
    if !lines.is_empty() {
        return Some(EventChangeNotice {
            lines,
            pings_role: true,
        });
    }
    let (text_before, text_after) = (before.to_string(), after.to_string());
    if text_before != text_after {
        return Some(EventChangeNotice {
            lines: vec!["-# The details were updated.".to_owned()],
            pings_role: false,
        });
    }
    None
}
