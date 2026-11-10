use std::sync::LazyLock;

use jiff::{
    Span, Zoned,
    civil::{Date, Weekday},
};
use regex::Regex;

pub const EVENT_TIME_ZONE: &str = "Europe/Vilnius";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialDates {
    pub start: String,
    pub end: Option<String>,
}

const MONTH_STEMS: [(&str, i8); 34] = [
    ("jan", 1), ("feb", 2), ("mar", 3), ("apr", 4), ("may", 5), ("jun", 6),
    ("jul", 7), ("aug", 8), ("sep", 9), ("oct", 10), ("nov", 11), ("dec", 12),
    ("sau", 1), ("vas", 2), ("kov", 3), ("bal", 4), ("geg", 5), ("bir", 6),
    ("lie", 7), ("rugp", 8), ("rugs", 9), ("spa", 10), ("lap", 11), ("gru", 12),
    ("oca", 1), ("sub", 2), ("nis", 4), ("haz", 6), ("tem", 7), ("agu", 8),
    ("eyl", 9), ("eki", 10), ("kas", 11), ("ara", 12),
];

const WEEKDAY_STEMS: [(&str, Weekday); 21] = [
    ("mon", Weekday::Monday), ("tue", Weekday::Tuesday), ("wed", Weekday::Wednesday),
    ("thu", Weekday::Thursday), ("fri", Weekday::Friday), ("sat", Weekday::Saturday),
    ("sun", Weekday::Sunday),
    ("pirm", Weekday::Monday), ("antr", Weekday::Tuesday), ("trec", Weekday::Wednesday),
    ("ketv", Weekday::Thursday), ("penk", Weekday::Friday), ("sest", Weekday::Saturday),
    ("sekm", Weekday::Sunday),
    ("pazart", Weekday::Monday), ("sal", Weekday::Tuesday), ("cars", Weekday::Wednesday),
    ("pers", Weekday::Thursday), ("cuma", Weekday::Friday), ("cumart", Weekday::Saturday),
    ("pazar", Weekday::Sunday),
];

const DAYS_FROM_TODAY: [(&str, i8); 7] =
    [("today", 0), ("tonight", 0), ("siandien", 0), ("bugun", 0), ("tomorrow", 1), ("rytoj", 1), ("yarin", 1)];
const RANGE_WORDS: [&str; 7] = ["to", "until", "till", "through", "thru", "iki", "ila"];
const IGNORED_WORDS: [&str; 13] = ["of", "the", "on", "at", "from", "next", "this", "nuo", "m", "d", "gunu", "gun", "and"];
const HOUR_AFTER_WORDS: [&str; 3] = ["val", "h", "o"];
const HOUR_BEFORE_WORDS: [&str; 1] = ["saat"];

static SUFFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"['’ʼ]\p{L}*").expect("suffix pattern"));
static MERIDIEM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b([ap])\.\s?m\b\.?").expect("meridiem pattern"));
static TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?P<zone>\[[^\]\s]+\])
        | (?P<iso>\d{4}(?:-\d{1,2}){1,2})(?:[Tt](?P<iso_time>\d{1,2}:\d{2}))?
        | (?P<dotted>\d+(?:\.\d+)+)\.?
        | (?P<slashed>\d+(?:/\d+)+)
        | (?P<colon>\d{1,2}:\d{2})
        | (?P<number>\d+)(?:st|nd|rd|th|\.)?
        | (?P<word>\p{L}+)\.?
        | (?P<dash>[-–—~])
        | (?P<space>[\s,;]+)
        | (?P<other>.)",
    )
    .expect("token pattern")
});

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Zone(String),
    Numbers(Vec<(i32, usize)>),
    Time(i8, i8),
    Number(i32, usize),
    Word(Word),
    Range,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Word {
    Month(i8),
    Weekday(Weekday),
    DaysFromToday(i8),
    HourAfter,
    HourBefore,
    Meridiem { after_noon: bool },
    Ignored,
}

fn folded(word: &str) -> String {
    word.chars()
        .map(|letter| match letter {
            'İ' | 'I' | 'ı' => 'i',
            other => other,
        })
        .flat_map(char::to_lowercase)
        .map(|letter| match letter {
            'ą' | 'â' => 'a',
            'č' | 'ç' => 'c',
            'ę' | 'ė' => 'e',
            'į' | 'î' => 'i',
            'š' | 'ş' => 's',
            'ų' | 'ū' | 'ü' | 'û' => 'u',
            'ž' => 'z',
            'ğ' => 'g',
            'ö' => 'o',
            other => other,
        })
        .collect()
}

