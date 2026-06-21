use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

// ==========================================
// 1. Duration Implementation
// ==========================================
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration {
    pub secs: i64,
    pub nanos: i32,
}

impl Duration {
    pub fn zero() -> Self { Self { secs: 0, nanos: 0 } }
    pub fn seconds(secs: i64) -> Self { Self { secs, nanos: 0 } }
    pub fn minutes(mins: i64) -> Self { Self { secs: mins * 60, nanos: 0 } }
    pub fn hours(hours: i64) -> Self { Self { secs: hours * 3600, nanos: 0 } }
    pub fn days(days: i64) -> Self { Self { secs: days * 86400, nanos: 0 } }
    pub fn milliseconds(ms: i64) -> Self {
        Self {
            secs: ms / 1000,
            nanos: ((ms % 1000) * 1_000_000) as i32,
        }
    }
    pub fn microseconds(us: i64) -> Self {
        Self {
            secs: us / 1_000_000,
            nanos: ((us % 1_000_000) * 1000) as i32,
        }
    }
    pub fn nanoseconds(ns: i64) -> Self {
        Self {
            secs: ns / 1_000_000_000,
            nanos: (ns % 1_000_000_000) as i32,
        }
    }

    pub fn num_seconds(&self) -> i64 {
        self.secs
    }

    pub fn num_minutes(&self) -> i64 {
        self.secs / 60
    }

    pub fn num_hours(&self) -> i64 {
        self.secs / 3600
    }

    pub fn num_days(&self) -> i64 {
        self.secs / 86400
    }

    pub fn num_milliseconds(&self) -> i64 {
        self.secs * 1000 + (self.nanos / 1_000_000) as i64
    }
}

// ==========================================
// 2. Leap Year & Calendar Arithmetic
// ==========================================
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 0,
    }
}

// Howard Hinnant's O(1) Epoch Days Calendar Algorithm
pub fn epoch_days_to_date(epoch_days: i64) -> (i32, u32, u32) {
    let z = epoch_days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe/1460 + doe/36524 - doe/146096) / 365;
    let mut y = (yoe as i32) + era as i32 * 400;
    let doy = doe - (365*yoe + yoe/4 - yoe/100);
    let mp = (5*doy + 2)/153;
    let d = doy - (153*mp + 2)/5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    if m <= 2 {
        y += 1;
    }
    (y, m, d)
}

pub fn date_to_epoch_days(y: i32, m: u32, d: u32) -> i64 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = (y - era * 400) as u32;
    let m_adj = m as i32;
    let doy = (153 * (if m_adj > 2 { m_adj - 3 } else { m_adj + 9 }) + 2)/5 + (d as i32 - 1);
    let doe = yoe as i32 * 365 + yoe as i32 / 4 - yoe as i32 / 100 + doy;
    era as i64 * 146097 + doe as i64 - 719468
}

// ==========================================
// 3. NaiveDate, NaiveTime, NaiveDateTime
// ==========================================
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NaiveDate {
    y: i32,
    m: u32,
    d: u32,
}

impl NaiveDate {
    pub fn from_ymd_opt(y: i32, m: u32, d: u32) -> Option<Self> {
        if (1..=12).contains(&m) && d >= 1 && d <= days_in_month(y, m) {
            Some(Self { y, m, d })
        } else {
            None
        }
    }
    pub fn year(&self) -> i32 { self.y }
    pub fn month(&self) -> u32 { self.m }
    pub fn day(&self) -> u32 { self.d }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NaiveTime {
    hour: u32,
    min: u32,
    sec: u32,
    pub nano: u32,
}

impl NaiveTime {
    pub fn from_hms_opt(hour: u32, min: u32, sec: u32) -> Option<Self> {
        Self::from_hms_nano_opt(hour, min, sec, 0)
    }

