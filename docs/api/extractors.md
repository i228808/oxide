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
