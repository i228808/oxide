use std::sync::Arc;

use axum::Router;

use crate::router::OxideRouter;
use crate::state::AppState;

/// Trait implemented by `#[controller]`-annotated types.
///
/// You don't implement this manually — the `#[controller("/prefix")]` macro
/// generates the implementation for you.
///
/// # Generated methods
///
/// - `from_state` — constructs the controller, extracting dependencies from
///   [`AppState`]. Panics with a clear message at startup if a dependency is
///   missing.
/// - `register` — returns an [`OxideRouter`] with all route methods registered.
///   Methods that take `&self` are wrapped in closures that capture `Arc<Self>`.
pub trait Controller: Send + Sync + Sized + 'static {
    /// URL prefix for all routes in this controller (e.g. `"/api/users"`).
    const PREFIX: &'static str;

    /// Construct the controller from application state.
    fn from_state(state: &AppState) -> Self;

    /// Register all route methods on a fresh router.
    fn register(self: Arc<Self>) -> OxideRouter;

    /// Override to apply controller-scoped middleware (auth, logging, etc.).
    ///
    /// The router passed in already contains all of this controller's routes.
    /// Return the router with additional layers applied. The default is a
    /// no-op (no extra middleware).
    ///
    /// If the `#[controller]` macro finds a `fn middleware(router: Router) -> Router`
    /// method in the impl block, it generates this override automatically.
    fn configure_router(router: Router) -> Router {
        router
    }
}
