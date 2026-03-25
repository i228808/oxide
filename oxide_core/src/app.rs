use axum::handler::Handler;
use axum::Router;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::config::AppConfig;
use crate::logging;
use crate::middleware::{self, InjectStateLayer, RequestTimeoutLayer};
use crate::rate_limit::RateLimitLayer;
use crate::router::{Method, OxideRouter};
use crate::state::{AppState, TypeMap};

/// Primary entry point for building an Oxide application.
///
/// Uses a builder pattern to configure routes, state, middleware, and then
/// start the server.
///
/// # Example
///
/// ```rust,no_run
/// use oxide_core::{App, ApiResponse, Config};
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

    // -- Internal: build the layered router -----------------------------------

    fn build_router(self, config: AppConfig) -> (Router, AppState) {
        let app_state = AppState::new(config, self.type_map);
        let mut router = self.router.into_inner();

        // Layer order (innermost → outermost):
        //   Handler ← State ← CatchPanic ← RateLimit ← Timeout ← CORS ← Logger
        //
        // - State injection is innermost so handlers can access AppState
        // - CatchPanic wraps the handler so panics become 500 (not connection reset)
        // - Rate limiting is outside CatchPanic (panicked requests still count)
        // - Timeout wraps the processing pipeline
        // - CORS is outer to rate limit + timeout so ALL responses get CORS headers
        //   and OPTIONS preflight is handled before reaching the rate limiter
        // - Logger is outermost to capture total latency including all middleware

        // 1. State injection (innermost)
        router = router.layer(InjectStateLayer::new(app_state.clone()));

        // 2. Panic recovery — handler panics become JSON 500 responses
        router = router.layer(CatchPanicLayer::custom(middleware::panic_json_response));

        // 3. Rate limiting
        if let Some((max, window)) = self.rate_limit {
            router = router.layer(RateLimitLayer::new(max, window));
        }

        // 4. Request timeout
        if let Some(timeout) = self.request_timeout {
            router = router.layer(RequestTimeoutLayer::new(timeout));
        }

        // 5. CORS (wraps everything — headers on ALL responses including 429/408/500)
        if let Some(cors) = self.cors {
            router = router.layer(cors);
        }

        // 6. Request logging (outermost)
        if self.request_logging {
            router = router.layer(axum::middleware::from_fn(middleware::request_logger));
        }

        (router, app_state)
    }

    // -- Server lifecycle -----------------------------------------------------

    /// Build and start the HTTP server. Blocks the current thread.
    pub fn run(mut self) {
        self.config = AppConfig::load(self.config_path.as_deref());

        let addr = format!("{}:{}", self.config.host, self.config.port);
        let app_name = if self.config.app_name.is_empty() {
            "oxide-app".to_string()
        } else {
            self.config.app_name.clone()
        };

        let config = self.config.clone();
        let (router, _state) = self.build_router(config);

        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

        rt.block_on(async move {
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
        });
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
