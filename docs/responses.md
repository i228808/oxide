# Responses

Oxide provides `ApiResponse<T>` — a standardized response type that wraps all handler output in a consistent JSON envelope.

This page reflects current behavior in `oxide-framework-core/src/response.rs`.

## JSON Envelope Format

### Success

```json
{
  "status": 200,
  "data": { ... }
}
```

The `data` field contains whatever `T` your handler returns (must implement `Serialize`).

### Error

```json
{
  "status": 404,
  "error": "resource not found"
}
```

The `error` field is always a string message.

## Using `ApiResponse<T>`

Import it and use it as your handler return type:

```rust
use oxide_framework_core::ApiResponse;
use serde::Serialize;

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
}

async fn get_user() -> ApiResponse<User> {
    ApiResponse::ok(User { id: 1, name: "Alice".into() })
}
```

Response:

```
HTTP/1.1 200 OK
Content-Type: application/json

{"status":200,"data":{"id":1,"name":"Alice"}}
```

## Factory Methods

### Success Responses

| Method | HTTP Status | Usage |
|---|---|---|
| `ApiResponse::ok(data)` | 200 OK | Standard success |
| `ApiResponse::created(data)` | 201 Created | After creating a resource |
| `ApiResponse::success(status, data)` | Custom | Any success status code |

```rust
// 200 OK
ApiResponse::ok(user)

// 201 Created
ApiResponse::created(new_user)

// Custom success status
use oxide_framework_core::StatusCode;
ApiResponse::success(StatusCode::ACCEPTED, job)
```

### Error Responses

| Method | HTTP Status | Usage |
|---|---|---|
| `ApiResponse::bad_request(msg)` | 400 | Validation failures |
| `ApiResponse::unauthorized(msg)` | 401 | Missing/invalid authentication |
| `ApiResponse::forbidden(msg)` | 403 | Authenticated but not allowed |
| `ApiResponse::not_found(msg)` | 404 | Resource doesn't exist |
| `ApiResponse::internal_error(msg)` | 500 | Unexpected server errors |
| `ApiResponse::error(status, msg)` | Custom | Any error status code |

```rust
// 400 Bad Request
ApiResponse::bad_request("name is required")

// 404 Not Found
ApiResponse::not_found("user not found")

// 401 Unauthorized
ApiResponse::unauthorized("login required")

// 403 Forbidden
ApiResponse::forbidden("insufficient permissions")

// 500 Internal Server Error
ApiResponse::internal_error("something went wrong")

// Custom error status
use oxide_framework_core::StatusCode;
ApiResponse::error(StatusCode::CONFLICT, "username already taken")
```

## Mixing Success and Error Returns

Because `ApiResponse<T>` is an enum, you can return either variant from the same handler:

```rust
async fn get_user(Path(id): Path<u64>) -> ApiResponse<User> {
    if id == 0 {
        return ApiResponse::not_found("user not found");
    }

    let user = User { id, name: format!("User#{id}") };
    ApiResponse::ok(user)
}
```

For `id = 5`:
```json
{"status": 200, "data": {"id": 5, "name": "User#5"}}
```

For `id = 0`:
```json
{"status": 404, "error": "user not found"}
```

## Extracting JSON Request Bodies

Use the re-exported `Json` extractor to parse incoming JSON:

```rust
use oxide_framework_core::Json;
use serde::Deserialize;

#[derive(Deserialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

async fn create_user(Json(body): Json<CreateUserRequest>) -> ApiResponse<User> {
    let user = User {
        id: 42,
        name: body.name,
    };
    ApiResponse::created(user)
}
```

## Raw Responses

You're not forced to use `ApiResponse`. Handlers can return any type that implements Axum's `IntoResponse`:

```rust
// Plain string
async fn ping() -> &'static str {
    "pong"
}

// Status code + string
async fn no_content() -> StatusCode {
    StatusCode::NO_CONTENT
}
```

However, `ApiResponse<T>` is the recommended pattern for API endpoints — it ensures every consumer gets a predictable JSON shape.

## Envelope Types (Advanced)

If you need to reference the envelope types directly (for testing, documentation generators, etc.):

The success body is:

```rust
pub struct SuccessBody<T: Serialize> {
    pub status: u16,
    pub data: T,
}
```

The error body is:

```rust
pub struct ErrorBody {
    pub status: u16,
    pub error: String,
}
```

Both are serialized to JSON automatically by the `IntoResponse` implementation
on `ApiResponse<T>`.

## Notes

- `ApiResponse<T>` requires `T: serde::Serialize`.
- The `status` value in the JSON body mirrors the HTTP status code.
- You can still return any Axum `IntoResponse` type directly when needed.