fn longest_stem<T: Copy>(word: &str, stems: &[(&str, T)]) -> Option<(usize, T)> {
    stems
        .iter()
        .filter(|(stem, _)| word.starts_with(stem))
        .map(|(stem, value)| (stem.len(), *value))
        .max_by_key(|(length, _)| *length)
}

fn word_of(written: &str) -> Option<Token> {
    let word = folded(written);
    let word = word.as_str();
    if RANGE_WORDS.contains(&word) {
        return Some(Token::Range);
    }
    let known = if IGNORED_WORDS.contains(&word) {
        Word::Ignored
    } else if HOUR_AFTER_WORDS.contains(&word) {
        Word::HourAfter
    } else if HOUR_BEFORE_WORDS.contains(&word) {
        Word::HourBefore
    } else if word == "am" || word == "pm" {
        Word::Meridiem { after_noon: word == "pm" }
    } else if let Some((_, days)) = DAYS_FROM_TODAY.iter().find(|(name, _)| *name == word) {
        Word::DaysFromToday(*days)
    } else if word.chars().count() < 3 {
        return None;
    } else {
        let month = longest_stem(word, &MONTH_STEMS).map(|(length, month)| (length, Word::Month(month)));
        let weekday = longest_stem(word, &WEEKDAY_STEMS).map(|(length, day)| (length, Word::Weekday(day)));
        [month, weekday].into_iter().flatten().max_by_key(|(length, _)| *length)?.1
    };
    Some(Token::Word(known))
}

fn numbers_of(text: &str, separator: char) -> Vec<(i32, usize)> {
    text.split(separator).map(|part| (part.parse().unwrap_or(i32::MAX), part.len())).collect()
}

fn time_of(hour: i32, minute: i32) -> Option<(i8, i8)> {
    ((0..24).contains(&hour) && (0..60).contains(&minute)).then_some((hour as i8, minute as i8))
}

fn tokens_of(written: &str) -> Option<Vec<Token>> {
    let text = MERIDIEM.replace_all(written, "${1}m");
    let text = SUFFIX.replace_all(&text, "");
    let mut tokens = vec![];
    for found in TOKEN.captures_iter(&text) {
        let group = |name: &str| found.name(name).map(|part| part.as_str());
        let token = if let Some(zone) = group("zone") {
            Token::Zone(zone.trim_matches(['[', ']']).to_owned())
        } else if let Some(iso) = group("iso") {
            tokens.push(Token::Numbers(numbers_of(iso, '-')));
            match group("iso_time") {
                Some(time) => Token::Numbers(numbers_of(time, ':')),
                None => continue,
            }
        } else if let Some(dotted) = group("dotted") {
            Token::Numbers(numbers_of(dotted, '.'))
        } else if let Some(slashed) = group("slashed") {
            Token::Numbers(numbers_of(slashed, '/'))
        } else if let Some(colon) = group("colon") {
            let parts = numbers_of(colon, ':');
            let (hour, minute) = time_of(parts[0].0, parts[1].0)?;
            Token::Time(hour, minute)
        } else if let Some(number) = group("number") {
            Token::Number(number.parse().ok()?, number.len())
        } else if let Some(word) = group("word") {
            word_of(word)?
        } else if group("dash").is_some() {
            Token::Range
        } else if group("space").is_some() {
            continue;
        } else {
            return None;
        };
        tokens.push(token);
    }
    Some(tokens)
}

#[derive(Clone, Debug, Default)]
struct WrittenSide {
    year: Option<i16>,
    month: Option<i8>,
    day: Option<i8>,
    weekday: Option<Weekday>,
    days_from_today: Option<i8>,
    time: Option<(i8, i8)>,
    zone: Option<String>,
    lone_number: Option<i32>,
}

impl WrittenSide {
    const fn has_date(&self) -> bool {
        self.year.is_some() || self.month.is_some() || self.day.is_some()
    }

    const fn has_day(&self) -> bool {
        self.day.is_some() || self.weekday.is_some() || self.days_from_today.is_some()
    }

    const fn is_empty(&self) -> bool {
        !self.has_date() && !self.has_day() && self.time.is_none() && self.lone_number.is_none()
    }

