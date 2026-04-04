# API: Extractors

Reference for extractors in `oxide-framework-core/src/extract.rs`.

## `Config`

- Extracts `Arc<AppConfig>` from `AppState`.
- Use for host/port/app metadata at runtime.

## `Data<T>`

- Extracts app singleton `Arc<T>`.
- Backed by values registered through `App::state(...)`.
- Missing registration returns HTTP 500 rejection.

## `Inject<T>`

- Alias of `Data<T>`.
- Use when constructor/handler signatures read better as dependency injection.

## `Scoped<T>`

- Extracts request-scoped `T` from request extensions.
- Backed by `App::scoped_state(...)` factories.
- Factory executes once per request.
- Missing scoped value returns HTTP 500 rejection.

## `RequestId`

- Extracts request correlation id set by framework middleware.
- Default header name is `x-request-id` (configurable via `App::request_id_header(...)`).

## `Validated<T>`

- Parses JSON body into `T` and runs `validator::Validate`.
- Requires `T: serde::de::DeserializeOwned + validator::Validate`.
- Validation failures return 400 with code `validation_error` and structured details.
