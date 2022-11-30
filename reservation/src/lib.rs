mod manager;
use async_trait::async_trait;
use sqlx::PgPool;

pub type ReservationId = String;
pub type UserId = String;
pub type ResourceId = String;

#[derive(Debug)]
pub struct ReservationManager {
    pool: PgPool,
}

#[async_trait]
pub trait Rsvp {
    async fn reserve(&self, rsvp: abi::Reservation) -> Result<abi::Reservation, abi::Error>;
    async fn change_status(&self, id: ReservationId) -> Result<abi::Reservation, abi::Error>;
    async fn update_note(
        &self,
        id: ReservationId,
        note: String,
    ) -> Result<abi::Reservation, abi::Error>;
    async fn delete(&self, id: ReservationId) -> Result<(), abi::Error>;
    async fn get(&self, id: ReservationId) -> Result<abi::Reservation, abi::Error>;
    async fn query(
        &self,
        query: abi::ReservationQuery,
    ) -> Result<Vec<abi::Reservation>, abi::Error>;
}
