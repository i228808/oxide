use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::extract::Request;
use axum::handler::Handler;
use axum::http::HeaderName;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Router;
use tokio::net::TcpListener;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::config::AppConfig;
use crate::controller::Controller;
use crate::error::FrameworkError;
use crate::logging;
use crate::auth::{AuthConfig, AuthLayer};
use crate::middleware::{self, InjectStateLayer, RequestIdConfig, RequestTimeoutLayer};
use crate::rate_limit::RateLimitLayer;
use crate::router::{Method, OxideRouter};
use crate::state::{AppState, TypeMap};

type RouterTransform = Box<dyn FnOnce(Router) -> Router>;

#[derive(Clone, Copy)]
pub struct HealthOptions {
    pub enabled: bool,
}

impl Default for HealthOptions {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[async_trait]
pub trait ReadinessCheck: Send + Sync {
    fn name(&self) -> &'static str;
    async fn check(&self) -> Result<(), FrameworkError>;
}

/// Primary entry point for building an Oxide application.
///
/// Uses a builder pattern to configure routes, state, middleware, and then
/// start the server.
///
/// # Example
///
/// ```rust,no_run
/// use oxide_framework_core::{App, ApiResponse, Config};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Msg { text: String }
///
/// async fn index(Config(cfg): Config) -> ApiResponse<Msg> {
///     ApiResponse::ok(Msg { text: format!("Hello from {}!", cfg.app_name) })
/// }
///
/// fn main() {
///     App::new()
///         .config("app.yaml")
///         .rate_limit(100, 60)
///         .cors_permissive()
///         .request_timeout(30)
///         .get("/", index)
///         .run();
/// }
/// ```
pub struct App {
    config: AppConfig,
    router: OxideRouter,
    config_path: Option<String>,
    type_map: TypeMap,
    request_logging: bool,
    rate_limit: Option<(u64, Duration)>,
    cors: Option<CorsLayer>,
    request_timeout: Option<Duration>,
    controller_factories: Vec<Box<dyn FnOnce(AppState) -> OxideRouter>>,
    /// User-registered middleware (before/after hooks, custom layers).
    /// Applied between State injection and CatchPanic (can access state,
    /// panics are still caught).
    user_layers: Vec<RouterTransform>,
    /// Optional JWT / session-cookie auth (runs after user hooks, before state injection).
    auth: Option<AuthConfig>,
    request_id: RequestIdConfig,
    health: HealthOptions,
    readiness_checks: Vec<Arc<dyn ReadinessCheck>>,
}

impl App {
    /// Create a new `App` with default configuration.
    ///
    /// Initialises structured logging on first call.
    pub fn new() -> Self {
        logging::init();

        Self {
            config: AppConfig::default(),
            router: OxideRouter::new(),
            config_path: None,
            type_map: TypeMap::default(),
            request_logging: true,
            rate_limit: None,
            cors: None,
            request_timeout: None,
            controller_factories: Vec::new(),
            user_layers: Vec::new(),
            auth: None,
            request_id: RequestIdConfig {
                header_name: HeaderName::from_static("x-request-id"),
                include_response_header: true,
            },
            health: HealthOptions::default(),
            readiness_checks: Vec::new(),
        }
    }

    // -- Configuration --------------------------------------------------------

    /// Point the application at a YAML config file.
    /// Config is loaded (and merged with env vars) when `.run()` is called.
    pub fn config(mut self, path: &str) -> Self {
        self.config_path = Some(path.to_string());
        self
    }

    // -- State ----------------------------------------------------------------

    /// Register a shared value accessible in handlers via the [`Data<T>`](crate::Data) extractor.
    pub fn state<T: Send + Sync + 'static>(mut self, value: T) -> Self {
        self.type_map.insert(value);
        self
    }

    // -- Generic route registration -------------------------------------------

    /// Register a route for the given method and path.
    pub fn route<H, T>(mut self, method: Method, path: &str, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.router = self.router.route(method, path, handler);
        self
    }

    // -- Convenience methods --------------------------------------------------

