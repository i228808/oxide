use axum::{
    handler::Handler,
    routing::{delete, get, head, options, patch, post, put},
    Router,
};

/// HTTP methods supported by the framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

/// Thin wrapper around `axum::Router` that provides a simplified registration API
/// with support for grouping, nesting, and merging.
pub struct OxideRouter<S = ()> {
    inner: Router<S>,
}

impl<S: Clone + Send + Sync + 'static> Default for OxideRouter<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Clone + Send + Sync + 'static> OxideRouter<S> {
    pub fn new() -> Self {
        Self {
            inner: Router::new(),
        }
    }

    /// Wrap an existing `axum::Router` in an `OxideRouter`.
    pub fn from_router(router: Router<S>) -> Self {
        Self { inner: router }
    }

    /// Nest this router under a prefix, consuming and returning `self`.
    /// Handles empty and "/" prefixes natively via merge (Axum 0.8 compatibility).
    pub fn nest_self(self, prefix: &str) -> Self {
        if prefix.is_empty() || prefix == "/" {
            self
        } else {
            Self {
                inner: Router::new().nest(prefix, self.inner),
            }
        }
    }

    /// Register a handler for the given method and path.
    pub fn route<H, T>(mut self, method: Method, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        let method_router = match method {
            Method::GET => get(handler),
            Method::POST => post(handler),
            Method::PUT => put(handler),
            Method::DELETE => delete(handler),
            Method::PATCH => patch(handler),
            Method::HEAD => head(handler),
            Method::OPTIONS => options(handler),
        };
        self.inner = self.inner.route(path, method_router);
        self
    }

    // -- Convenience methods --------------------------------------------------

    pub fn get<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.route(Method::GET, path, handler)
    }

    pub fn post<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.route(Method::POST, path, handler)
    }

    pub fn put<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.route(Method::PUT, path, handler)
    }

    pub fn delete<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.route(Method::DELETE, path, handler)
    }

    pub fn patch<H, T>(self, path: &str, handler: H) -> Self
    where
        H: Handler<T, S>,
        T: 'static,
    {
        self.route(Method::PATCH, path, handler)
    }

    // -- Composition ----------------------------------------------------------

    /// Merge another `OxideRouter` into this one (flat merge, no prefix).
    pub fn merge(mut self, other: OxideRouter<S>) -> Self {
        self.inner = self.inner.merge(other.inner);
        self
    }

    /// Nest a sub-router under the given prefix. Handles empty and "/" prefixes gracefully.
    ///
    /// ```rust,ignore
    /// let api = OxideRouter::new()
    ///     .get("/users", list_users)
    ///     .post("/users", create_user);
    ///
    /// let app_router = OxideRouter::new()
    ///     .nest("/api", api);
    /// // produces: GET /api/users, POST /api/users
    /// ```
    pub fn nest(mut self, prefix: &str, other: OxideRouter<S>) -> Self {
        if prefix.is_empty() || prefix == "/" {
            self.inner = self.inner.merge(other.inner);
        } else {
            self.inner = self.inner.nest(prefix, other.inner);
        }
        self
    }

    /// Consume the wrapper and return the underlying `axum::Router`.
    pub fn into_inner(self) -> Router<S> {
        self.inner
    }
}
