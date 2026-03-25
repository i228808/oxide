use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::AppConfig;

/// Clone-able, type-safe map for storing arbitrary state by type.
#[derive(Clone, Default)]
pub struct TypeMap {
    map: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl TypeMap {
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.map.insert(TypeId::of::<T>(), Arc::new(value));
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|v| v.clone().downcast::<T>().ok())
    }
}

/// Shared application state injected into every request.
///
/// Contains:
/// - `config` — the loaded [`AppConfig`] (always present)
/// - User-provided state registered via [`App::state()`](crate::App::state)
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    extensions: Arc<TypeMap>,
}

impl AppState {
    pub(crate) fn new(config: AppConfig, extensions: TypeMap) -> Self {
        Self {
            config: Arc::new(config),
            extensions: Arc::new(extensions),
        }
    }

    /// Retrieve user-provided state by type.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.extensions.get::<T>()
    }
}
