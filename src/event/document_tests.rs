use serde_json::{Value, json};

use super::{
    change_notice::{EventChangeNotice, event_change_notice},
    dates::{event_dates_as_discord, event_is_over_at},
    document::{EventField, EventFieldProblem, new_event_document},
    fields::{EventFields, event_document_of_fields, fields_of_event_document},
};

const ID: &str = "http://localhost:8660/events/1.evnt.json";

fn unix(iso: &str) -> i64 {
    iso.parse::<jiff::Timestamp>().unwrap().as_second()
}

#[derive(Default)]
struct Changed<'a> {
    start: Option<&'a str>,
    end: Option<&'a str>,
    place: Option<&'a str>,
    description: Option<&'a str>,
}

fn fields(changed: Changed) -> EventFields {
    EventFields {
        name: "Bar meet".to_owned(),
        start: changed.start.map(str::to_owned),
        end: changed.end.map(str::to_owned),
        place: changed.place.map(str::to_owned),
        description: changed.description.map(str::to_owned),
    }
}

fn document_on(changed: Changed, base: &Value) -> Value {
    event_document_of_fields(&fields(changed), base).unwrap()
}

fn document_of(changed: Changed) -> Value {
    document_on(changed, &new_event_document(ID, "Bar meet"))
}

fn problem_of(changed: Changed) -> EventFieldProblem {
    event_document_of_fields(&fields(changed), &new_event_document(ID, "Bar meet")).unwrap_err()
}

#[test]
fn the_form_makes_a_valid_open_evnt_document() {
    let document = document_of(Changed {
        start: Some("2026-12-05 18:00"),
        end: Some("2026-12-06 02:00"),
        place: Some("Snekutis, Vilnius"),
        description: Some("Bring snacks"),
    });
    assert_eq!(
        document,
        json!({
            "v": "0.2",
            "id": ID,
            "name": { "und": "Bar meet" },
            "venue": { "address": "Snekutis, Vilnius" },
            "occurrences": [
                { "start": "2026-12-05T18:00[Europe/Vilnius]", "end": "2026-12-06T02:00[Europe/Vilnius]" }
            ],
            "components": [{ "$type": "directory.evnt.text.markdown#0", "content": { "und": "Bring snacks" } }],
        })
    );
}

#[test]
fn dates_that_do_not_exist_or_an_end_before_the_start_are_refused_with_the_field() {
    let start = Some(EventField::Start);
    assert_eq!(problem_of(Changed { start: Some("2026-02-30"), ..Default::default() }).field, start);
    let backwards = Changed { start: Some("2026-12-05 18:00"), end: Some("2026-12-05 17:00"), ..Default::default() };
    assert_eq!(problem_of(backwards).field, start);
    assert_eq!(problem_of(Changed { end: Some("2026-12-05"), ..Default::default() }).field, start);
    assert_eq!(problem_of(Changed { start: Some("next friday"), ..Default::default() }).field, start);
    assert_eq!(
        problem_of(Changed { start: Some("2026-02-30"), ..Default::default() }).message,
        "Start must be a PartialDate such as 2026-06-15T09:00[Europe/Vilnius]."
    );
}

#[test]
fn the_form_shows_what_the_document_holds_and_saving_it_unchanged_keeps_the_document() {
    let from_an_app = json!({
        "v": "0.2",
        "id": ID,
        "name": { "en": "Bar meet", "lt": "Baro susitikimas" },
        "venue": { "name": { "en": "Snekutis" }, "address": "Stiklių g. 7, Vilnius", "countryCode": "LT" },
        "occurrences": [
            { "start": "2026-12-05T18:00[Europe/Vilnius]" },
            { "start": "2026-12-12T18:00[Europe/Vilnius]" },
        ],
        "components": [{ "$type": "directory.evnt.link#0", "url": "https://example.com" }],
    });
    let shown = fields_of_event_document(&from_an_app);
    assert_eq!(
        shown,
        EventFields {
            name: "Bar meet".to_owned(),
            start: Some("2026-12-05 18:00".to_owned()),
            end: None,
            place: Some("Stiklių g. 7, Vilnius".to_owned()),
            description: None,
        }
    );
    assert_eq!(event_document_of_fields(&shown, &from_an_app).unwrap(), from_an_app);
}

