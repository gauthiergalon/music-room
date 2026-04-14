use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub struct Friend {
    pub user_id_1: Uuid,
    pub user_id_2: Uuid,
    pub sender_id: Uuid,
    pub is_pending: bool,
}