    fn take_month_of(&mut self, day: Date) {
        self.month = Some(day.month());
        self.year = self.year.or(Some(day.year()));
    }

    fn move_to_month_after(&mut self, day: Date) -> Option<()> {
        let next_month = day.first_of_month().checked_add(Span::new().months(1)).ok()?;
        self.month = Some(next_month.month());
        self.year = Some(next_month.year());
        Some(())
    }
}

fn year_of(number: i32, digits: usize) -> Option<i16> {
    match digits {
        2 => Some(2000 + number as i16),
        4 => i16::try_from(number).ok(),
        _ => None,
    }
}

fn small(number: i32, range: std::ops::RangeInclusive<i32>) -> Option<i8> {
    range.contains(&number).then_some(number as i8)
}

fn set_once<T>(slot: &mut Option<T>, value: T) -> Option<()> {
    match slot {
        Some(_) => None,
        None => {
            *slot = Some(value);
            Some(())
        }
    }
}

fn set_date_of_numbers(side: &mut WrittenSide, numbers: &[(i32, usize)]) -> Option<()> {
    let [first, rest @ ..] = numbers else { return None };
    if first.1 == 4 {
        set_once(&mut side.year, year_of(first.0, first.1)?)?;
        return match rest {
            [] => Some(()),
            [month] => set_once(&mut side.month, small(month.0, 1..=12)?),
            [month, day] => {
                set_once(&mut side.month, small(month.0, 1..=12)?)?;
                set_once(&mut side.day, small(day.0, 1..=31)?)
            }
            _ => None,
        };
    }
    let [month, year @ ..] = rest else { return None };
    set_once(&mut side.day, small(first.0, 1..=31)?)?;
    set_once(&mut side.month, small(month.0, 1..=12)?)?;
    match year {
        [] => Some(()),
        [year] => set_once(&mut side.year, year_of(year.0, year.1)?),
        _ => None,
    }
}

fn set_time_of_numbers(side: &mut WrittenSide, numbers: &[(i32, usize)]) -> Option<()> {
    let [(hour, _), (minute, 2)] = numbers else { return None };
    set_once(&mut side.time, time_of(*hour, *minute)?)
}

fn on_twenty_four_hour_clock(hour: i8, after_noon: bool) -> Option<i8> {
    (1..=12).contains(&hour).then_some(hour % 12 + if after_noon { 12 } else { 0 })
}

