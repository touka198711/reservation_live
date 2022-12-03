use std::ops::Bound;

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use sqlx::{
    postgres::{types::PgRange, PgRow},
    types::Uuid,
    FromRow, Row,
};

use crate::{utils::to_timestamp, Error, Reservation, ReservationStatus, RsvpStatus, Validator};

use super::{get_timespan, validate_range};

impl Reservation {
    pub fn new_pending(
        uid: impl Into<String>,
        rid: impl Into<String>,
        start: DateTime<FixedOffset>,
        end: DateTime<FixedOffset>,
        note: impl Into<String>,
    ) -> Self {
        Self {
            id: "".to_string(),
            resource_id: rid.into(),
            status: ReservationStatus::Pending as i32,
            user_id: uid.into(),
            end_time: Some(to_timestamp(end)),
            start_time: Some(to_timestamp(start)),
            note: note.into(),
        }
    }

    pub fn get_timespan(&self) -> PgRange<DateTime<Utc>> {
        get_timespan(self.start_time.as_ref(), self.end_time.as_ref())
    }
}

impl Validator for Reservation {
    fn validate(&self) -> Result<(), Error> {
        if self.user_id.is_empty() {
            return Err(Error::InvalidUserId("".into()));
        }

        if self.resource_id.is_empty() {
            return Err(Error::InvalidResourceId("".into()));
        }

        validate_range(self.start_time.as_ref(), self.end_time.as_ref())?;

        Ok(())
    }
}

impl FromRow<'_, PgRow> for Reservation {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        let range: PgRange<DateTime<Utc>> = row.get("timespan");
        let range: NaiveRange<DateTime<Utc>> = range.into();

        assert!(range.start.is_some() && range.end.is_some());
        let start =
            to_timestamp(FixedOffset::east(0).from_utc_datetime(&range.start.unwrap().naive_utc()));
        let end =
            to_timestamp(FixedOffset::east(0).from_utc_datetime(&range.end.unwrap().naive_utc()));

        let status: RsvpStatus = row.get("status");

        let id: Uuid = row.get("id");

        Ok(Self {
            id: id.to_string(),
            resource_id: row.get("resource_id"),
            status: ReservationStatus::from(status) as i32,
            user_id: row.get("user_id"),
            end_time: Some(end),
            start_time: Some(start),
            note: row.get("note"),
        })
    }
}

struct NaiveRange<T> {
    start: Option<T>,
    end: Option<T>,
}

impl<T> From<PgRange<T>> for NaiveRange<T> {
    fn from(range: PgRange<T>) -> Self {
        let f = |b: Bound<T>| match b {
            Bound::Included(v) => Some(v),
            Bound::Excluded(v) => Some(v),
            Bound::Unbounded => None,
        };
        let start = f(range.start);
        let end = f(range.end);

        Self { start, end }
    }
}