    pub fn from_hms_nano_opt(hour: u32, min: u32, sec: u32, nano: u32) -> Option<Self> {
        if hour < 24 && min < 60 && sec < 60 && nano < 1_000_000_000 {
            Some(Self { hour, min, sec, nano })
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NaiveDateTime {
    pub date: NaiveDate,
    pub time: NaiveTime,
}

impl NaiveDateTime {
    pub fn new(date: NaiveDate, time: NaiveTime) -> Self {
        Self { date, time }
    }

    pub fn from_timestamp_opt(secs: i64, nsecs: u32) -> Option<Self> {
        if nsecs >= 1_000_000_000 { return None; }
        let days = if secs >= 0 { secs / 86400 } else { (secs - 86399) / 86400 };
        let mut rem_secs = (secs - days * 86400) as u32;
        let hour = rem_secs / 3600;
        rem_secs %= 3600;
        let min = rem_secs / 60;
        let sec = rem_secs % 60;
        let (y, m, d) = epoch_days_to_date(days);
        Some(Self {
            date: NaiveDate { y, m, d },
            time: NaiveTime { hour, min, sec, nano: nsecs },
        })
    }

    pub fn timestamp(&self) -> i64 {
        let days = date_to_epoch_days(self.date.y, self.date.m, self.date.d);
        days * 86400 + (self.time.hour as i64 * 3600) + (self.time.min as i64 * 60) + (self.time.sec as i64)
    }

    pub fn date(&self) -> NaiveDate { self.date }
    pub fn time(&self) -> NaiveTime { self.time }

    pub fn format<'a>(&'a self, fmt_str: &'a str) -> Format<'a> {
        Format {
            dt: self,
            offset_secs: 0,
            fmt_str,
        }
    }

    pub fn signed_duration_since(&self, other: NaiveDateTime) -> Duration {
        let diff_secs = self.timestamp() - other.timestamp();
        let diff_nanos = self.time.nano as i64 - other.time.nano as i64;
        let (final_secs, final_nanos) = if diff_nanos < 0 {
            (diff_secs - 1, diff_nanos + 1_000_000_000)
        } else {
            (diff_secs, diff_nanos)
        };
        Duration {
            secs: final_secs,
            nanos: final_nanos as i32,
        }
    }
}

impl std::fmt::Display for NaiveDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.y, self.m, self.d)
    }
}

impl std::fmt::Display for NaiveTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02}:{:02}:{:02}", self.hour, self.min, self.sec)?;
        if self.nano > 0 {
            write!(f, ".{:09}", self.nano)?;
        }
        Ok(())
    }
}

impl std::fmt::Display for NaiveDateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.date, self.time)
    }
}

// Format string parsing helper
pub struct Format<'a> {
    dt: &'a NaiveDateTime,
    offset_secs: i32,
    fmt_str: &'a str,
}

impl<'a> std::fmt::Display for Format<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut chars = self.fmt_str.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '%' {
                match chars.next() {
                    Some('Y') => write!(f, "{:04}", self.dt.date.y)?,
                    Some('m') => write!(f, "{:02}", self.dt.date.m)?,
                    Some('d') => write!(f, "{:02}", self.dt.date.d)?,
                    Some('H') => write!(f, "{:02}", self.dt.time.hour)?,
                    Some('M') => write!(f, "{:02}", self.dt.time.min)?,
                    Some('S') => write!(f, "{:02}", self.dt.time.sec)?,
                    Some('f') => write!(f, "{:09}", self.dt.time.nano)?,
                    Some('.') => {
                        if chars.peek() == Some(&'3') {
                            chars.next();
                            if chars.peek() == Some(&'f') {
                                chars.next();
                                write!(f, ".{:03}", self.dt.time.nano / 1_000_000)?;
                            } else {
                                write!(f, "%.3")?;
                            }
                        } else if chars.peek() == Some(&'6') {
                            chars.next();
                            if chars.peek() == Some(&'f') {
                                chars.next();
                                write!(f, ".{:06}", self.dt.time.nano / 1_000)?;
                            } else {
                                write!(f, "%.6")?;
                            }
                        } else {
                            write!(f, "%.")?;
                        }
                    }
                    Some('z') => {
                        let sign = if self.offset_secs >= 0 { '+' } else { '-' };
                        let abs_offset = self.offset_secs.abs();
                        let hours = abs_offset / 3600;
                        let mins = (abs_offset % 3600) / 60;
                        write!(f, "{}{:02}{:02}", sign, hours, mins)?;
                    }
                    Some('%') => write!(f, "%")?,
                    Some(other) => write!(f, "%{}", other)?,
                    None => write!(f, "%")?,
                }
            } else {
                write!(f, "{}", c)?;
            }
        }
        Ok(())
    }
}