fn side_of(tokens: &[Token]) -> Option<WrittenSide> {
    let mut side = WrittenSide::default();
    let mut numbers: Vec<i32> = vec![];
    let mut year_before_numbers = false;
    let mut hour_next = false;
    for token in tokens {
        match token {
            Token::Zone(zone) => set_once(&mut side.zone, zone.clone())?,
            Token::Numbers(parts) if side.has_date() || side.has_day() || !numbers.is_empty() => {
                set_time_of_numbers(&mut side, parts)?
            }
            Token::Numbers(parts) => {
                let mut dated = side.clone();
                match set_date_of_numbers(&mut dated, parts) {
                    Some(()) => side = dated,
                    None => set_time_of_numbers(&mut side, parts)?,
                }
            }
            Token::Time(hour, minute) => set_once(&mut side.time, (*hour, *minute))?,
            Token::Number(number, _) if hour_next => {
                set_once(&mut side.time, time_of(*number, 0)?)?;
                hour_next = false;
            }
            Token::Number(number, 4) => {
                year_before_numbers = numbers.is_empty();
                set_once(&mut side.year, i16::try_from(*number).ok()?)?;
            }
            Token::Number(number, _) => numbers.push(*number),
            Token::Word(Word::Month(month)) => set_once(&mut side.month, *month)?,
            Token::Word(Word::Weekday(weekday)) => side.weekday = Some(*weekday),
            Token::Word(Word::DaysFromToday(days)) => side.days_from_today = Some(*days),
            Token::Word(Word::HourBefore) => hour_next = side.time.is_none(),
            Token::Word(Word::HourAfter) => set_once(&mut side.time, time_of(numbers.pop()?, 0)?)?,
            Token::Word(Word::Meridiem { after_noon }) => {
                let (hour, minute) = match side.time.take() {
                    Some(time) => time,
                    None => time_of(numbers.pop()?, 0)?,
                };
                side.time = Some((on_twenty_four_hour_clock(hour, *after_noon)?, minute));
            }
            Token::Word(Word::Ignored) => {}
            Token::Range => return None,
        }
    }
    let mut numbers = numbers.into_iter();
    if side.month.is_some() || side.day.is_some() {
        if let Some(number) = numbers.next() {
            if side.day.is_some() {
                set_once(&mut side.time, time_of(number, 0)?)?;
            } else {
                side.day = Some(small(number, 1..=31)?);
            }
        }
    } else {
        match (numbers.next(), numbers.next()) {
            (Some(first), Some(second)) if year_before_numbers => {
                side.month = Some(small(first, 1..=12)?);
                side.day = Some(small(second, 1..=31)?);
            }
            (Some(first), Some(second)) => {
                side.day = Some(small(first, 1..=31)?);
                side.month = Some(small(second, 1..=12)?);
            }
            (Some(lone), None) => side.lone_number = Some(lone),
            _ => {}
        }
    }
    if let Some(hour) = numbers.next() {
        set_once(&mut side.time, time_of(hour, 0)?)?;
    }
    numbers.next().is_none().then_some(side)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum When {
    Year(i16),
    Month(i16, i8),
    Day(Date),
    DayAt(Date, (i8, i8)),
}

impl When {
    fn first_day(self) -> Option<Date> {
        match self {
            Self::Year(year) => Date::new(year, 1, 1).ok(),
            Self::Month(year, month) => Date::new(year, month, 1).ok(),
            Self::Day(day) | Self::DayAt(day, _) => Some(day),
        }
    }

    fn written(self, zone: &str) -> String {
        let date = match self {
            Self::Year(year) => format!("{year:04}"),
            Self::Month(year, month) => format!("{year:04}-{month:02}"),
            Self::Day(day) => day.to_string(),
            Self::DayAt(day, (hour, minute)) => format!("{day}T{hour:02}:{minute:02}"),
        };
        format!("{date}[{zone}]")
    }
}

fn day_of(side: &WrittenSide, today: Date, earliest: Date) -> Option<Date> {
    let (Some(month), Some(day)) = (side.month, side.day) else {
        if let Some(days) = side.days_from_today {
            return today.checked_add(Span::new().days(days)).ok();
        }
        let weekday = side.weekday?;
        return if earliest.weekday() == weekday { Some(earliest) } else { earliest.nth_weekday(1, weekday).ok() };
    };
    match side.year {
        Some(year) => Date::new(year, month, day).ok(),
        None => match Date::new(earliest.year(), month, day) {
            Ok(date) if date >= earliest => Some(date),
            _ => Date::new(earliest.year() + 1, month, day).ok(),
        },
    }
}

fn when_of(side: &WrittenSide, today: Date, earliest: Date) -> Option<When> {
    let names_a_day = side.day.is_some() || side.days_from_today.is_some() || side.weekday.is_some() && side.month.is_none();
    let when = if names_a_day {
        When::Day(day_of(side, today, earliest)?)
    } else if let Some(month) = side.month {
        let year = side.year.unwrap_or(if month >= earliest.month() { earliest.year() } else { earliest.year() + 1 });
        When::Month(year, month)
    } else {
        When::Year(side.year?)
    };
    match (when, side.time) {
        (When::Day(day), Some(time)) => Some(When::DayAt(day, time)),
        (_, Some(_)) => None,
        (when, None) => Some(when),
    }
}

fn end_of(mut end: WrittenSide, start: When, today: Date) -> Option<When> {
    let start_day = start.first_day()?;
    if let Some(lone) = end.lone_number.take() {
        if matches!(start, When::DayAt(..)) && end.time.is_none() && !end.has_date() && !end.has_day() {
            end.time = Some(time_of(lone, 0)?);
        } else {
            end.day = Some(small(lone, 1..=31)?);
        }
    }
    if !end.has_date() && !end.has_day() {
        let time = end.time?;
        return match start {
            When::DayAt(day, start_time) if time <= start_time => Some(When::DayAt(day.tomorrow().ok()?, time)),
            When::DayAt(day, _) | When::Day(day) => Some(When::DayAt(day, time)),
            _ => None,
        };
    }
    if end.day.is_some() && end.month.is_none() {
        end.take_month_of(start_day);
        if end.day < Some(start_day.day()) {
            end.move_to_month_after(start_day)?;
        }
    }
    when_of(&end, today, start_day)
}

pub fn partial_dates_of_written(written: &str, today: Date) -> Option<PartialDates> {
    let tokens = tokens_of(written)?;
    let mut sides = tokens.split(|token| *token == Token::Range);
    let mut start = side_of(sides.next()?)?;
    let end = match sides.next() {
        Some(tokens) => Some(side_of(tokens)?),
        None => None,
    };
    if sides.next().is_some() || end.as_ref().is_some_and(WrittenSide::is_empty) {
        return None;
    }
    if let Some(lone) = start.lone_number.take() {
        start.day = Some(small(lone, 1..=31)?);
    }
    if start.day.is_some() && start.month.is_none() {
        match end.as_ref().and_then(|end| end.month.map(|month| (month, end.year))) {
            Some((month, year)) => {
                start.month = Some(month);
                start.year = start.year.or(year);
            }
            None => {
                start.take_month_of(today);
                if start.day < Some(today.day()) {
                    start.move_to_month_after(today)?;
                }
            }
        }
    }
    let start_zone = start.zone.clone().or_else(|| end.as_ref()?.zone.clone()).unwrap_or(EVENT_TIME_ZONE.to_owned());
    let start_when = when_of(&start, today, today)?;
    let end = match end {
        Some(end) => {
            let zone = end.zone.clone().unwrap_or_else(|| start_zone.clone());
            Some(end_of(end, start_when, today)?.written(&zone))
        }
        None => None,
    };
    Some(PartialDates { start: start_when.written(&start_zone), end })
}

pub fn today_in_event_time_zone() -> Date {
    let now = Zoned::now();
    now.in_tz(EVENT_TIME_ZONE).map_or(now.date(), |now| now.date())
}

fn date_and_zone(partial_date: &str) -> Option<(&str, &str)> {
    partial_date
        .strip_suffix(']')
        .and_then(|inner| inner.split_once('['))
        .filter(|(date, _)| !date.is_empty())
}

pub fn written_of_partial_date(partial_date: &str) -> String {
    let Some((date, zone)) = date_and_zone(partial_date) else {
        return partial_date.to_owned();
    };
    let written = date.replacen('T', " ", 1);
    if zone == EVENT_TIME_ZONE {
        written
    } else {
        format!("{written} [{zone}]")
    }
}

fn end_time_alone(start: &str, end: &str) -> Option<String> {
    let (start_date, start_zone) = date_and_zone(start)?;
    let (end_date, end_zone) = date_and_zone(end)?;
    let (start_day, start_time) = start_date.split_once('T')?;
    let (end_day, end_time) = end_date.split_once('T')?;
    let next_day = start_day.parse::<Date>().ok()?.tomorrow().ok()?.to_string();
    let reads_back = (end_day == start_day && end_time > start_time) || (end_day == next_day && end_time <= start_time);
    (start_zone == end_zone && reads_back).then(|| end_time.to_owned())
}

pub fn written_of_partial_dates(start: &str, end: Option<&str>) -> String {
    let written_start = written_of_partial_date(start);
    match end {
        None => written_start,
        Some(end) => {
            let written_end = end_time_alone(start, end).unwrap_or_else(|| written_of_partial_date(end));
            format!("{written_start} - {written_end}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> Date {
        Date::new(2026, 11, 10).unwrap()
    }

    fn read(written: &str) -> Option<(String, Option<String>)> {
        partial_dates_of_written(written, today()).map(|dates| (dates.start, dates.end))
    }

    fn vilnius(date: &str) -> String {
        format!("{date}[Europe/Vilnius]")
    }

    #[test]
    fn loosely_written_dates_become_vilnius_partial_dates_at_the_precision_written() {
        let cases = [
            ("2026-12-05 18:00", "2026-12-05T18:00"),
            (" 2026-12-05T18:00 ", "2026-12-05T18:00"),
            ("2026-12-05", "2026-12-05"),
            ("2026-12", "2026-12"),
            ("2026", "2026"),
            ("12 october 15:00", "2027-10-12T15:00"),
            ("October 12th, 3pm", "2027-10-12T15:00"),
            ("2026 10 22", "2026-10-22"),
            ("22 10 2026", "2026-10-22"),
            ("22.10.2026", "2026-10-22"),
            ("2026.10.22 15.00", "2026-10-22T15:00"),
            ("05/12/2026 18:30", "2026-12-05T18:30"),
            ("5.12 18:00", "2026-12-05T18:00"),
            ("dec 5 6:30 pm", "2026-12-05T18:30"),
            ("5 Dec 2026, 6 p.m.", "2026-12-05T18:00"),
            ("december 2026", "2026-12"),
            ("march", "2027-03"),
            ("saturday 18:00", "2026-11-14T18:00"),
            ("tomorrow at 20:00", "2026-11-11T20:00"),
            ("the 20th", "2026-11-20"),
            ("gruodžio 5 d. 18 val.", "2026-12-05T18:00"),
            ("2026 m. gruodžio 5 d.", "2026-12-05"),
            ("šeštadienį 19:00", "2026-11-14T19:00"),
            ("5 Aralık 2026 Cumartesi 18.00", "2026-12-05T18:00"),
            ("Kasım'ın 14'ü saat 15:00", "2026-11-14T15:00"),
            ("cuma saat 20", "2026-11-13T20:00"),
        ];
        for (written, expected) in cases {
            assert_eq!(read(written), Some((vilnius(expected), None)), "{written}");
        }
        assert_eq!(
            read("2026-12-05 18:00 [Europe/London]"),
            Some(("2026-12-05T18:00[Europe/London]".to_owned(), None))
        );
    }

    #[test]
    fn a_range_fills_the_end_from_the_start_and_an_earlier_end_time_is_the_next_day() {
        let cases = [
            ("2026-12-05 18:00 - 2026-12-06 02:00", "2026-12-05T18:00", "2026-12-06T02:00"),
            ("5 Dec 18:00-22:00", "2026-12-05T18:00", "2026-12-05T22:00"),
            ("5 dec 22:00 to 02:00", "2026-12-05T22:00", "2026-12-06T02:00"),
            ("5 dec 18-22", "2026-12-05T18:00", "2026-12-05T22:00"),
            ("5-7 dec", "2026-12-05", "2026-12-07"),
            ("5 dec - 7", "2026-12-05", "2026-12-07"),
            ("28 dec – 2 jan", "2026-12-28", "2027-01-02"),
            ("30.11 - 2.12", "2026-11-30", "2026-12-02"),
            ("12 ekim 15.00 ila 18.00", "2027-10-12T15:00", "2027-10-12T18:00"),
            ("gruodžio 5 18:00 iki 23:00", "2026-12-05T18:00", "2026-12-05T23:00"),
        ];
        for (written, start, end) in cases {
            assert_eq!(read(written), Some((vilnius(start), Some(vilnius(end)))), "{written}");
        }
        assert_eq!(
            read("5 dec 18:00 - 22:00 [Europe/London]"),
            Some(("2026-12-05T18:00[Europe/London]".to_owned(), Some("2026-12-05T22:00[Europe/London]".to_owned())))
        );
    }

    #[test]
    fn unreadable_or_impossible_dates_are_refused() {
        let refused = [
            "", "18:00", "next week", "31.11", "2026-02-30", "5 dec - 7 dec - 9 dec", "5 dec -", "25:00 5 dec",
            "5 dec 18 19 20",
        ];
        for written in refused {
            assert_eq!(read(written), None, "{written}");
        }
    }

    #[test]
    fn partial_dates_are_written_back_without_the_event_zone_and_read_back_the_same() {
        let cases = [
            ("2026-12-05T18:00[Europe/Vilnius]", None, "2026-12-05 18:00"),
            ("2026-12-05T18:00[Europe/Vilnius]", Some("2026-12-05T22:00[Europe/Vilnius]"), "2026-12-05 18:00 - 22:00"),
            ("2026-12-05T22:00[Europe/Vilnius]", Some("2026-12-06T02:00[Europe/Vilnius]"), "2026-12-05 22:00 - 02:00"),
            (
                "2026-12-05T18:00[Europe/Vilnius]",
                Some("2026-12-06T20:00[Europe/Vilnius]"),
                "2026-12-05 18:00 - 2026-12-06 20:00",
            ),
            ("2026-12-05[Europe/Vilnius]", Some("2026-12-07[Europe/Vilnius]"), "2026-12-05 - 2026-12-07"),
            ("2026-12-05[Europe/London]", None, "2026-12-05 [Europe/London]"),
        ];
        for (start, end, written) in cases {
            assert_eq!(written_of_partial_dates(start, end), written);
            let read_back = partial_dates_of_written(written, today()).unwrap();
            assert_eq!((read_back.start.as_str(), read_back.end.as_deref()), (start, end), "{written}");
        }
        assert_eq!(written_of_partial_date("nonsense"), "nonsense");
    }
}
