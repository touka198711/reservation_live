use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Timelike, Utc};
use prost_types::Timestamp;

use crate::Error;

pub fn to_datetime(tm: Option<&prost_types::Timestamp>) -> Result<DateTime<Utc>, Error> {
    tm
        // .map(|x| FixedOffset::east(0).timestamp(x.seconds, 0))
        .map(|x| Utc.timestamp(x.seconds, 0))
        .ok_or(Error::InvalidTime)
}

pub fn to_timestamp(d: DateTime<FixedOffset>) -> prost_types::Timestamp {
    // 偏移的时间未算在 时分秒 中，所以先转化成无偏移的时间
    let d = d.with_timezone(&Utc);
    prost_types::Timestamp::date_time(
        d.year().into(),
        d.month() as u8,
        d.day() as u8,
        d.hour() as u8,
        d.minute() as u8,
        d.second() as u8,
    )
    .unwrap()
}

pub fn convert_to_utc_time(t: &Timestamp) -> DateTime<Utc> {
    Utc.timestamp(t.seconds, 0)
}

pub fn convert_to_timestamp(d: DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp::date_time(
        d.year().into(),
        d.month() as u8,
        d.day() as u8,
        d.hour() as u8,
        d.minute() as u8,
        d.second() as u8,
    )
    .unwrap()
}
