//! The migrations must apply to a database created before the application ran
//! them itself (the schema used to be loaded by the PostgreSQL container).

use sqlx::PgPool;

/// The schema exactly as it was before migrations became idempotent.
const LEGACY_SCHEMA: &str = include_str!("fixtures/legacy_schema.sql");

async fn column_exists(pool: &PgPool, table: &str, column: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = $1 AND column_name = $2)",
    )
    .bind(table)
    .bind(column)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = false)]
async fn migrations_upgrade_a_legacy_database_and_keep_its_data(pool: PgPool) {
    sqlx::raw_sql(LEGACY_SCHEMA).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO users (username, email, password_hash) VALUES ('legacy', 'legacy@example.com', 'hash')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO rooms (owner_id, name, current_track) SELECT id, 'legacy room', 7 FROM users",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(!column_exists(&pool, "rooms", "last_active_at").await);

    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    assert!(column_exists(&pool, "rooms", "last_active_at").await);
    let rooms: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rooms WHERE name = 'legacy room'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rooms, 1);
    // The track that was playing gets a playback identifier.
    let has_item: bool =
        sqlx::query_scalar("SELECT current_item IS NOT NULL FROM rooms WHERE name = 'legacy room'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(has_item);

    // Starting the server a second time is a no-op.
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
}

#[sqlx::test(migrations = false)]
async fn migrations_create_a_fresh_database(pool: PgPool) {
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    for table in [
        "users",
        "rooms",
        "queue",
        "refresh_tokens",
        "reset_tokens",
        "email_tokens",
        "friends",
        "invitations",
        "tracks",
    ] {
        assert!(
            column_exists(&pool, table, primary_column(table)).await,
            "{table}"
        );
    }
}

fn primary_column(table: &str) -> &'static str {
    match table {
        "refresh_tokens" | "reset_tokens" | "email_tokens" => "token_hash",
        "friends" => "user_id_1",
        _ => "id",
    }
}
