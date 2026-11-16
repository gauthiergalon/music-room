# Music Room

Real-time collaborative music room application.

## Architecture

```
music-room/
├── backend/          # REST API + WebSocket (Rust + Axum)
├── mobile/           # Flutter Application (iOS, Android, Web)
└── docker-compose.yaml
```

## Technologies

### Backend
- **Rust** with [Axum](https://github.com/tokio-rs/axum) for REST API and WebSocket
- **PostgreSQL** for data persistence
- **JWT** for authentication
- **Google Sign-In** (ID tokens verified against Google's public keys)
- **HiFi API** for audio streaming

### Mobile
- **Flutter** for iOS, Android, and Web

## Prerequisites

- Docker and Docker Compose
- Rust (via rustup), only to run the backend outside Docker
- Flutter SDK

## Configuration

Copy `.env.example` to `.env` and fill it in:

```bash
cp .env.example .env
```

#### Database
| Variable | Description |
|----------|-------------|
| `DATABASE_URL` | PostgreSQL connection URL |
| `POSTGRES_USER` | PostgreSQL username |
| `POSTGRES_PASSWORD` | PostgreSQL password |
| `DB_MAX_CONNECTIONS` | Size of the connection pool (default `10`) |

#### Authentication
| Variable | Description |
|----------|-------------|
| `JWT_SECRET` | Secret key for signing JWT tokens |
| `GOOGLE_CLIENT_ID` | Google OAuth client ID the mobile app signs in with |
| `GOOGLE_JWKS_URL` | Where Google's signing keys are fetched (default: Google's endpoint) |
| `AUTH_RATE_LIMIT_PER_MINUTE` | Requests per client and per minute on login, register and password reset (default `30`, `0` disables) |

#### HiFi API (Audio Streaming)
| Variable | Description |
|----------|-------------|
| `HIFI_API_HOST` | `host[:port]` of the HiFi API (default `localhost:8000`) |
| `HIFI_API_CLIENT_ID` | HiFi API client ID |
| `HIFI_API_CLIENT_SECRET` | HiFi API client secret |
| `HIFI_API_USER_ID` | HiFi user ID |
| `HIFI_API_ACCESS_TOKEN` | HiFi access token (JWT) |
| `HIFI_API_REFRESH_TOKEN` | HiFi refresh token (JWT) |
| `STREAM_PROXY_ALLOWED_HOSTS` | Domains the web audio relay may fetch from (default `tidal.com`) |

#### Email / SMTP
| Variable | Description |
|----------|-------------|
| `SMTP_HOST` | SMTP server host (default `localhost`) |
| `SMTP_PORT` | SMTP port (default `1025` without TLS, the standard port otherwise) |
| `SMTP_SECURITY` | `none` (default, for Mailhog), `starttls` or `tls` |
| `SMTP_USERNAME` / `SMTP_PASSWORD` | Credentials, if the server needs them |
| `SMTP_FROM` | Sender address |

#### Other
| Variable | Description |
|----------|-------------|
| `PORT` | Port the backend listens on (default `3000`) |
| `CORS_ALLOWED_ORIGINS` | Comma-separated allowed origins; unset or `*` allows any |

## Running

### With Docker Compose (recommended)

```bash
docker compose up --build
```

Inside Compose the backend reaches the other services by their service name, so
`.env` must use `postgres`, `hifi-api:8000` and `mailhog` as hosts (see
`.env.example`).

Available services:
- **Backend**: http://localhost:3000
- **Swagger UI**: http://localhost:3000/swagger-ui
- **PostgreSQL**: localhost:5432
- **Mailhog** (email testing): http://localhost:8025
- **HiFi API**: localhost:8000

The backend applies the SQL migrations in `backend/migrations/` when it starts,
on a fresh database as well as on an existing one. To change the schema, add a
new numbered file; never edit a migration that has already been applied.

### Local Backend

```bash
cd backend
cargo run
```

### Mobile

```bash
cd mobile
cp .env.example .env   # backend URL and Google client ID
flutter pub get
flutter run
```

A release APK is signed with the debug key unless `mobile/android/key.properties`
exists (see the comment in `mobile/android/app/build.gradle.kts`).

## Tests

### Backend

The tests need a PostgreSQL server; each test creates its own database on it.

```bash
cd backend
DATABASE_URL=postgres://user:password@localhost:5432/postgres cargo test
```

SQL queries are checked at compile time. After changing a query, refresh the
metadata used by offline builds (Docker image, CI) and commit the result:

```bash
cargo install sqlx-cli --no-default-features --features postgres,rustls
cargo sqlx prepare -- --all-targets
```

### Mobile

```bash
cd mobile
flutter analyze
flutter test
```

### Load test

How many people can use the Playlist Editor and the Control Delegation at
the same time, on a backend capped like a 2 vCPU / 2 GB server. Method,
success criteria and results: [loadtest/README.md](loadtest/README.md).

```bash
cd loadtest
./run.sh playlist
./run.sh delegation
```

## Backend Structure

```
backend/src/
├── handlers/     # HTTP/WebSocket handlers
├── models/       # Data models
├── repositories/ # Data access
├── services/     # Business logic
├── dtos/         # Data Transfer Objects
├── routes/       # Route definitions
├── middleware/   # Authentication, request logging, rate limiting
├── ws/           # WebSocket connections and room presence
└── validation.rs # Normalisation of usernames, emails and passwords
```

## Rooms and playback

- The database decides whether a room exists; the server only keeps in memory
  who is connected to it.
- A room is closed when its owner deletes it, creates another one, stays
  disconnected for more than 30 seconds, or when nobody controls it for two hours.
- The server stamps every playback change with its own clock and numbers each
  room state, so clients stay in sync whatever their local clock says.
- When the queue is not empty and nothing is playing, the next track starts by
  itself.
- Tracks are streamed as HLS. Native apps load them straight from the
  provider. The provider's CDN refuses requests made by a browser, so the web
  app plays through the backend (`/hifi/stream/...`, signed links) with
  hls.js (`mobile/web/hls_audio.js`).

## API

Swagger documentation is available at `/swagger-ui` once the backend is running.
The WebSocket protocol is described in `backend/src/dtos/ws.rs`.
