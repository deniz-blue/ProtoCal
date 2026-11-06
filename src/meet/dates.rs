use jiff::civil::Date;
use serde_json::Value;

use crate::open_evnt::{PartialDate, Precision};

const SECONDS_PER_DAY: i64 = 86_400;
const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];
const WEEKDAY_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

fn unix_seconds_of(date: &PartialDate) -> i64 {
    date.start_epoch_milliseconds().div_euclid(1000)
}

fn month_name(month: i8) -> &'static str {
    MONTH_NAMES[usize::try_from(month - 1).unwrap_or(0)]
}

fn date_as_discord(date: &PartialDate) -> String {
    let (month, day) = (date.month.unwrap_or(1), date.day.unwrap_or(1));
    match date.precision() {
        Precision::Time => format!("<t:{}:f>", unix_seconds_of(date)),
        Precision::Day => {
            let weekday = Date::new(date.year as i16, month, day)
                .map(|civil| WEEKDAY_NAMES[civil.weekday().to_monday_zero_offset() as usize])
                .unwrap_or_default();
            format!("{weekday}, {day} {} {}", month_name(month), date.year)
        }
        Precision::Month => format!("{} {}", month_name(month), date.year),
        Precision::Year => date.year.to_string(),
    }
}

fn occurrence_as_discord(occurrence: &Value, relative: bool) -> Option<String> {
    let written_start = occurrence.get("start")?.as_str()?;
    let start = PartialDate::parse(written_start)?;
    let written_end = occurrence.get("end").and_then(Value::as_str);
    let until = written_end.and_then(PartialDate::parse);
    let range = match (&until, written_end) {
        (Some(until), Some(written_end)) => {
            let same_day = start.precision() == Precision::Time
                && until.precision() == Precision::Time
                && written_start.get(..10) == written_end.get(..10)
                && start.time_zone_id == until.time_zone_id;
            let end = if same_day {
                format!("<t:{}:t>", unix_seconds_of(until))
            } else {
                date_as_discord(until)
            };
            format!("{} – {end}", date_as_discord(&start))
        }
        _ => date_as_discord(&start),
    };
    let shown = if relative && start.has(Precision::Day) {
        format!("{range} <t:{}:R>", unix_seconds_of(&start))
    } else {
        range
    };
    let cancelled = occurrence.get("status").and_then(Value::as_str) == Some("cancelled");
    Some(if cancelled { format!("~~{shown}~~") } else { shown })
}

fn occurrences_of(document: &Value) -> &[Value] {
    document
        .get("occurrences")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

pub fn meet_dates_as_discord(document: &Value, relative: bool) -> Option<String> {
    let dates: Vec<String> = occurrences_of(document)
        .iter()
        .filter_map(|occurrence| occurrence_as_discord(occurrence, relative))
        .collect();
    (!dates.is_empty()).then(|| dates.join(", "))
}

pub fn meet_is_over_at(document: &Value) -> Option<i64> {
    occurrences_of(document)
        .iter()
        .filter_map(|occurrence| occurrence.get("end").or_else(|| occurrence.get("start")))
        .filter_map(Value::as_str)
        .filter_map(PartialDate::parse)
        .map(|date| date.end_epoch_milliseconds().div_euclid(1000))
        .max()
        .map(|last| last + SECONDS_PER_DAY)
}
