use std::sync::LazyLock;

use regex::Regex;

pub const EVENT_TIME_ZONE: &str = "Europe/Vilnius";

static WRITTEN_DATE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([0-9]{4})(?:-([0-9]{2})(?:-([0-9]{2})(?:[ T]([0-9]{2}):([0-9]{2}))?)?)?(?:\s*\[([^\]\s]+)\])?$")
        .expect("written date pattern")
});

pub fn partial_date_of_written(written: &str) -> Option<String> {
    let parts = WRITTEN_DATE.captures(written.trim())?;
    let part = |index: usize| parts.get(index).map(|found| found.as_str());
    let date: Vec<&str> = [part(1), part(2), part(3)].into_iter().flatten().collect();
    let time = match (part(4), part(5)) {
        (Some(hour), Some(minute)) => format!("T{hour}:{minute}"),
        _ => String::new(),
    };
    let time_zone = part(6).unwrap_or(EVENT_TIME_ZONE);
    Some(format!("{}{time}[{time_zone}]", date.join("-")))
}

pub fn written_of_partial_date(partial_date: &str) -> String {
    let Some((date, zone)) = partial_date
        .strip_suffix(']')
        .and_then(|inner| inner.split_once('['))
        .filter(|(date, _)| !date.is_empty())
    else {
        return partial_date.to_owned();
    };
    let written = date.replacen('T', " ", 1);
    if zone == EVENT_TIME_ZONE {
        written
    } else {
        format!("{written} [{zone}]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_dates_become_vilnius_partial_dates_at_the_precision_written() {
        let cases = [
            ("2026-12-05 18:00", "2026-12-05T18:00[Europe/Vilnius]"),
            (" 2026-12-05T18:00 ", "2026-12-05T18:00[Europe/Vilnius]"),
            ("2026-12-05", "2026-12-05[Europe/Vilnius]"),
            ("2026-12", "2026-12[Europe/Vilnius]"),
            ("2026-12-05 18:00 [Europe/London]", "2026-12-05T18:00[Europe/London]"),
        ];
        for (written, expected) in cases {
            assert_eq!(partial_date_of_written(written).as_deref(), Some(expected));
        }
        for written in ["05/12/2026", "2026-12-05 6pm", "saturday", "2026-12-05 18"] {
            assert_eq!(partial_date_of_written(written), None, "{written}");
        }
    }

    #[test]
    fn a_partial_date_is_written_back_without_the_event_zone() {
        assert_eq!(written_of_partial_date("2026-12-05T18:00[Europe/Vilnius]"), "2026-12-05 18:00");
        assert_eq!(
            written_of_partial_date("2026-12-05[Europe/London]"),
            "2026-12-05 [Europe/London]"
        );
        assert_eq!(written_of_partial_date("nonsense"), "nonsense");
    }
}