    pub fn get<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.router = self.router.get(path, handler);
        self
    }

    pub fn post<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.router = self.router.post(path, handler);
        self
    }

    pub fn put<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.router = self.router.put(path, handler);
        self
    }

    pub fn delete<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.router = self.router.delete(path, handler);
        self
    }

    pub fn patch<H, T>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler<T, ()>,
        T: 'static,
    {
        self.router = self.router.patch(path, handler);
        self
    }

    // -- Controller registration ----------------------------------------------

    /// Register a `#[controller]`-annotated struct.
    ///
    /// At startup the framework will:
    /// 1. Construct the controller via `C::from_state(&app_state)`.
    /// 2. Call `C::register(Arc::new(instance))` to build its routes.
    /// 3. Nest those routes under `C::PREFIX`.
    ///
    /// Dependencies are resolved eagerly — a missing `Data<T>` will panic at
    /// startup, not at request time (fail-fast).
    pub fn controller<C: Controller>(mut self) -> Self {
        self.controller_factories.push(Box::new(|state: AppState| {
            let instance = Arc::new(C::from_state(&state));
            let routes = C::register(instance);
            let inner = C::configure_router(routes.into_inner());
            OxideRouter::from_router(inner).nest_self(C::PREFIX)
        }));
        self
    }

    // -- Router composition ---------------------------------------------------

    /// Merge a pre-built `OxideRouter` into the application (flat, no prefix).
    pub fn routes(mut self, router: OxideRouter) -> Self {
        self.router = self.router.merge(router);
        self
    }

    /// Nest a pre-built `OxideRouter` under the given path prefix.
    pub fn nest(mut self, prefix: &str, router: OxideRouter) -> Self {
        self.router = self.router.nest(prefix, router);
        self
    }

    // -- Scalability & production middleware -----------------------------------

    /// Enable per-IP rate limiting.
    ///
    /// Returns HTTP 429 with `Retry-After` header when the limit is exceeded.
    pub fn rate_limit(mut self, max_requests: u64, window_secs: u64) -> Self {
        self.rate_limit = Some((max_requests, Duration::from_secs(window_secs)));
        self
    }

    /// Enable permissive CORS (allow any origin, method, and header).
    pub fn cors_permissive(mut self) -> Self {
        self.cors = Some(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );
        self
    }

    /// Enable CORS with a specific set of allowed origins.
    pub fn cors_origins<I, S>(mut self, origins: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let origins: Vec<_> = origins
            .into_iter()
            .filter_map(|o| o.as_ref().parse().ok())
            .collect();

        self.cors = Some(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods(Any)
                .allow_headers(Any),
        );
        self
    }

    /// Set a maximum duration for request processing.
    pub fn request_timeout(mut self, secs: u64) -> Self {
        self.request_timeout = Some(Duration::from_secs(secs));
        self
    }

    /// Disable the built-in per-request logging middleware.
    pub fn disable_request_logging(mut self) -> Self {
        self.request_logging = false;
        self
    }

    /// Set request id header name used for correlation extraction/propagation.
    pub fn request_id_header(mut self, header: &str) -> Self {
        if let Ok(name) = header.parse::<HeaderName>() {
            self.request_id.header_name = name;
        }
        self
    }

    /// Disable writing request id back to response headers.
    pub fn disable_response_request_id_header(mut self) -> Self {
        self.request_id.include_response_header = false;
        self
    }

    /// Disable default `/health/live` and `/health/ready` routes.
    pub fn disable_default_health_routes(mut self) -> Self {
        self.health.enabled = false;
        self
    }

    /// Register a readiness check for `/health/ready`.
    pub fn readiness_check<C>(mut self, check: C) -> Self
    where
        C: ReadinessCheck + 'static,
    {
        self.readiness_checks.push(Arc::new(check));
        self
    }

    /// Enable JWT authentication from `Authorization: Bearer` and/or a session cookie.
    ///
    /// Inserts [`crate::auth::AuthClaims`] into request extensions when the token is valid.
    /// Invalid or expired tokens return **401** before your handler runs.
    ///
    /// Relative to state and hooks: application state is injected first, then JWT is validated, then
    /// [`App::before`](Self::before) / [`App::layer`](Self::layer) hooks, then the route handler.
    pub fn auth(mut self, config: AuthConfig) -> Self {
        assert!(
            !config.secret.is_empty(),
            "AuthConfig.secret must not be empty"
        );
        self.auth = Some(config);
        self
    }

    // -- Lifecycle hooks & custom middleware -----------------------------------

    /// Register a "before" hook that runs on every request.
    ///
    /// The hook receives the request and a [`Next`] handle, and must produce a
    /// response. Use it for auth checks, request mutation, short-circuit
    /// responses, etc.
    ///
    /// ```rust,ignore
    /// app.before(|req: Request, next: Next| async move {
    ///     println!("incoming: {} {}", req.method(), req.uri());
    ///     next.run(req).await
    /// })
    /// ```
    pub fn before<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Request, Next) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.user_layers.push(Box::new(move |router: Router| {
            router.layer(axum::middleware::from_fn(f))
        }));
        self
    }

    /// Register a request-scoped dependency factory.
    /// 
    /// The factory closure is called on *every* incoming request, and its output
    /// is automatically injected into the request extensions, making it available
    /// to handlers via the `Scoped<T>` extractor.
    pub fn scoped_state<F, Fut, T>(mut self, factory: F) -> Self
    where
        F: Fn(&axum::http::request::Parts) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = T> + Send + 'static,
        T: Clone + Send + Sync + 'static,
    {
        let factory = Arc::new(factory);
        self.user_layers.push(Box::new(move |router: Router| {
            let f = factory.clone();
            router.layer(axum::middleware::from_fn(move |req: Request, next: Next| {
                let f = f.clone();
                async move {
                    let (mut parts, body) = req.into_parts();
                    let val = f(&parts).await;
                    parts.extensions.insert(val);
                    let req = axum::extract::Request::from_parts(parts, body);
                    next.run(req).await
                }
            }))
        }));
        self
    }

    /// Register an "after" hook that can transform every outgoing response.
    ///
    /// ```rust,ignore
    /// app.after(|mut res: Response| async move {
    ///     res.headers_mut().insert("X-Powered-By", "Oxide".parse().unwrap());
    ///     res
    /// })
    /// ```
    pub fn after<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Response) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        self.user_layers.push(Box::new(move |router: Router| {
            router.layer(axum::middleware::map_response(f))
        }));
        self
    }

    /// Add an arbitrary Tower `Layer` to the middleware stack.
    ///
    /// The layer is positioned between state injection and the panic catcher,
    /// so it has access to `AppState` and any panics it causes are caught.
    pub fn layer<L>(mut self, layer: L) -> Self
    where
        L: tower::Layer<axum::routing::Route> + Clone + Send + Sync + 'static,
        L::Service: tower::Service<Request, Response = Response, Error = std::convert::Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        <L::Service as tower::Service<Request>>::Future: Send + 'static,
    {
        self.user_layers.push(Box::new(move |router: Router| {
            router.layer(layer)
        }));
        self
    }

    // -- Internal: build the layered router -----------------------------------

    fn build_router(self, config: AppConfig) -> (Router, AppState) {
        let app_state = AppState::new(config, self.type_map);

        let mut base = self.router;
        for factory in self.controller_factories {
            let ctrl_router = factory(app_state.clone());
            base = base.merge(ctrl_router);
        }
        let mut router = base.into_inner();

        if self.health.enabled {
            let checks = self.readiness_checks.clone();
            router = router
                .route("/health/live", axum::routing::get(health_live))
                .route(
                    "/health/ready",
                    axum::routing::get(move || {
                        let checks = checks.clone();
                        async move { health_ready(checks).await }
                    }),
                );
        }

        // Layer application order: first applied = innermost = closest to the route handler.
        // Request flow (outer → inner): Logger → CORS → Timeout → RateLimit → CatchPanic →
        // InjectState → JwtAuth → UserHooks → Route handler
        //
        // User hooks run after JWT validation so `OptionalAuth` / [`AuthClaims`] are visible in `before` / custom layers.

        // 1. User-registered hooks / layers (innermost)
        for transform in self.user_layers {
            router = transform(router);
        }

        // 2. JWT / session cookie auth
        if let Some(auth_cfg) = self.auth {
            router = router.layer(AuthLayer::new(auth_cfg));
        }

        // 3. State injection
        router = router.layer(InjectStateLayer::new(app_state.clone()));

        // 4. Panic recovery — catches panics in hooks AND handlers
        router = router.layer(CatchPanicLayer::custom(middleware::panic_json_response));

        // 5. Rate limiting
        if let Some((max, window)) = self.rate_limit {
            router = router.layer(RateLimitLayer::new(max, window));
        }

        // 6. Request timeout
        if let Some(timeout) = self.request_timeout {
            router = router.layer(RequestTimeoutLayer::new(timeout));
        }

        // 7. CORS (wraps everything — headers on ALL responses including 429/408/500)
        if let Some(cors) = self.cors {
            router = router.layer(cors);
        }

        // 8. Request logging (outermost)
        if self.request_logging {
            router = router.layer(axum::middleware::from_fn(middleware::request_logger));
        }

        let request_id_cfg = self.request_id.clone();
        router = router.layer(axum::middleware::from_fn(move |req, next| {
            let cfg = request_id_cfg.clone();
            async move { middleware::request_id_middleware(cfg, req, next).await }
        }));

        (router, app_state)
    }

    // -- Server lifecycle -----------------------------------------------------

    /// Build and start the HTTP server. Blocks the current thread, creating a new Tokio runtime.
    pub fn run(self) {
        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        rt.block_on(self.serve());
    }

    /// Build and start the HTTP server using the current Tokio runtime.
    pub async fn serve(mut self) {
        self.config = AppConfig::load(self.config_path.as_deref());

        let addr = format!("{}:{}", self.config.host, self.config.port);
        let app_name = if self.config.app_name.is_empty() {
            "oxide-app".to_string()
        } else {
            self.config.app_name.clone()
        };

        let config = self.config.clone();
        let (router, _state) = self.build_router(config);

        let listener = TcpListener::bind(&addr)
            .await
            .unwrap_or_else(|e| panic!("failed to bind to {addr}: {e}"));

        info!(
            name = %app_name,
            address = %addr,
            "Oxide server started"
        );

        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");

        info!("Oxide server shut down gracefully");
    }

    // -- Testing --------------------------------------------------------------

    /// Start the server on a random port for integration testing.
    ///
    /// Returns a [`TestServer`] with the bound address. The server runs in a
    /// background tokio task and is stopped when the `TestServer` is dropped.
    pub async fn into_test_server(self) -> TestServer {
        let config = self.config.clone();
        let (router, _state) = self.build_router(config);

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind test server");
        let addr = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .ok();
        });

        TestServer { addr, handle }
    }
}

