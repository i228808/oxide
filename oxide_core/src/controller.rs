use std::sync::Arc;

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
}