// Parsing function helper
pub fn parse_naive_datetime(s: &str) -> Result<NaiveDateTime, String> {
    let s = s.trim();
    if s.len() < 19 {
        return Err(format!("Datetime string too short: {}", s));
    }
    let year: i32 = s[0..4].parse().map_err(|_| "Invalid year")?;
    let month: u32 = s[5..7].parse().map_err(|_| "Invalid month")?;
    let day: u32 = s[8..10].parse().map_err(|_| "Invalid day")?;
    let hour: u32 = s[11..13].parse().map_err(|_| "Invalid hour")?;
    let min: u32 = s[14..16].parse().map_err(|_| "Invalid minute")?;
    let sec: u32 = s[17..19].parse().map_err(|_| "Invalid second")?;
    
    let mut nano = 0;
    if s.len() > 19 && (s.as_bytes()[19] == b'.' || s.as_bytes()[19] == b',') {
        let rest = &s[20..];
        let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
        let digit_str = &rest[..end];
        if !digit_str.is_empty() {
            let val: u32 = digit_str.parse().map_err(|_| "Invalid subsecond")?;
            let len = digit_str.len() as u32;
            let multiplier = match len {
                1 => 100_000_000,
                2 => 10_000_000,
                3 => 1_000_000,
                4 => 100_000,
                5 => 10_000,
                6 => 1_000,
                7 => 100,
                8 => 10,
                9 => 1,
                _ => 0,
            };
            if len <= 9 {
                nano = val * multiplier;
            } else {
                nano = digit_str[..9].parse::<u32>().unwrap();
            }
        }
    }
    
    let date = NaiveDate { y: year, m: month, d: day };
    let time = NaiveTime::from_hms_nano_opt(hour, min, sec, nano).ok_or("Invalid time components")?;
    Ok(NaiveDateTime::new(date, time))
}

// ==========================================
// 4. Timezone traits & FixedOffset, Utc, Local
// ==========================================
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalResult<T> {
    None,
    Single(T),
    Ambiguous(T, T),
}

impl<T> LocalResult<T> {
    pub fn single(self) -> Option<T> {
        match self {
            LocalResult::Single(t) => Some(t),
            _ => None,
        }
    }
    pub fn unwrap(self) -> T {
        match self {
            LocalResult::Single(t) => t,
            _ => panic!("LocalResult::unwrap failed"),
        }
    }
}

pub trait Offset: Copy + Clone + std::fmt::Display {
    fn fix(&self) -> FixedOffset;
}

pub trait TimeZone: Sized + Copy + Clone {
    type Offset: Offset;
    fn from_offset(offset: &Self::Offset) -> Self;
    fn offset_from_local_date(&self, local: &NaiveDate) -> LocalResult<Self::Offset>;
    fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> LocalResult<Self::Offset>;
    fn offset_from_utc_date(&self, utc: &NaiveDate) -> Self::Offset;
    fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> Self::Offset;

    // NOTE: `from_utc_datetime` takes `&self` intentionally to match the chrono crate's
    // trait interface. Clippy's `wrong_self_convention` warning is suppressed here.
    #[allow(clippy::wrong_self_convention)]
    fn from_utc_datetime(&self, utc: &NaiveDateTime) -> DateTime<Self> {
        let offset = self.offset_from_utc_datetime(utc);
        let fixed = offset.fix();
        let local_secs = utc.timestamp() + fixed.local_minus_utc() as i64;
        let naive = NaiveDateTime::from_timestamp_opt(local_secs, utc.time.nano).unwrap();
        DateTime { naive, offset, tz: *self }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedOffset {
    local_minus_utc: i32,
}

impl FixedOffset {
    pub fn east_opt(secs: i32) -> Option<Self> {
        if secs.abs() <= 86400 {
            Some(Self { local_minus_utc: secs })
        } else {
            None
        }
    }
    pub fn west_opt(secs: i32) -> Option<Self> {
        if secs.abs() <= 86400 {
            Some(Self { local_minus_utc: -secs })
        } else {
            None
        }
    }
    pub fn local_minus_utc(&self) -> i32 {
        self.local_minus_utc
    }
}

impl Offset for FixedOffset {
    fn fix(&self) -> FixedOffset {
        *self
    }
}

impl std::fmt::Display for FixedOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sign = if self.local_minus_utc >= 0 { '+' } else { '-' };
        let abs_offset = self.local_minus_utc.abs();
        let hours = abs_offset / 3600;
        let mins = (abs_offset % 3600) / 60;
        write!(f, "{}{:02}:{:02}", sign, hours, mins)
    }
}

