mod common;

use axum::http::StatusCode;
use axum_test::TestServer;
use common::create_app;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

#[derive(Serialize, Deserialize, Debug)]
struct TestAuthResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct TestRoomResponse {
    id: String,
    owner_id: String,
}

async fn register_and_login(server: &TestServer, username: &str, email: &str) -> String {
    let res = server
        .post("/auth/register")
        .json(&json!({
            "username": username,
            "email": email,
            "password": "password123"
        }))
        .await;

    let json = res.json::<TestAuthResponse>();
    json.access_token
}

#[sqlx::test]
async fn test_create(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_create", "create@example.com").await;

    let res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::CREATED);
    let room = res.json::<TestRoomResponse>();
    assert!(!room.id.is_empty());
    assert!(!room.owner_id.is_empty());
}

#[sqlx::test]
async fn test_get_room(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_get", "get@example.com").await;

    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    let res = server
        .get(&format!("/rooms/{}", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::OK);
    let fetched_room = res.json::<TestRoomResponse>();
    assert_eq!(fetched_room.id, room.id);
    assert_eq!(fetched_room.owner_id, room.owner_id);
}

#[sqlx::test]
async fn test_delete(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_del", "del@example.com").await;

    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    let res = server
        .delete(&format!("/rooms/{}", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::NO_CONTENT);

    // Verify it's deleted
    let get_res = server
        .get(&format!("/rooms/{}", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    get_res.assert_status(StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn test_enable_license_room(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_pub", "pub@example.com").await;

    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    let res = server
        .post(&format!("/rooms/{}/enable-license", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::NO_CONTENT);
}

#[sqlx::test]
async fn test_disable_license_room(pool: PgPool) {
    let app = create_app(pool);
    let server = TestServer::new(app);
    let token = register_and_login(&server, "test_priv", "priv@example.com").await;

    let create_res = server
        .post("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;
    let room = create_res.json::<TestRoomResponse>();

    let res = server
        .post(&format!("/rooms/{}/disable-license", room.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", token),
        )
        .await;

    res.assert_status(StatusCode::NO_CONTENT);
}

async fn list_rooms(server: &TestServer, user: &common::TestUser) -> Vec<String> {
    server
        .get("/rooms")
        .add_header(
            axum::http::header::AUTHORIZATION,
            common::bearer(&user.access_token),
        )
        .await
        .json::<Vec<TestRoomResponse>>()
        .into_iter()
        .map(|room| room.id)
        .collect()
}

#[sqlx::test]
async fn test_list_hides_private_rooms_from_users_without_access(pool: PgPool) {
    let server = TestServer::new(create_app(pool.clone()));
    let owner = common::register(&server, "owner").await;
    let guest = common::register(&server, "guest").await;
    let stranger = common::register(&server, "stranger").await;
    let other_owner = common::register(&server, "public").await;

    let private_room = common::create_room(&server, &owner).await;
    let public_room = common::create_room(&server, &other_owner).await;
    sqlx::query("UPDATE rooms SET is_public = false WHERE id = $1")
        .bind(private_room)
        .execute(&pool)
        .await
        .unwrap();

    // Invite the guest and let them accept.
    let invitation = server
        .post(&format!("/rooms/{private_room}/invite/{}", guest.id))
        .add_header(
            axum::http::header::AUTHORIZATION,
            common::bearer(&owner.access_token),
        )
        .await;
    invitation.assert_status(StatusCode::CREATED);
    let invitation_id = invitation.json::<serde_json::Value>()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // A pending invitation does not give access yet.
    assert_eq!(
        list_rooms(&server, &guest).await,
        vec![public_room.to_string()]
    );

    server
        .post(&format!("/me/invitations/{invitation_id}/accept"))
        .add_header(
            axum::http::header::AUTHORIZATION,
            common::bearer(&guest.access_token),
        )
        .await
        .assert_status(StatusCode::NO_CONTENT);

    let private_id = private_room.to_string();
    assert!(list_rooms(&server, &owner).await.contains(&private_id));
    assert!(list_rooms(&server, &guest).await.contains(&private_id));
    assert_eq!(
        list_rooms(&server, &stranger).await,
        vec![public_room.to_string()]
    );
}

#[sqlx::test]
async fn test_list_is_paginated(pool: PgPool) {
    let server = TestServer::new(create_app(pool));
    let mut viewer = None;
    for index in 0..3 {
        let user = common::register(&server, &format!("page{index}")).await;
        common::create_room(&server, &user).await;
        viewer = Some(user);
    }
    let viewer = viewer.unwrap();

    let page = |query: &'static str| {
        server.get(&format!("/rooms{query}")).add_header(
            axum::http::header::AUTHORIZATION,
            common::bearer(&viewer.access_token),
        )
    };

    assert_eq!(page("").await.json::<Vec<TestRoomResponse>>().len(), 3);
    let first = page("?limit=2").await.json::<Vec<TestRoomResponse>>();
    let rest = page("?limit=2&offset=2")
        .await
        .json::<Vec<TestRoomResponse>>();
    assert_eq!(first.len(), 2);
    assert_eq!(rest.len(), 1);
    assert!(first.iter().all(|room| room.id != rest[0].id));
}

#[sqlx::test]
async fn test_creating_a_room_replaces_the_previous_one(pool: PgPool) {
    let server = TestServer::new(create_app(pool));
    let owner = common::register(&server, "again").await;

    let first = common::create_room(&server, &owner).await;
    let second = common::create_room(&server, &owner).await;

    assert_ne!(first, second);
    assert_eq!(list_rooms(&server, &owner).await, vec![second.to_string()]);
}

#[sqlx::test]
async fn test_room_name_uses_the_current_username(pool: PgPool) {
    // The username in the access token can be stale after a rename.
    let server = TestServer::new(create_app(pool));
    let owner = common::register(&server, "oldname").await;
    let auth = common::bearer(&owner.access_token);

    server
        .patch("/users/me/username")
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .json(&json!({ "username": "fresh_name" }))
        .await
        .assert_status(StatusCode::OK);

    let room = server
        .post("/rooms")
        .add_header(axum::http::header::AUTHORIZATION, auth)
        .await
        .json::<serde_json::Value>();
    assert_eq!(room["name"], "fresh_name's room");
}

#[sqlx::test]
async fn test_list_shows_the_track_playing_in_each_room(pool: PgPool) {
    let server = TestServer::new(create_app(pool));
    let owner = common::register(&server, "dj").await;
    let auth = common::bearer(&owner.access_token);
    let room_id = common::create_room(&server, &owner).await;

    let rooms = server
        .get("/rooms")
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .await
        .json::<serde_json::Value>();
    assert!(rooms[0]["now_playing"].is_null());

    // The first track added to an idle room starts playing by itself.
    server
        .post(&format!("/rooms/{room_id}/queue"))
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .json(&json!({ "track_id": 42 }))
        .await
        .assert_status(StatusCode::NO_CONTENT);

    let rooms = server
        .get("/rooms")
        .add_header(axum::http::header::AUTHORIZATION, auth.clone())
        .await
        .json::<serde_json::Value>();
    let track = &rooms[0]["now_playing"];
    assert_eq!(rooms[0]["current_track"], 42);
    assert_eq!(track["id"], 42);
    assert_eq!(track["title"], "Title 42");
    assert_eq!(track["album"]["cover"], "cover-id");
    assert!(track["artists"][0]["name"].is_string());

    let room = server
        .get(&format!("/rooms/{room_id}"))
        .add_header(axum::http::header::AUTHORIZATION, auth)
        .await
        .json::<serde_json::Value>();
    assert_eq!(room["now_playing"]["title"], "Title 42");
}
