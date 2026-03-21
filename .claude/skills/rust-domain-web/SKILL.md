---
name: rust-domain-web
description: Rust web backend development. Activates for HTTP servers, REST APIs, middleware, authentication, database integration, and web application architecture with axum, actix-web, or tower.
---

# Rust Web Domain Skill

## Stack Selection
| Component | Recommended | Alternative |
|---|---|---|
| Framework | `axum` | `actix-web` |
| Middleware | `tower` + `tower-http` | — |
| Database | `sqlx` | `diesel`, `sea-orm` |
| Migrations | `sqlx migrate` | `refinery` |
| Auth | `axum-extra` (cookies) + `jsonwebtoken` | `oauth2` |
| Validation | `validator` (derive) | `garde` |
| OpenAPI | `utoipa` | `aide` |
| HTTP client | `reqwest` | `hyper` (low-level) |

## Axum Application Structure
```rust
use axum::{Router, routing::{get, post}, middleware};

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .nest("/api/v1", api_routes())
        .layer(middleware::from_fn(logging_middleware))
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
}

fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route("/users/:id", get(get_user).put(update_user).delete(delete_user))
}
```

## Handler Pattern
```rust
use axum::{extract::{State, Path, Json}, http::StatusCode};

async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<User>), AppError> {
    let user = state.user_service.create(payload).await?;
    Ok((StatusCode::CREATED, Json(user)))
}
```

## Error Handling in Web
```rust
use axum::response::{IntoResponse, Response};

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self.0.downcast_ref::<MyError>() {
            Some(MyError::NotFound) => StatusCode::NOT_FOUND,
            Some(MyError::Unauthorized) => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({ "error": self.0.to_string() }))).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(err: E) -> Self { Self(err.into()) }
}
```

## Database with sqlx
```rust
// Compile-time checked query
let user = sqlx::query_as!(User,
    "SELECT id, name, email FROM users WHERE id = $1",
    user_id
)
.fetch_optional(&pool)
.await?;
```

## Middleware Patterns
- Request ID injection (`tower-http`)
- Rate limiting (`tower-governor`)
- Authentication extraction
- Request/response logging with `tracing`
- CORS configuration
- Compression (`tower-http::compression`)