impl TimeZone for FixedOffset {
    type Offset = FixedOffset;

    fn from_offset(offset: &Self::Offset) -> Self {
        *offset
    }

    fn offset_from_local_date(&self, _local: &NaiveDate) -> LocalResult<Self::Offset> {
        LocalResult::Single(*self)
    }

    fn offset_from_local_datetime(&self, _local: &NaiveDateTime) -> LocalResult<Self::Offset> {
        LocalResult::Single(*self)
    }

    fn offset_from_utc_date(&self, _utc: &NaiveDate) -> Self::Offset {
        *self
    }

    fn offset_from_utc_datetime(&self, _utc: &NaiveDateTime) -> Self::Offset {
        *self
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Utc;

impl TimeZone for Utc {
    type Offset = FixedOffset;

    fn from_offset(_offset: &Self::Offset) -> Self {
        Utc
    }

    fn offset_from_local_date(&self, _local: &NaiveDate) -> LocalResult<Self::Offset> {
        LocalResult::Single(FixedOffset::east_opt(0).unwrap())
    }

    fn offset_from_local_datetime(&self, _local: &NaiveDateTime) -> LocalResult<Self::Offset> {
        LocalResult::Single(FixedOffset::east_opt(0).unwrap())
    }

    fn offset_from_utc_date(&self, _utc: &NaiveDate) -> Self::Offset {
        FixedOffset::east_opt(0).unwrap()
    }

    fn offset_from_utc_datetime(&self, _utc: &NaiveDateTime) -> Self::Offset {
        FixedOffset::east_opt(0).unwrap()
    }
}

impl Utc {
    pub fn now() -> DateTime<Utc> {
        let now_system = std::time::SystemTime::now();
        let duration = now_system.duration_since(std::time::UNIX_EPOCH).unwrap();
        let naive = NaiveDateTime::from_timestamp_opt(duration.as_secs() as i64, duration.subsec_nanos()).unwrap();
        DateTime {
            naive,
            offset: FixedOffset::east_opt(0).unwrap(),
            tz: Utc,
        }
    }
}

#[repr(C)]
struct tm {
    tm_sec: i32,
    tm_min: i32,
    tm_hour: i32,
    tm_mday: i32,
    tm_mon: i32,
    tm_year: i32,
    tm_wday: i32,
    tm_yday: i32,
    tm_isdst: i32,
    tm_gmtoff: i64,
    tm_zone: *const std::os::raw::c_char,
}

unsafe extern "C" {
    fn localtime_r(timep: *const i64, result: *mut tm) -> *mut tm;
}

fn get_local_offset_secs(secs: i64) -> i32 {
    unsafe {
        let mut t = tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 0,
            tm_mon: 0,
            tm_year: 0,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
            tm_gmtoff: 0,
            tm_zone: std::ptr::null(),
        };
        localtime_r(&secs, &mut t);
        t.tm_gmtoff as i32
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Local;

impl TimeZone for Local {
    type Offset = FixedOffset;

    fn from_offset(_offset: &Self::Offset) -> Self {
        Local
    }

    fn offset_from_local_date(&self, local: &NaiveDate) -> LocalResult<Self::Offset> {
        let naive = NaiveDateTime::new(*local, NaiveTime { hour: 0, min: 0, sec: 0, nano: 0 });
        self.offset_from_local_datetime(&naive)
    }

    fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> LocalResult<Self::Offset> {
        let utc_approx = local.timestamp();
        let offset = get_local_offset_secs(utc_approx);
        LocalResult::Single(FixedOffset::east_opt(offset).unwrap())
    }

    fn offset_from_utc_date(&self, utc: &NaiveDate) -> Self::Offset {
        let naive = NaiveDateTime::new(*utc, NaiveTime { hour: 0, min: 0, sec: 0, nano: 0 });
        self.offset_from_utc_datetime(&naive)
    }

    fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> Self::Offset {
        let offset = get_local_offset_secs(utc.timestamp());
        FixedOffset::east_opt(offset).unwrap()
    }
}

impl Local {
    pub fn now() -> DateTime<Local> {
        let now_system = std::time::SystemTime::now();
        let duration = now_system.duration_since(std::time::UNIX_EPOCH).unwrap();
        let secs = duration.as_secs() as i64;
        let offset_secs = get_local_offset_secs(secs);
        let naive = NaiveDateTime::from_timestamp_opt(secs + offset_secs as i64, duration.subsec_nanos()).unwrap();
        DateTime {
            naive,
            offset: FixedOffset::east_opt(offset_secs).unwrap(),
            tz: Local,
        }
    }
}

// ==========================================
// 5. DateTime Tz-Aware Struct
// ==========================================
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime<Tz: TimeZone> {
    pub naive: NaiveDateTime,
    pub offset: Tz::Offset,
    pub tz: Tz,
}

impl<Tz: TimeZone> DateTime<Tz> {
    pub fn naive_utc(&self) -> NaiveDateTime {
        let offset = self.offset.fix();
        let utc_secs = self.naive.timestamp() - offset.local_minus_utc() as i64;
        NaiveDateTime::from_timestamp_opt(utc_secs, self.naive.time.nano).unwrap()
    }

    pub fn timestamp(&self) -> i64 {
        let offset = self.offset.fix();
        self.naive.timestamp() - offset.local_minus_utc() as i64
    }

    pub fn naive_local(&self) -> NaiveDateTime {
        self.naive
    }

    pub fn with_timezone<Tz2: TimeZone>(&self, tz2: &Tz2) -> DateTime<Tz2> {
        let utc_dt = self.naive_utc();
        tz2.from_utc_datetime(&utc_dt)
    }

    pub fn format<'a>(&'a self, fmt_str: &'a str) -> Format<'a> {
        Format {
            dt: &self.naive,
            offset_secs: self.offset.fix().local_minus_utc(),
            fmt_str,
        }
    }
}

pub fn parse_offset_secs(offset_str: &str) -> Option<i32> {
    let offset_str = offset_str.trim();
    if offset_str == "Z" || offset_str.eq_ignore_ascii_case("utc") || offset_str.is_empty() {
        return Some(0);
    }
    let sign = match offset_str.chars().next()? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    let rest = &offset_str[1..];
    let parts: Vec<&str> = rest.split(':').collect();
    if parts.len() == 2 {
        let hours: i32 = parts[0].parse().ok()?;
        let minutes: i32 = parts[1].parse().ok()?;
        Some(sign * (hours * 3600 + minutes * 60))
    } else if parts.len() == 1 {
        if rest.len() == 4 {
            let hours: i32 = rest[0..2].parse().ok()?;
            let minutes: i32 = rest[2..4].parse().ok()?;
            Some(sign * (hours * 3600 + minutes * 60))
        } else if rest.len() == 2 || rest.len() == 1 {
            let hours: i32 = rest.parse().ok()?;
            Some(sign * hours * 3600)
        } else {
            None
        }
    } else {
        None
    }
}

impl DateTime<FixedOffset> {
    pub fn parse_from_rfc3339(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.len() < 19 {
            return Err("Datetime string too short".to_string());
        }
        let tz_idx = s[19..]
            .find(['Z', '+', '-'])
            .map(|idx| idx + 19)
            .ok_or_else(|| "Timezone offset missing".to_string())?;
        let naive_str = &s[..tz_idx];
        let offset_str = &s[tz_idx..];
        let naive = parse_naive_datetime(naive_str)?;
        let offset_secs = parse_offset_secs(offset_str).ok_or("Invalid offset offset format")?;
        let utc_secs = naive.timestamp() - offset_secs as i64;
        let utc_naive = NaiveDateTime::from_timestamp_opt(utc_secs, naive.time.nano).ok_or("Invalid timestamp")?;
        let tz = FixedOffset::east_opt(offset_secs).ok_or("Invalid offset seconds")?;
        Ok(tz.from_utc_datetime(&utc_naive))
    }
}

impl<Tz: TimeZone> DateTime<Tz> {
    pub fn signed_duration_since<Tz2: TimeZone>(&self, other: DateTime<Tz2>) -> Duration {
        let self_utc = self.timestamp();
        let other_utc = other.timestamp();
        let diff_secs = self_utc - other_utc;
        let diff_nanos = self.naive.time.nano as i64 - other.naive.time.nano as i64;
        let (final_secs, final_nanos) = if diff_nanos < 0 {
            (diff_secs - 1, diff_nanos + 1_000_000_000)
        } else {
            (diff_secs, diff_nanos)
        };
        Duration {
            secs: final_secs,
            nanos: final_nanos as i32,
        }
    }
}

// ==========================================
// 6. Arithmetic Operators
// ==========================================
impl<Tz: TimeZone> Add<Duration> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    fn add(self, rhs: Duration) -> Self::Output {
        let utc = self.naive_utc();
        let new_secs = utc.timestamp() + rhs.secs;
        let new_nanos = (utc.time.nano as i64 + rhs.nanos as i64) as u32;
        let final_secs = new_secs + (new_nanos / 1_000_000_000) as i64;
        let final_nanos = new_nanos % 1_000_000_000;
        let new_utc = NaiveDateTime::from_timestamp_opt(final_secs, final_nanos).unwrap();
        self.tz.from_utc_datetime(&new_utc)
    }
}

impl<Tz: TimeZone> Sub<Duration> for DateTime<Tz> {
    type Output = DateTime<Tz>;
    fn sub(self, rhs: Duration) -> Self::Output {
        let utc = self.naive_utc();
        let new_secs = utc.timestamp() - rhs.secs;
        let mut new_nanos = utc.time.nano as i64 - rhs.nanos as i64;
        let final_secs = if new_nanos < 0 {
            new_nanos += 1_000_000_000;
            new_secs - 1
        } else {
            new_secs
        };
        let new_utc = NaiveDateTime::from_timestamp_opt(final_secs, new_nanos as u32).unwrap();
        self.tz.from_utc_datetime(&new_utc)
    }
}

impl Add<Duration> for NaiveDateTime {
    type Output = NaiveDateTime;
    fn add(self, rhs: Duration) -> Self::Output {
        let new_secs = self.timestamp() + rhs.secs;
        let new_nanos = (self.time.nano as i64 + rhs.nanos as i64) as u32;
        let final_secs = new_secs + (new_nanos / 1_000_000_000) as i64;
        let final_nanos = new_nanos % 1_000_000_000;
        NaiveDateTime::from_timestamp_opt(final_secs, final_nanos).unwrap()
    }
}

impl Sub<Duration> for NaiveDateTime {
    type Output = NaiveDateTime;
    fn sub(self, rhs: Duration) -> Self::Output {
        let new_secs = self.timestamp() - rhs.secs;
        let mut new_nanos = self.time.nano as i64 - rhs.nanos as i64;
        let final_secs = if new_nanos < 0 {
            new_nanos += 1_000_000_000;
            new_secs - 1
        } else {
            new_secs
        };
        NaiveDateTime::from_timestamp_opt(final_secs, new_nanos as u32).unwrap()
    }
}

impl Sub<NaiveDateTime> for NaiveDateTime {
    type Output = Duration;
    fn sub(self, rhs: NaiveDateTime) -> Self::Output {
        self.signed_duration_since(rhs)
    }
}

// ==========================================
// 7. Serde Serialization & Deserialization
// ==========================================
impl Serialize for NaiveDateTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.format("%Y-%m-%dT%H:%M:%S").to_string())
    }
}

