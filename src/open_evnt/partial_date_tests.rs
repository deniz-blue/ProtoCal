use jiff::Timestamp;

use super::partial_date::*;

fn unix_millis(iso: &str) -> i64 {
    iso.parse::<Timestamp>().unwrap().as_millisecond()
}

#[test]
fn only_dates_that_exist_in_known_zones_are_valid() {
    assert!(PartialDate::is_valid("2026-06-15T18:00[Europe/London]"));
    assert!(PartialDate::is_valid("2024-02-29[UTC]"));
    assert!(PartialDate::is_valid("2026[Etc/GMT+5]"));
    for text in [
        "2026-02-29[UTC]",
        "2026-13[UTC]",
        "2026-06-15T24:00[UTC]",
        "2026-06-15[Mars/Olympus]",
        "2026-06-15",
        "2026-06-15 18:00[UTC]",
    ] {
        assert!(!PartialDate::is_valid(text), "{text}");
    }
}

#[test]
fn a_period_runs_from_its_first_instant_to_the_first_instant_after_it() {
    let day = PartialDate::parse("2026-03-29[Europe/Vilnius]").unwrap();
    assert_eq!(
        (day.end_epoch_milliseconds() - day.start_epoch_milliseconds()) / 3_600_000,
        23
    );
    let month = PartialDate::parse("2026-12[Europe/Vilnius]").unwrap();
    assert_eq!(month.start_epoch_milliseconds(), unix_millis("2026-11-30T22:00:00Z"));
    assert_eq!(month.end_epoch_milliseconds(), unix_millis("2026-12-31T22:00:00Z"));
    let time = PartialDate::parse("2026-12-05T18:00[Europe/Vilnius]").unwrap();
    assert_eq!(time.end_epoch_milliseconds(), unix_millis("2026-12-05T16:00:00Z"));
}

#[test]
fn a_skipped_time_uses_the_offset_before_and_a_repeated_one_the_first() {
    let skipped = PartialDate::parse("2026-03-29T03:30[Europe/Vilnius]").unwrap();
    assert_eq!(skipped.start_epoch_milliseconds(), unix_millis("2026-03-29T01:30:00Z"));
    let repeated = PartialDate::parse("2026-10-25T03:30[Europe/Vilnius]").unwrap();
    assert_eq!(repeated.start_epoch_milliseconds(), unix_millis("2026-10-25T00:30:00Z"));
}

#[test]
fn a_day_is_not_after_a_time_inside_it() {
    let day = PartialDate::parse("2025-11-12[Europe/London]").unwrap();
    let evening = PartialDate::parse("2025-11-12T18:00[Europe/London]").unwrap();
    assert!(!day.is_after(&evening));
    assert!(!evening.is_after(&day));
    let later = PartialDate::parse("2025-11-13[Europe/London]").unwrap();
    assert!(later.is_after(&evening));
}
