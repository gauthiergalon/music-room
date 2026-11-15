//! Room WebSocket: access control, room lifecycle and playback synchronisation.

mod common;

use std::time::Duration;

use axum::http::{StatusCode, header::AUTHORIZATION};
use backend::services::cleanup::rooms_to_close;
use common::{
    TestApp, TestConfig, add_track, bearer, connect, create_room, expect_no_event_of_type,
    next_of_type, register, send_event,
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

async fn make_private(pool: &PgPool, room_id: Uuid) {
    sqlx::query("UPDATE rooms SET is_public = false WHERE id = $1")
        .bind(room_id)
        .execute(pool)
        .await
        .unwrap();
}

async fn room_exists(pool: &PgPool, room_id: Uuid) -> bool {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM rooms WHERE id = $1)")
        .bind(room_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn track_id(room_state: &Value) -> Option<i64> {
    room_state["current_track"]["id"].as_i64()
}

fn queue_ids(room_state: &Value) -> Vec<i64> {
    room_state["queue"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["track"]["id"].as_i64().unwrap())
        .collect()
}

// ------------------------------------------------------------------ access

#[sqlx::test]
async fn private_room_refuses_users_without_an_invitation(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let stranger = register(&server, "stranger").await;
    let room_id = create_room(&server, &owner).await;
    make_private(&pool, room_id).await;

    server
        .get_websocket(&format!("/rooms/{room_id}/ws"))
        .add_header(AUTHORIZATION, bearer(&stranger.access_token))
        .await
        .assert_status(StatusCode::FORBIDDEN);

    // The owner still gets in.
    let (_socket, _) = connect(&server, &owner, room_id).await;
}

#[sqlx::test]
async fn unknown_room_and_missing_token_are_refused(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let user = register(&server, "user").await;
    let room_id = create_room(&server, &user).await;

    server
        .get_websocket(&format!("/rooms/{}/ws", Uuid::new_v4()))
        .add_header(AUTHORIZATION, bearer(&user.access_token))
        .await
        .assert_status(StatusCode::NOT_FOUND);

    server
        .get_websocket(&format!("/rooms/{room_id}/ws"))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn the_query_string_token_is_only_accepted_on_websocket_upgrades(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let user = register(&server, "browser").await;
    let room_id = create_room(&server, &user).await;

    // A browser cannot set headers on a WebSocket: the token goes in the URL.
    let mut socket = server
        .get_websocket(&format!(
            "/rooms/{room_id}/ws?token={}&device=Web",
            user.access_token
        ))
        .await
        .into_websocket()
        .await;
    let welcome = common::next_event(&mut socket).await;
    assert_eq!(welcome["type"], "Welcome");
    let users = next_of_type(&mut socket, "UserState").await;
    assert_eq!(users["user_list"][0]["device_name"], "Web");

    // Anywhere else the token must come in the Authorization header.
    server
        .get(&format!("/rooms?token={}", user.access_token))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
    server
        .get(&format!("/users/me?token={}", user.access_token))
        .await
        .assert_status(StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn room_limit_refuses_new_rooms(pool: PgPool) {
    let app = TestApp::with_config(
        pool,
        TestConfig {
            max_rooms: 1,
            ..TestConfig::default()
        },
    );
    let server = app.ws_server();
    let first = register(&server, "first").await;
    let second = register(&server, "second").await;
    let first_room = create_room(&server, &first).await;
    let second_room = create_room(&server, &second).await;

    let (_socket, _) = connect(&server, &first, first_room).await;

    server
        .get_websocket(&format!("/rooms/{second_room}/ws"))
        .add_header(AUTHORIZATION, bearer(&second.access_token))
        .await
        .assert_status(StatusCode::SERVICE_UNAVAILABLE);
}

// --------------------------------------------------------------- handshake

#[sqlx::test]
async fn a_new_connection_gets_welcome_users_and_state(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let room_id = create_room(&server, &owner).await;

    // `connect` asserts that Welcome comes first.
    let (mut socket, connection_id) = connect(&server, &owner, room_id).await;

    let state = next_of_type(&mut socket, "RoomState").await;
    assert_eq!(state["current_track"], Value::Null);
    assert_eq!(state["is_playing"], false);
    assert!(state["version"].as_u64().unwrap() >= 1);
    assert!(state["server_time"].is_string());

    let users = next_of_type(&mut socket, "UserState").await;
    assert_eq!(users["owner"], owner.id.to_string());
    assert_eq!(users["user_list"].as_array().unwrap().len(), 1);
    assert_eq!(
        users["user_list"][0]["connection_id"],
        connection_id.to_string()
    );
    assert_eq!(users["user_list"][0]["device_name"], "Test device");
}

#[sqlx::test]
async fn the_displayed_username_is_the_current_one(pool: PgPool) {
    // The name inside the access token may be stale after a rename.
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let owner = register(&server, "before").await;
    let room_id = create_room(&server, &owner).await;

    server
        .patch("/users/me/username")
        .add_header(AUTHORIZATION, bearer(&owner.access_token))
        .json(&json!({ "username": "after_rename" }))
        .await
        .assert_status(StatusCode::OK);

    let (mut socket, _) = connect(&server, &owner, room_id).await;
    let users = next_of_type(&mut socket, "UserState").await;
    assert_eq!(users["user_list"][0]["username"], "after_rename");
}

// --------------------------------------------------------------- lifecycle

#[sqlx::test]
async fn room_survives_an_owner_reconnect_within_the_grace_period(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let listener = register(&server, "listener").await;
    let room_id = create_room(&server, &owner).await;
    add_track(&server, &owner, room_id, 11).await;
    add_track(&server, &owner, room_id, 12).await;

    let (owner_socket, _) = connect(&server, &owner, room_id).await;
    let (mut listener_socket, _) = connect(&server, &listener, room_id).await;

    // The owner's connection drops and comes back before the grace period ends.
    owner_socket.close().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    let (mut owner_socket, _) = connect(&server, &owner, room_id).await;

    // Well past the grace period, the room and its queue are intact.
    expect_no_event_of_type(
        &mut listener_socket,
        "RoomClosed",
        Duration::from_millis(600),
    )
    .await;
    assert!(room_exists(&pool, room_id).await);

    let state = next_of_type(&mut owner_socket, "RoomState").await;
    assert_eq!(track_id(&state), Some(11));
    assert_eq!(queue_ids(&state), vec![12]);
}

#[sqlx::test]
async fn room_closes_when_the_owner_stays_away(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let listener = register(&server, "listener").await;
    let room_id = create_room(&server, &owner).await;

    let (owner_socket, _) = connect(&server, &owner, room_id).await;
    let (mut listener_socket, _) = connect(&server, &listener, room_id).await;

    owner_socket.close().await;

    // Still there right after the disconnection...
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(room_exists(&pool, room_id).await);

    // ...and closed once the grace period (300 ms in tests) has elapsed.
    next_of_type(&mut listener_socket, "RoomClosed").await;
    assert!(!room_exists(&pool, room_id).await);
}

#[sqlx::test]
async fn owner_with_a_second_device_keeps_the_room_open(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let room_id = create_room(&server, &owner).await;

    let (phone, _) = connect(&server, &owner, room_id).await;
    let (mut laptop, _) = connect(&server, &owner, room_id).await;

    phone.close().await;

    expect_no_event_of_type(&mut laptop, "RoomClosed", Duration::from_millis(600)).await;
    assert!(room_exists(&pool, room_id).await);
}

#[sqlx::test]
async fn a_listener_leaving_does_not_close_the_room(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let listener = register(&server, "listener").await;
    let room_id = create_room(&server, &owner).await;

    let (mut owner_socket, _) = connect(&server, &owner, room_id).await;
    let (listener_socket, _) = connect(&server, &listener, room_id).await;
    // Wait until the owner has seen the listener arrive.
    loop {
        let users = next_of_type(&mut owner_socket, "UserState").await;
        if users["user_list"].as_array().unwrap().len() == 2 {
            break;
        }
    }

    listener_socket.close().await;

    let users = next_of_type(&mut owner_socket, "UserState").await;
    assert_eq!(users["user_list"].as_array().unwrap().len(), 1);
    expect_no_event_of_type(&mut owner_socket, "RoomClosed", Duration::from_millis(600)).await;
    assert!(room_exists(&pool, room_id).await);
}

#[sqlx::test]
async fn deleting_a_room_notifies_connected_clients(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let listener = register(&server, "listener").await;
    let room_id = create_room(&server, &owner).await;

    let (_owner_socket, _) = connect(&server, &owner, room_id).await;
    let (mut listener_socket, _) = connect(&server, &listener, room_id).await;

    server
        .delete(&format!("/rooms/{room_id}"))
        .add_header(AUTHORIZATION, bearer(&owner.access_token))
        .await
        .assert_status(StatusCode::NO_CONTENT);

    next_of_type(&mut listener_socket, "RoomClosed").await;
}

#[sqlx::test]
async fn creating_a_new_room_closes_the_previous_one_for_its_listeners(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let listener = register(&server, "listener").await;
    let old_room = create_room(&server, &owner).await;

    let (mut listener_socket, _) = connect(&server, &listener, old_room).await;

    let new_room = create_room(&server, &owner).await;
    assert_ne!(old_room, new_room);

    next_of_type(&mut listener_socket, "RoomClosed").await;
}

#[sqlx::test]
async fn cleanup_only_targets_rooms_nobody_owns_any_more(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let server = app.ws_server();
    let present = register(&server, "present").await;
    let absent = register(&server, "absent").await;
    let recent = register(&server, "recent").await;

    let attended_room = create_room(&server, &present).await;
    let abandoned_room = create_room(&server, &absent).await;
    let fresh_room = create_room(&server, &recent).await;
    let (_socket, _) = connect(&server, &present, attended_room).await;

    // Both older rooms have been untouched for a long time; only one has its owner.
    sqlx::query("UPDATE rooms SET last_active_at = now() - interval '1 hour' WHERE id = ANY($1)")
        .bind(vec![attended_room, abandoned_room])
        .execute(&pool)
        .await
        .unwrap();

    let to_close = rooms_to_close(&app.state).await;

    assert_eq!(to_close, vec![abandoned_room]);
    assert!(!to_close.contains(&fresh_room));
}

// ---------------------------------------------------------------- playback

#[sqlx::test]
async fn adding_a_track_to_an_idle_room_starts_it(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let room_id = create_room(&server, &owner).await;
    let (mut socket, _) = connect(&server, &owner, room_id).await;
    next_of_type(&mut socket, "RoomState").await;

    add_track(&server, &owner, room_id, 21).await;
    let state = next_of_type(&mut socket, "RoomState").await;
    assert_eq!(track_id(&state), Some(21));
    assert_eq!(state["is_playing"], true);
    assert_eq!(state["current_position"], 0);
    assert_eq!(state["current_track"]["title"], "Title 21");
    assert!(queue_ids(&state).is_empty());

    // The next one waits in the queue.
    add_track(&server, &owner, room_id, 22).await;
    let state = next_of_type(&mut socket, "RoomState").await;
    assert_eq!(track_id(&state), Some(21));
    assert_eq!(queue_ids(&state), vec![22]);
    assert_eq!(
        state["queue"][0]["track"]["artists"][0]["name"],
        "Artist 22"
    );
}

#[sqlx::test]
async fn next_track_is_idempotent(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let room_id = create_room(&server, &owner).await;
    for id in [31, 32, 33] {
        add_track(&server, &owner, room_id, id).await;
    }

    // Two devices of the owner both notice that track 31 ended.
    let (mut phone, _) = connect(&server, &owner, room_id).await;
    let (mut laptop, _) = connect(&server, &owner, room_id).await;
    assert_eq!(
        track_id(&next_of_type(&mut phone, "RoomState").await),
        Some(31)
    );

    send_event(&mut phone, "NextTrack", json!({ "track_id": 31 })).await;
    send_event(&mut laptop, "NextTrack", json!({ "track_id": 31 })).await;

    let state = next_of_type(&mut phone, "RoomState").await;
    assert_eq!(track_id(&state), Some(32));
    assert_eq!(queue_ids(&state), vec![33]);

    // The duplicate changed nothing: no second state follows.
    expect_no_event_of_type(&mut phone, "RoomState", Duration::from_millis(300)).await;
}

#[sqlx::test]
async fn next_track_on_an_empty_queue_clears_the_current_track(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let room_id = create_room(&server, &owner).await;
    add_track(&server, &owner, room_id, 41).await;

    let (mut socket, _) = connect(&server, &owner, room_id).await;
    assert_eq!(
        track_id(&next_of_type(&mut socket, "RoomState").await),
        Some(41)
    );

    send_event(&mut socket, "NextTrack", json!({ "track_id": 41 })).await;

    let state = next_of_type(&mut socket, "RoomState").await;
    assert_eq!(state["current_track"], Value::Null);
    assert_eq!(state["is_playing"], false);

    // A later track starts on its own again.
    add_track(&server, &owner, room_id, 42).await;
    let state = next_of_type(&mut socket, "RoomState").await;
    assert_eq!(track_id(&state), Some(42));
    assert_eq!(state["is_playing"], true);
}

#[sqlx::test]
async fn pause_seek_and_play_are_stamped_by_the_server(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let listener = register(&server, "listener").await;
    let room_id = create_room(&server, &owner).await;
    add_track(&server, &owner, room_id, 51).await;

    let (mut owner_socket, _) = connect(&server, &owner, room_id).await;
    let (mut listener_socket, _) = connect(&server, &listener, room_id).await;
    let initial = next_of_type(&mut listener_socket, "RoomState").await;

    // A wildly wrong client clock must not matter: the timestamp is ignored.
    send_event(
        &mut owner_socket,
        "Pause",
        json!({ "position": 12_000, "timestamp": "1999-01-01T00:00:00Z" }),
    )
    .await;
    let paused = next_of_type(&mut listener_socket, "RoomState").await;
    assert_eq!(paused["is_playing"], false);
    assert_eq!(paused["current_position"], 12_000);
    assert!(paused["version"].as_u64() > initial["version"].as_u64());
    assert!(paused["timestamp"].as_str().unwrap() >= initial["timestamp"].as_str().unwrap());
    assert!(!paused["timestamp"].as_str().unwrap().starts_with("1999"));

    send_event(&mut owner_socket, "SeekTo", json!({ "position": 30_000 })).await;
    let sought = next_of_type(&mut listener_socket, "RoomState").await;
    assert_eq!(sought["is_playing"], false);
    assert_eq!(sought["current_position"], 30_000);

    send_event(&mut owner_socket, "Play", json!({ "position": 30_000 })).await;
    let playing = next_of_type(&mut listener_socket, "RoomState").await;
    assert_eq!(playing["is_playing"], true);
    assert_eq!(playing["current_position"], 30_000);
    assert!(playing["version"].as_u64() > sought["version"].as_u64());
}

#[sqlx::test]
async fn listeners_cannot_control_playback(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let listener = register(&server, "listener").await;
    let room_id = create_room(&server, &owner).await;
    add_track(&server, &owner, room_id, 61).await;

    let (mut owner_socket, _) = connect(&server, &owner, room_id).await;
    let (mut listener_socket, _) = connect(&server, &listener, room_id).await;
    next_of_type(&mut owner_socket, "RoomState").await;

    send_event(&mut listener_socket, "Pause", json!({ "position": 5_000 })).await;
    send_event(&mut listener_socket, "NextTrack", json!({ "track_id": 61 })).await;
    // Garbage is ignored too, and does not drop the connection.
    listener_socket.send_text("not json").await;

    expect_no_event_of_type(&mut owner_socket, "RoomState", Duration::from_millis(300)).await;
}

#[sqlx::test]
async fn delegation_moves_control_to_one_device_and_back(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let guest = register(&server, "guest").await;
    let room_id = create_room(&server, &owner).await;
    add_track(&server, &owner, room_id, 71).await;

    let (mut owner_socket, _) = connect(&server, &owner, room_id).await;
    let (mut guest_socket, guest_connection) = connect(&server, &guest, room_id).await;
    next_of_type(&mut owner_socket, "RoomState").await;

    let delegate = |user_id: Uuid, connection_id: Uuid| {
        server
            .post(&format!("/rooms/{room_id}/delegate"))
            .add_header(AUTHORIZATION, bearer(&owner.access_token))
            .json(&json!({ "user_id": user_id, "connection_id": connection_id }))
    };

    // Unknown device, self-delegation and non-owners are refused.
    delegate(guest.id, Uuid::new_v4())
        .await
        .assert_status(StatusCode::NOT_FOUND);
    delegate(owner.id, guest_connection)
        .await
        .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    server
        .post(&format!("/rooms/{room_id}/delegate"))
        .add_header(AUTHORIZATION, bearer(&guest.access_token))
        .json(&json!({ "user_id": guest.id, "connection_id": guest_connection }))
        .await
        .assert_status(StatusCode::FORBIDDEN);

    delegate(guest.id, guest_connection)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    loop {
        let users = next_of_type(&mut owner_socket, "UserState").await;
        if users["delegate_connection_id"] == guest_connection.to_string() {
            assert_eq!(users["delegate_user_id"], guest.id.to_string());
            break;
        }
    }

    // The delegate controls playback; the owner no longer does.
    send_event(&mut owner_socket, "Pause", json!({ "position": 1_000 })).await;
    send_event(&mut guest_socket, "Pause", json!({ "position": 2_000 })).await;
    let state = next_of_type(&mut owner_socket, "RoomState").await;
    assert_eq!(state["current_position"], 2_000);

    server
        .delete(&format!("/rooms/{room_id}/delegate"))
        .add_header(AUTHORIZATION, bearer(&owner.access_token))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    loop {
        let users = next_of_type(&mut owner_socket, "UserState").await;
        if users["delegate_user_id"].is_null() {
            break;
        }
    }

    send_event(&mut owner_socket, "Pause", json!({ "position": 3_000 })).await;
    let state = next_of_type(&mut owner_socket, "RoomState").await;
    assert_eq!(state["current_position"], 3_000);
}

#[sqlx::test]
async fn queue_changes_are_broadcast_with_increasing_versions(pool: PgPool) {
    let app = TestApp::new(pool);
    let server = app.ws_server();
    let owner = register(&server, "owner").await;
    let room_id = create_room(&server, &owner).await;
    let (mut socket, _) = connect(&server, &owner, room_id).await;
    let mut last_version = next_of_type(&mut socket, "RoomState").await["version"]
        .as_u64()
        .unwrap();

    for id in [81, 82, 83] {
        add_track(&server, &owner, room_id, id).await;
        let version = next_of_type(&mut socket, "RoomState").await["version"]
            .as_u64()
            .unwrap();
        assert!(version > last_version);
        last_version = version;
    }
}