impl<'de> Deserialize<'de> for NaiveDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NaiveDateTimeVisitor;
        impl<'de> serde::de::Visitor<'de> for NaiveDateTimeVisitor {
            type Value = NaiveDateTime;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a datetime string in ISO 8601 / RFC 3339 format")
            }
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                parse_naive_datetime(value).map_err(|e| E::custom(e))
            }
        }
        deserializer.deserialize_str(NaiveDateTimeVisitor)
    }
}

impl Serialize for DateTime<FixedOffset> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string())
    }
}

impl<'de> Deserialize<'de> for DateTime<FixedOffset> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FixedDateTimeVisitor;
        impl<'de> serde::de::Visitor<'de> for FixedDateTimeVisitor {
            type Value = DateTime<FixedOffset>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a datetime string with timezone")
            }
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let value = value.trim();
                if value.len() < 19 {
                    return Err(E::custom("Datetime string too short"));
                }
                let tz_idx = value[19..]
                    .find(['Z', '+', '-'])
                    .map(|idx| idx + 19)
                    .ok_or_else(|| E::custom("Timezone offset missing"))?;
                let naive_str = &value[..tz_idx];
                let offset_str = &value[tz_idx..];
                let naive = parse_naive_datetime(naive_str).map_err(|e| E::custom(e))?;
                let offset_secs = parse_offset_secs(offset_str).ok_or_else(|| E::custom("Invalid offset seconds"))?;
                let utc_secs = naive.timestamp() - offset_secs as i64;
                let utc_naive = NaiveDateTime::from_timestamp_opt(utc_secs, naive.time.nano).unwrap();
                let tz = FixedOffset::east_opt(offset_secs).ok_or_else(|| E::custom("Invalid offset seconds"))?;
                Ok(tz.from_utc_datetime(&utc_naive))
            }
        }
        deserializer.deserialize_str(FixedDateTimeVisitor)
    }
}