#[test]
fn a_changed_start_keeps_the_other_dates_and_a_changed_place_keeps_the_venue_name() {
    let base = document_of(Changed { start: Some("2026-12-05 18:00"), ..Default::default() });
    let mut two_dates = base.clone();
    two_dates["occurrences"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "start": "2026-12-12T18:00[Europe/Vilnius]" }));
    let moved = document_on(Changed { start: Some("2026-12-05 19:00"), ..Default::default() }, &two_dates);
    assert_eq!(
        moved["occurrences"],
        json!([
            { "start": "2026-12-05T19:00[Europe/Vilnius]" },
            { "start": "2026-12-12T18:00[Europe/Vilnius]" },
        ])
    );
    let mut named = base.clone();
    named["venue"] = json!({ "name": { "en": "Snekutis" }, "address": "Stiklių g. 7" });
    let moved_place = Changed { start: Some("2026-12-05 18:00"), place: Some("Totorių g. 1"), ..Default::default() };
    assert_eq!(
        document_on(moved_place, &named)["venue"],
        json!({ "name": { "en": "Snekutis" }, "address": "Totorių g. 1" })
    );
}

#[test]
fn dates_read_in_discord_a_time_a_whole_day_several_days_a_month() {
    let at = unix("2026-12-05T16:00:00Z");
    let evening = document_of(Changed { start: Some("2026-12-05 18:00"), ..Default::default() });
    assert_eq!(event_dates_as_discord(&evening, true), Some(format!("<t:{at}:f> <t:{at}:R>")));
    let until_late = document_of(Changed { start: Some("2026-12-05 18:00"), end: Some("2026-12-05 23:00"), ..Default::default() });
    let late = unix("2026-12-05T21:00:00Z");
    assert_eq!(event_dates_as_discord(&until_late, true), Some(format!("<t:{at}:f> – <t:{late}:t> <t:{at}:R>")));
    let weekend = document_of(Changed { start: Some("2026-12-05"), end: Some("2026-12-07"), ..Default::default() });
    assert_eq!(
        event_dates_as_discord(&weekend, false).as_deref(),
        Some("Sat, 5 December 2026 – Mon, 7 December 2026")
    );
    let month = document_of(Changed { start: Some("2026-12"), ..Default::default() });
    assert_eq!(event_dates_as_discord(&month, true).as_deref(), Some("December 2026"));
    assert_eq!(event_dates_as_discord(&document_of(Changed::default()), true), None);
}

#[test]
fn an_event_is_over_a_day_after_it_ends_or_after_its_start_when_it_has_no_end() {
    let over = |changed| event_is_over_at(&document_of(changed));
    assert_eq!(over(Changed { start: Some("2026-12-05 18:00"), ..Default::default() }), Some(unix("2026-12-06T16:00:00Z")));
    assert_eq!(over(Changed { start: Some("2026-12-05"), ..Default::default() }), Some(unix("2026-12-06T22:00:00Z")));
    let weekend = Changed { start: Some("2026-12-05"), end: Some("2026-12-07"), ..Default::default() };
    assert_eq!(over(weekend), Some(unix("2026-12-08T22:00:00Z")));
    assert_eq!(over(Changed::default()), None);
}

#[test]
fn a_moved_time_or_place_pings_a_description_change_does_not_no_change_says_nothing() {
    let planned = || Changed { start: Some("2026-12-05 18:00"), end: None, place: Some("Bar"), description: Some("Snacks") };
    let before = document_of(planned());
    let moved = Changed { start: Some("2026-12-05 19:00"), place: Some("Park"), ..planned() };
    let notice = event_change_notice(&before, &document_on(moved, &before)).unwrap();
    assert_eq!(
        notice.lines,
        vec![
            format!("📆 ~~<t:{}:f>~~ <t:{}:f>", unix("2026-12-05T16:00:00Z"), unix("2026-12-05T17:00:00Z")),
            "📍 ~~Bar~~ Park".to_owned(),
        ]
    );
    assert!(notice.pings_role);
    let detailed = document_on(Changed { description: Some("More"), ..planned() }, &before);
    assert_eq!(
        event_change_notice(&before, &detailed),
        Some(EventChangeNotice { lines: vec!["-# The details were updated.".to_owned()], pings_role: false })
    );
    assert_eq!(event_change_notice(&before, &before), None);
    let mut cancelled = before.clone();
    cancelled["status"] = json!("cancelled");
    assert_eq!(event_change_notice(&before, &cancelled).unwrap().lines, vec!["❌ **Cancelled**".to_owned()]);
}