async fn health_live() -> axum::response::Response {
    crate::ApiResponse::ok(serde_json::json!({ "status": "live" })).into_response()
}

async fn health_ready(
    checks: Vec<Arc<dyn ReadinessCheck>>,
) -> axum::response::Response {
    let mut failures = Vec::new();
    for check in checks {
        if let Err(err) = check.check().await {
            failures.push(serde_json::json!({
                "check": check.name(),
                "error": err.to_string(),
                "code": err.code(),
            }));
        }
    }

    if failures.is_empty() {
        crate::ApiResponse::ok(serde_json::json!({ "status": "ready" })).into_response()
    } else {
        (
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(serde_json::json!({
                "status": axum::http::StatusCode::SERVICE_UNAVAILABLE.as_u16(),
                "error": "readiness check failed",
                "code": "readiness_failed",
                "failures": failures,
            })),
        )
            .into_response()
    }
}

/// A running test server bound to a random port.
///
/// The server is automatically stopped when this value is dropped.
pub struct TestServer {
    addr: SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Build a full URL for the given path.
    pub fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => info!("received Ctrl+C, shutting down…"),
            _ = sigterm.recv() => info!("received SIGTERM, shutting down…"),
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.expect("failed to listen for Ctrl+C");
        info!("received Ctrl+C, shutting down…");
    }
}