impl Serialize for DateTime<Utc> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string())
    }
}

impl<'de> Deserialize<'de> for DateTime<Utc> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let dt_fixed = DateTime::<FixedOffset>::deserialize(deserializer)?;
        Ok(dt_fixed.with_timezone(&Utc))
    }
}

impl Serialize for DateTime<Local> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string())
    }
}

impl<'de> Deserialize<'de> for DateTime<Local> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let dt_fixed = DateTime::<FixedOffset>::deserialize(deserializer)?;
        Ok(dt_fixed.with_timezone(&Local))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration() {
        let d1 = Duration::seconds(10);
        let d2 = Duration::seconds(5);
        assert!(d1 > d2);
        assert_eq!(d1.num_seconds(), 10);
        assert_eq!(d1.num_milliseconds(), 10000);
        assert_eq!(Duration::zero().num_seconds(), 0);
    }

    #[test]
    fn test_leap_year() {
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2004));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2023));
        assert!(is_leap_year(2024));
    }

    #[test]
    fn test_epoch_days() {
        // Test Howard Hinnant's algorithm consistency
        for days in -10000..10000 {
            let (y, m, d) = epoch_days_to_date(days);
            let recon = date_to_epoch_days(y, m, d);
            assert_eq!(recon, days, "Failed recon at days={}", days);
        }
    }

    #[test]
    fn test_naive_date_time_parsing() {
        let s = "2026-06-11T20:07:53.123456789";
        let dt = parse_naive_datetime(s).unwrap();
        assert_eq!(dt.date.year(), 2026);
        assert_eq!(dt.date.month(), 6);
        assert_eq!(dt.date.day(), 11);
        assert_eq!(dt.time.hour, 20);
        assert_eq!(dt.time.min, 7);
        assert_eq!(dt.time.sec, 53);
        assert_eq!(dt.time.nano, 123456789);

        let s2 = "2026-06-11T20:07:53";
        let dt2 = parse_naive_datetime(s2).unwrap();
        assert_eq!(dt2.time.nano, 0);
    }

    #[test]
    fn test_datetime_serialization() {
        let s = "2026-06-11T20:07:53.123+07:00";
        let dt = DateTime::<FixedOffset>::parse_from_rfc3339(s).unwrap();
        let serialized = serde_json::to_string(&dt).unwrap();
        assert!(serialized.contains("2026-06-11T20:07:53.123+0700") || serialized.contains("2026-06-11T20:07:53.123+07:00"));

        let deserialized: DateTime<FixedOffset> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.timestamp(), dt.timestamp());
    }

    #[test]
    fn test_datetime_arithmetic() {
        let dt = Utc::now();
        let added = dt + Duration::days(2);
        let subtracted = added - Duration::days(2);
        assert_eq!(dt.timestamp(), subtracted.timestamp());
    }
}
