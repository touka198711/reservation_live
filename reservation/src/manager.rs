use crate::{ReservationId, ReservationManager, Rsvp};
use abi::{ReservationStatus, Validator};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{postgres::types::PgRange, types::Uuid, PgPool, Row};

#[async_trait]
impl Rsvp for ReservationManager {
    async fn reserve(&self, mut rsvp: abi::Reservation) -> Result<abi::Reservation, abi::Error> {
        rsvp.validate()?;

        let status = ReservationStatus::from_i32(rsvp.status).unwrap_or(ReservationStatus::Pending);

        let range: PgRange<DateTime<Utc>> = rsvp.get_timespan();

        let sql = r#"
            INSERT INTO rsvp.reservations (user_id, resource_id, timespan, note, status) 
            VALUES ($1, $2, $3, $4, $5::rsvp.reservation_status) RETURNING id
        "#;
        let id: Uuid = sqlx::query(sql)
            .bind(rsvp.user_id.clone())
            .bind(rsvp.resource_id.clone())
            .bind(range)
            .bind(rsvp.note.clone())
            .bind(status.to_string())
            .fetch_one(&self.pool)
            .await?
            .get("id");

        rsvp.id = id.to_string();
        Ok(rsvp)
    }

    async fn change_status(&self, id: ReservationId) -> Result<abi::Reservation, abi::Error> {
        let id = Uuid::parse_str(&id).map_err(|_| abi::Error::InvalidReservationId(id.clone()))?;
        let rsvp = sqlx::query_as::<_, abi::Reservation>(r#"
        UPDATE rsvp.reservations SET status = 'confirmed' WHERE id = $1 AND status = 'pending' RETURNING *
        "#)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(rsvp)
    }


    async fn update_note(
        &self,
        id: ReservationId,
        note: String,
    ) -> Result<abi::Reservation, abi::Error> {
        let id = Uuid::parse_str(&id).map_err(|_| abi::Error::InvalidReservationId(id.clone()))?;
        let rsvp = sqlx::query_as::<_, abi::Reservation>(r#"
        UPDATE rsvp.reservations SET note = $1 WHERE id = $2 RETURNING *
        "#)
        .bind(note)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(rsvp)
    }

    async fn delete(&self, id: ReservationId) -> Result<(), abi::Error> {
        let id = Uuid::parse_str(&id).map_err(|_| abi::Error::InvalidReservationId(id.clone()))?;
        let _ = sqlx::query("DELETE FROM rsvp.reservations WHERE id = $1")
        .bind(id)
        .execute(&self.pool)
        // .fetch_optional(&self.pool)
        .await?;
        
        Ok(())
    }

    async fn get(&self, id: ReservationId) -> Result<abi::Reservation, abi::Error> {
        let id = Uuid::parse_str(&id).map_err(|_| abi::Error::InvalidReservationId(id.clone()))?;
        let rsvp = sqlx::query_as::<_, abi::Reservation>(r#"
        SELECT * FROM rsvp.reservations WHERE id = $1
        "#)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        
        Ok(rsvp)
    }

    async fn query(
        &self,
        query: abi::ReservationQuery,
    ) -> Result<Vec<abi::Reservation>, abi::Error> {
        let user_id = str_to_option(&query.user_id);
        let resource_id = str_to_option(&query.resource_id);
        let timespan = query.timespan();
        let status = ReservationStatus::from_i32(query.status)
            .unwrap_or(ReservationStatus::Pending);

        let rsvps = sqlx::query_as::<_, abi::Reservation>("SELECT * FROM rsvp.query($1, $2, $3, $4::rsvp.reservation_status, $5, $6, $7)")
            .bind(user_id)
            .bind(resource_id)
            .bind(timespan)
            .bind(status.to_string())
            .bind(query.page)
            .bind(query.desc)
            .bind(query.pagesize)
            .fetch_all(&self.pool)
            .await?;

        Ok(rsvps)
    }
}

fn str_to_option(s: &str) -> Option<&str> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

impl ReservationManager {
    pub fn new(pool: PgPool) -> ReservationManager {
        Self { pool }
    }
}

#[cfg(test)]
mod tests {

    use abi::{Reservation, ReservationConflictInfo, ReservationConflict, ReservationWindow, ReservationQueryBuilder};
    use chrono::{DateTime, FixedOffset};

    use super::*;

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_should_work_for_valid_window() {
        let manager = ReservationManager::new(migrated_pool.clone());
        let start: DateTime<FixedOffset> = "2022-12-25T15:00:00-0700".parse().unwrap();
        let end: DateTime<FixedOffset> = "2022-12-28T12:00:00-0700".parse().unwrap();
        let rsvp = Reservation::new_pending("tyrid", "1121", start, end, "......");
        let res = manager.reserve(rsvp).await;
        assert!(res.is_ok());
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_conflict_reservation_should_reject() {
        let manager = ReservationManager::new(migrated_pool.clone());
        let rsvp1 = Reservation::new_pending(
            "tyrid",
            "1121",
            "2022-12-25T15:00:00-0700".parse().unwrap(),
            "2022-12-28T12:00:00-0700".parse().unwrap(),
            "hello",
        );
        let rsvp2 = Reservation::new_pending(
            "aliceid",
            "1121",
            "2022-12-26T15:00:00-0700".parse().unwrap(),
            "2022-12-30T12:00:00-0700".parse().unwrap(),
            "world",
        );

        let _rsvp1 = manager.reserve(rsvp1).await.unwrap();
        let err = manager.reserve(rsvp2).await.unwrap_err();
       
        let info = ReservationConflictInfo::Parsed(ReservationConflict {
            new: ReservationWindow { 
                rid: "1121".to_string(), 
                start: "2022-12-26T15:00:00-0700".parse().unwrap(), 
                end: "2022-12-30T12:00:00-0700".parse().unwrap(), 
            }, 
            old: ReservationWindow {
                rid: "1121".to_string(), 
                start: "2022-12-25T15:00:00-0700".parse().unwrap(), 
                end: "2022-12-28T12:00:00-0700".parse().unwrap(), 
            }
        });

        assert_eq!(err, abi::Error::ConflictReservation(info));
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_change_status_should_work() {
        let manager = ReservationManager::new(migrated_pool.clone());
        let rsvp = Reservation::new_pending(
            "aliceId",
            "1021",
            "2023-01-25T15:00:00-0700".parse().unwrap(),
            "2023-02-25T12:00:00-0700".parse().unwrap(),
            "hello...",
        );
        let rsvp = manager.reserve(rsvp).await.unwrap();

        let res = manager
            .change_status(rsvp.id)
            .await
            .unwrap();

        assert_eq!(res.status, abi::ReservationStatus::Confirmed as i32);
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn reserve_change_status_not_pending_should_do_nothings() {
        let manager = ReservationManager::new(migrated_pool.clone());
        let rsvp = Reservation::new_pending(
            "aliceId",
            "1021",
            "2023-01-25T15:00:00-0700".parse().unwrap(),
            "2023-02-25T12:00:00-0700".parse().unwrap(),
            "hello...",
        );
        let rsvp = manager.reserve(rsvp).await.unwrap();

        let rsvp = manager
            .change_status(rsvp.id)
            .await
            .unwrap();

        assert_eq!(rsvp.status, abi::ReservationStatus::Confirmed as i32);

        let ret = manager.change_status(rsvp.id).await.unwrap_err();

        assert_eq!(ret, abi::Error::NotFound);
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn update_note_should_work() {
        let (manager, rsvp) = make_alice_reservation(&migrated_pool.clone()).await;

        let rsvp = manager.update_note(rsvp.id, "world.".to_string()).await.unwrap();

        assert_eq!(rsvp.note, "world.");
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn get_reservation_should_work() {
        let (manager, rsvp) = make_tyr_reservation(&migrated_pool.clone()).await;

        let res = manager.get(rsvp.id).await.unwrap();

        assert_eq!(res.user_id, rsvp.user_id);
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn dalete_reservation_should_work() {
        let (manager, rsvp) = make_tyr_reservation(&migrated_pool.clone()).await;

        let _ = manager.delete(rsvp.id.clone()).await.unwrap();
        let err = manager.get(rsvp.id).await.unwrap_err();
        println!("{:?}", err);
    }

    #[sqlx_database_tester::test(pool(variable = "migrated_pool", migrations = "../migrations"))]
    async fn query_reservation_should_work() {
        let (manager, rsvp) = make_tyr_reservation(&migrated_pool.clone()).await;

        let query = ReservationQueryBuilder::default()
            .user_id("tyrId")
            .resource_id("1021")
            .start("2022-12-25T15:00:00-0700".parse::<prost_types::Timestamp>().unwrap())
            .end("2022-12-28T12:00:00-0700".parse::<prost_types::Timestamp>().unwrap())
            .status(ReservationStatus::Pending)
            .build().unwrap();

        let rsvps = manager.query(query).await.unwrap();

        assert_eq!(rsvps.len(), 1);
        assert_eq!(rsvps[0], rsvp);

        let query = ReservationQueryBuilder::default()
            .user_id("tyrId")
            .resource_id("1021")
            .start("2023-01-25T15:00:00-0700".parse::<prost_types::Timestamp>().unwrap())
            .end("2023-02-28T12:00:00-0700".parse::<prost_types::Timestamp>().unwrap())
            .status(ReservationStatus::Pending)
            .build().unwrap();

        let rsvps1  = manager.query(query).await.unwrap();
        
        assert!(rsvps1.is_empty());
        
        let _rsvp =  manager.change_status(rsvps[0].id.clone()).await.unwrap();

        let query = ReservationQueryBuilder::default()
            .user_id("tyrId")
            .resource_id("1021")
            .start("2022-12-25T15:00:00-0700".parse::<prost_types::Timestamp>().unwrap())
            .end("2022-12-28T12:00:00-0700".parse::<prost_types::Timestamp>().unwrap())
            .status(ReservationStatus::Pending)
            .build().unwrap();

        let rsvps1  = manager.query(query).await.unwrap();

        assert!(rsvps1.is_empty());
    }


    async fn make_tyr_reservation(pool: &PgPool) -> (ReservationManager, Reservation) {
        make_reservation(
            pool,
            "tyrId",
            "1021",
            "2022-12-25T15:00:00-0700",
            "2022-12-28T12:00:00-0700",
            "hahaha..."
        )
        .await
    }

    async fn make_alice_reservation(pool: &PgPool) -> (ReservationManager, Reservation) {
        make_reservation(
            pool,
            "aliceId",
            "1021",
            "2023-01-25T15:00:00-0700",
            "2023-02-25T12:00:00-0700",
            "hello..."
        )
        .await
    }

    async fn make_reservation(
        pool: &PgPool, 
        uid: &str, 
        rid: &str, 
        start: &str, 
        end: &str, 
        note: &str
    ) -> (ReservationManager, Reservation) {
        let manager = ReservationManager::new(pool.clone());
        let rsvp = Reservation::new_pending(
            uid, rid, start.parse().unwrap(), end.parse().unwrap(), note
        );
        let rsvp = manager.reserve(rsvp).await.unwrap();

        (manager, rsvp)
    }
}
