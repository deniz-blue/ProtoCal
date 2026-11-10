use std::sync::LazyLock;

use jiff::{Timestamp, tz::TimeZone};
use regex::Regex;

const MILLISECONDS_PER_DAY: i64 = 86_400_000;

static PARTIAL_DATE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(\d{4})(?:-(\d{2})(?:-(\d{2})(?:T(\d{2}):(\d{2}))?)?)?\[([A-Za-z][A-Za-z0-9_/+-]*)\]$",
    )
    .expect("partial date pattern")
});

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precision {
    Year,
    Month,
    Day,
    Time,
}

#[derive(Clone, Debug)]
pub struct PartialDate {
    pub year: i32,
    pub month: Option<i8>,
    pub day: Option<i8>,
    pub hour_minute: Option<(i8, i8)>,
    pub time_zone_id: String,
    time_zone: TimeZone,
}

impl PartialDate {
    pub fn parse(text: &str) -> Option<Self> {
        let groups = PARTIAL_DATE_PATTERN.captures(text)?;
        let number = |index: usize| groups.get(index).map(|digits| digits.as_str().parse::<i8>());
        let year: i32 = groups[1].parse().ok()?;
        let month = number(2).transpose().ok()?;
        let day = number(3).transpose().ok()?;
        let hour = number(4).transpose().ok()?;
        let minute = number(5).transpose().ok()?;
        let time_zone_id = groups[6].to_owned();
        if month.is_some_and(|month| !(1..=12).contains(&month))
            || day.is_some_and(|day| day < 1 || day > days_in_month(year, month.unwrap_or(1)))
            || hour.is_some_and(|hour| !(0..=23).contains(&hour))
            || minute.is_some_and(|minute| !(0..=59).contains(&minute))
        {
            return None;
        }
        let time_zone = TimeZone::get(&time_zone_id).ok()?;
        Some(Self {
            year,
            month,
            day,
            hour_minute: hour.zip(minute),
            time_zone_id,
            time_zone,
        })
    }

    pub fn is_valid(text: &str) -> bool {
        Self::parse(text).is_some()
    }

    pub fn precision(&self) -> Precision {
        match (self.month, self.day, self.hour_minute) {
            (_, _, Some(_)) => Precision::Time,
            (_, Some(_), None) => Precision::Day,
            (Some(_), None, None) => Precision::Month,
            (None, None, None) => Precision::Year,
        }
    }

    pub fn has(&self, precision: Precision) -> bool {
        self.precision() >= precision
    }

    pub fn start_epoch_milliseconds(&self) -> i64 {
        let (hour, minute) = self.hour_minute.unwrap_or((0, 0));
        let wall = wall_clock_as_epoch_milliseconds(
            self.year,
            self.month.unwrap_or(1).into(),
            self.day.unwrap_or(1).into(),
            hour.into(),
            minute.into(),
        );
        instant_of_wall_clock(wall, &self.time_zone)
    }

    pub fn end_epoch_milliseconds(&self) -> i64 {
        let precision = self.precision();
        if precision == Precision::Time {
            return self.start_epoch_milliseconds();
        }
        let month: i64 = self.month.unwrap_or(1).into();
        let day: i64 = self.day.unwrap_or(1).into();
        let wall = match precision {
            Precision::Year => wall_clock_as_epoch_milliseconds(self.year + 1, month, day, 0, 0),
            Precision::Month => wall_clock_as_epoch_milliseconds(self.year, month + 1, day, 0, 0),
            _ => wall_clock_as_epoch_milliseconds(self.year, month, day + 1, 0, 0),
        };
        instant_of_wall_clock(wall, &self.time_zone)
    }

    pub fn is_after(&self, other: &PartialDate) -> bool {
        let start = self.start_epoch_milliseconds();
        start >= other.end_epoch_milliseconds() && start != other.start_epoch_milliseconds()
    }
}

pub fn days_in_month(year: i32, month: i8) -> i8 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn wall_clock_as_epoch_milliseconds(year: i32, month: i64, day: i64, hour: i64, minute: i64) -> i64 {
    let months = i64::from(year) * 12 + month - 1;
    let first_of_month = days_from_civil(months.div_euclid(12), months.rem_euclid(12) + 1, 1);
    ((first_of_month + day - 1) * 24 * 60 + hour * 60 + minute) * 60_000
}

fn offset_at(epoch_milliseconds: i64, time_zone: &TimeZone) -> i64 {
    let seconds = epoch_milliseconds
        .div_euclid(1000)
        .clamp(Timestamp::MIN.as_second(), Timestamp::MAX.as_second());
    let timestamp = Timestamp::from_second(seconds).unwrap_or(Timestamp::UNIX_EPOCH);
    i64::from(time_zone.to_offset(timestamp).seconds()) * 1000
}

fn instant_of_wall_clock(wall: i64, time_zone: &TimeZone) -> i64 {
    let offset_before = offset_at(wall - MILLISECONDS_PER_DAY, time_zone);
    let offset_after = offset_at(wall + MILLISECONDS_PER_DAY, time_zone);
    [wall - offset_before, wall - offset_after]
        .into_iter()
        .filter(|&instant| instant + offset_at(instant, time_zone) == wall)
        .min()
        .unwrap_or(wall - offset_before)
}
