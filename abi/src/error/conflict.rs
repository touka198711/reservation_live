use std::{collections::HashMap, convert::Infallible, str::FromStr};

use chrono::{DateTime, Utc};
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReservationConflictInfo {
    Parsed(ReservationConflict),
    Unparsed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationConflict {
    pub new: ReservationWindow,
    pub old: ReservationWindow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservationWindow {
    pub rid: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl FromStr for ReservationConflictInfo {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(conflict) = s.parse() {
            Ok(ReservationConflictInfo::Parsed(conflict))
        } else {
            Ok(ReservationConflictInfo::Unparsed(s.to_string()))
        }
    }
}

impl FromStr for ReservationConflict {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ParseInfo::from_str(s)?.try_into()
    }
}

impl TryFrom<ParseInfo> for ReservationConflict {
    type Error = ();

    fn try_from(value: ParseInfo) -> Result<Self, Self::Error> {
        Ok(Self {
            new: value.new.try_into()?,
            old: value.old.try_into()?,
        })
    }
}

impl TryFrom<HashMap<String, String>> for ReservationWindow {
    type Error = ();

    fn try_from(value: HashMap<String, String>) -> Result<Self, Self::Error> {
        let timespan_str = value.get("timespan").ok_or(())?.replace('"', "");

        let mut split = timespan_str.splitn(2, ',');
        let start = parse_date(split.next().ok_or(())?)?;
        let end = parse_date(split.next().ok_or(())?)?;
        Ok(Self {
            rid: value.get("resource_id").unwrap().to_owned(),
            start: start,
            end: end,
        })
    }
}

struct ParseInfo {
    new: HashMap<String, String>,
    old: HashMap<String, String>,
}

impl FromStr for ParseInfo {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r#"\((?P<k1>[a-zA-Z0-9_-]+)\s*,\s*(?P<k2>[a-zA-Z0-9_-]+)\)=\((?P<v1>[a-zA-Z0-9_-]+)\s*,\s*\[(?P<v2>[^\)\]]+)"#)
            .unwrap();
        let mut maps = vec![];
        for cap in re.captures_iter(s) {
            let mut map = HashMap::new();
            map.insert(cap["k1"].to_string(), cap["v1"].to_string());
            map.insert(cap["k2"].to_string(), cap["v2"].to_string());
            maps.push(Some(map));
        }
        if maps.len() != 2 {
            return Err(());
        }
        Ok(ParseInfo {
            new: maps[0].take().unwrap(),
            old: maps[1].take().unwrap(),
        })
    }
}

fn parse_date(s: &str) -> Result<DateTime<Utc>, ()> {
    Ok(DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%#z")
        .map_err(|_| ())?
        .with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ERR_MSG: &str = "Key (resource_id, timespan)=(ocean-view-room-713, [\"2022-12-26 22:00:00+00\",\"2022-12-30 19:00:00+00\")) conflicts with existing key (resource_id, timespan)=(ocean-view-room-713, [\"2022-12-25 22:00:00+00\",\"2022-12-28 19:00:00+00\")).";

    #[test]
    fn convert_parse_into_should_work() {
        let p = ParseInfo::from_str(ERR_MSG).unwrap();
        let new = p.new.get("resource_id").unwrap();
        let old = p.old.get("resource_id").unwrap();
        assert_eq!(new, "ocean-view-room-713");
        assert_eq!(old, "ocean-view-room-713");
    }

    #[test]
    fn parse_date_should_work() {
        let s = "2022-12-26 22:00:00+00";
        let s = parse_date(s).unwrap();
        assert_eq!(s.to_rfc3339(), "2022-12-26T22:00:00+00:00");
        let s = parse_date("2022-12-26 15:00:00-0700").unwrap();
        assert_eq!(s.to_rfc3339(), "2022-12-26T22:00:00+00:00");
        let s: DateTime<Utc> = "2022-12-26T15:00:00-0700".parse().unwrap();
        assert_eq!(s.to_rfc3339(), "2022-12-26T22:00:00+00:00");
    }

    #[test]
    fn conflict_error_message_should_parse() {
        let info: ReservationConflictInfo = ERR_MSG.parse().unwrap();
        match info {
            ReservationConflictInfo::Parsed(conflict) => {
                assert_eq!(conflict.new.rid, "ocean-view-room-713");
                assert_eq!(conflict.new.start.to_rfc3339(), "2022-12-26T22:00:00+00:00");
                assert_eq!(conflict.new.end.to_rfc3339(), "2022-12-30T19:00:00+00:00");
                assert_eq!(conflict.old.rid, "ocean-view-room-713");
                assert_eq!(conflict.old.start.to_rfc3339(), "2022-12-25T22:00:00+00:00");
                assert_eq!(conflict.old.end.to_rfc3339(), "2022-12-28T19:00:00+00:00");
            }
            ReservationConflictInfo::Unparsed(_) => panic!("should be parsed"),
        }
    }
}
