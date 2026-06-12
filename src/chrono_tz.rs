use std::str::FromStr;
use crate::chrono::{FixedOffset, TimeZone, Offset};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Tz {
    Utc,
    Fixed(FixedOffset),
}

pub const UTC: Tz = Tz::Utc;

impl FromStr for Tz {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.eq_ignore_ascii_case("utc") || s.eq_ignore_ascii_case("gmt") || s.is_empty() {
            return Ok(Tz::Utc);
        }
        if let Some(offset) = parse_offset_string(s) {
            return Ok(Tz::Fixed(offset));
        }
        match s {
            "Asia/Jakarta" | "WIB" => Ok(Tz::Fixed(FixedOffset::east_opt(7 * 3600).unwrap())),
            "Asia/Singapore" | "Asia/Kuala_Lumpur" | "Asia/Makassar" | "WITA" => {
                Ok(Tz::Fixed(FixedOffset::east_opt(8 * 3600).unwrap()))
            }
            "Asia/Jayapura" | "WIT" => Ok(Tz::Fixed(FixedOffset::east_opt(9 * 3600).unwrap())),
            "Asia/Bangkok" => Ok(Tz::Fixed(FixedOffset::east_opt(7 * 3600).unwrap())),
            "Asia/Tokyo" | "Asia/Seoul" => Ok(Tz::Fixed(FixedOffset::east_opt(9 * 3600).unwrap())),
            "America/New_York" | "EST" => Ok(Tz::Fixed(FixedOffset::west_opt(5 * 3600).unwrap())),
            "America/Chicago" | "CST" => Ok(Tz::Fixed(FixedOffset::west_opt(6 * 3600).unwrap())),
            "America/Denver" | "MST" => Ok(Tz::Fixed(FixedOffset::west_opt(7 * 3600).unwrap())),
            "America/Los_Angeles" | "PST" => Ok(Tz::Fixed(FixedOffset::west_opt(8 * 3600).unwrap())),
            "Europe/London" | "GMT" => Ok(Tz::Fixed(FixedOffset::east_opt(0).unwrap())),
            _ => {
                Ok(Tz::Fixed(FixedOffset::east_opt(7 * 3600).unwrap())) // Default to Jakarta/WIB
            }
        }
    }
}

fn parse_offset_string(s: &str) -> Option<FixedOffset> {
    let s = s.trim();
    if s.is_empty() { return None; }
    let sign = match s.chars().next()? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    let rest = &s[1..];
    let parts: Vec<&str> = rest.split(':').collect();
    if parts.len() == 2 {
        let hours: i32 = parts[0].parse().ok()?;
        let minutes: i32 = parts[1].parse().ok()?;
        FixedOffset::east_opt(sign * (hours * 3600 + minutes * 60))
    } else if parts.len() == 1 {
        if rest.len() == 4 {
            let hours: i32 = rest[0..2].parse().ok()?;
            let minutes: i32 = rest[2..4].parse().ok()?;
            FixedOffset::east_opt(sign * (hours * 3600 + minutes * 60))
        } else if rest.len() == 2 || rest.len() == 1 {
            let hours: i32 = rest.parse().ok()?;
            FixedOffset::east_opt(sign * hours * 3600)
        } else {
            None
        }
    } else {
        None
    }
}

impl TimeZone for Tz {
    type Offset = TzOffset;

    fn from_offset(offset: &Self::Offset) -> Self {
        match offset {
            TzOffset::Utc => Tz::Utc,
            TzOffset::Fixed(fo) => Tz::Fixed(*fo),
        }
    }

    fn offset_from_local_date(&self, _local: &crate::chrono::NaiveDate) -> crate::chrono::LocalResult<Self::Offset> {
        match self {
            Tz::Utc => crate::chrono::LocalResult::Single(TzOffset::Utc),
            Tz::Fixed(fo) => crate::chrono::LocalResult::Single(TzOffset::Fixed(*fo)),
        }
    }

    fn offset_from_local_datetime(&self, _local: &crate::chrono::NaiveDateTime) -> crate::chrono::LocalResult<Self::Offset> {
        match self {
            Tz::Utc => crate::chrono::LocalResult::Single(TzOffset::Utc),
            Tz::Fixed(fo) => crate::chrono::LocalResult::Single(TzOffset::Fixed(*fo)),
        }
    }

    fn offset_from_utc_date(&self, _utc: &crate::chrono::NaiveDate) -> Self::Offset {
        match self {
            Tz::Utc => TzOffset::Utc,
            Tz::Fixed(fo) => TzOffset::Fixed(*fo),
        }
    }

    fn offset_from_utc_datetime(&self, _utc: &crate::chrono::NaiveDateTime) -> Self::Offset {
        match self {
            Tz::Utc => TzOffset::Utc,
            Tz::Fixed(fo) => TzOffset::Fixed(*fo),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TzOffset {
    Utc,
    Fixed(FixedOffset),
}

impl Offset for TzOffset {
    fn fix(&self) -> FixedOffset {
        match self {
            TzOffset::Utc => FixedOffset::east_opt(0).unwrap(),
            TzOffset::Fixed(fo) => *fo,
        }
    }
}

impl std::fmt::Display for TzOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fix())
    }
}
