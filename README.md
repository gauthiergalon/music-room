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
- **Redis** for caching and sessions
- **JWT** for authentication
- **Google OAuth 2.0** for social login
- **HiFi API** for audio streaming

### Mobile
- **Flutter** for iOS, Android, and Web

## Prerequisites

- Docker and Docker Compose
- Rust (via rustup)
- Flutter SDK

## Configuration

1. Copy `.env.example` to `.env` and configure the variables:

```bash
cp .env.example .env
```

Key variables:

#### Database
| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection URL | `postgresql://user:pass@localhost:5432/musicroom` |
| `POSTGRES_USER` | PostgreSQL username | `musicroom` |
| `POSTGRES_PASSWORD` | PostgreSQL password | `secret` |

#### Redis
| Variable | Description | Example |
|----------|-------------|---------|
| `REDIS_URL` | Redis connection URL | `redis://redis:6379` |

#### Authentication
| Variable | Description | Example |
|----------|-------------|---------|
| `JWT_SECRET` | Secret key for signing JWT tokens | `supersecretkey` |

#### Google OAuth
| Variable | Description | Example |
|----------|-------------|---------|
| `GOOGLE_AUTH_URL` | Google OAuth 2.0 endpoint | `https://oauth2.googleapis.com` |
| `GOOGLE_CLIENT_ID` | Google OAuth client ID | `106866...apps.googleusercontent.com` |
| `GOOGLE_CLIENT_SECRET` | Google OAuth client secret | `GOCSPX-...` |

#### HiFi API (Audio Streaming)
| Variable | Description | Example |
|----------|-------------|---------|
| `HIFI_API_HOST` | HiFi API host | `hifi-api` or `localhost` |
| `HIFI_API_CLIENT_ID` | HiFi API client ID | `fX2JxdmntZWK0ixT` |
| `HIFI_API_CLIENT_SECRET` | HiFi API client secret | `1Nm5AfDAjxrgJFJ...` |
| `HIFI_API_USER_ID` | HiFi user ID | `208227952` |
| `HIFI_API_ACCESS_TOKEN` | HiFi access token (JWT) | `eyJraWQi...` |
| `HIFI_API_REFRESH_TOKEN` | HiFi refresh token (JWT) | `eyJraWQi...` |

## Running

### With Docker Compose (recommended)

```bash
docker-compose up
```

Available services:
- **Backend**: http://localhost:3000
- **Swagger UI**: http://localhost:3000/swagger
- **PostgreSQL**: localhost:5432
- **Redis**: localhost:6379
- **Mailhog** (email testing): http://localhost:8025
- **HiFi API**: localhost:8000

### Local Backend

```bash
cd backend
cargo run
```

### Mobile

```bash
cd mobile
flutter pub get
flutter run
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
├── middleware/   # Middleware (auth, CORS, etc.)
└── ws/           # WebSocket handlers
```

## API

Swagger documentation is available at `/swagger` once the backend is running.