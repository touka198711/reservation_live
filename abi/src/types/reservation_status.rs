use crate::{ReservationStatus, RsvpStatus};

impl From<RsvpStatus> for ReservationStatus {
    fn from(r: RsvpStatus) -> Self {
        match r {
            RsvpStatus::Unknown => Self::Unknown,
            RsvpStatus::Pending => Self::Pending,
            RsvpStatus::Confirmed => Self::Confirmed,
            RsvpStatus::Blocked => Self::Blocked,
        }
    }
}

impl std::fmt::Display for ReservationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReservationStatus::Unknown => write!(f, "unknown"),
            ReservationStatus::Pending => write!(f, "pending"),
            ReservationStatus::Blocked => write!(f, "blocked"),
            ReservationStatus::Confirmed => write!(f, "confirmed"),
        }
    }
}
