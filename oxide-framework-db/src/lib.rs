pub use sqlx;
pub use sqlx::{Database, MySql, Postgres, Sqlite};

use async_trait::async_trait;
use oxide_framework_core::{App, FrameworkError, ReadinessCheck};

/// A type alias for injecting Database pools via the DI system.
pub type DbPool<D> = sqlx::Pool<D>;

/// Connection strategy used during app bootstrap.
#[derive(Clone, Copy, Debug)]
pub enum ConnectMode {
    /// Build pool lazily; first query validates connectivity.
    Lazy,
    /// Verify connectivity up-front via readiness check.
    Strict,
}

#[derive(Clone)]
struct SqlxReadyCheck<D: sqlx::Database> {
    pool: DbPool<D>,
    label: &'static str,
}

#[async_trait]
impl<D> ReadinessCheck for SqlxReadyCheck<D>
where
    D: sqlx::Database,
    DbPool<D>: Send + Sync,
{
    fn name(&self) -> &'static str {
        self.label
    }

    async fn check(&self) -> Result<(), FrameworkError> {
        self.pool
            .acquire()
            .await
            .map(|_| ())
            .map_err(|e| FrameworkError::ReadinessFailed {
                check: self.label,
                message: e.to_string(),
            })
    }
}

/// Extension trait for `App` to enable SQL database injection.
pub trait AppDbExt {
    /// Register a SQL pool using lazy-connect mode.
    fn database<D: sqlx::Database>(
        self,
        url: &str,
        opts: impl FnOnce(sqlx::pool::PoolOptions<D>) -> sqlx::pool::PoolOptions<D>,
    ) -> Self;

    /// Register a SQL pool with explicit connect mode.
    fn database_with_mode<D: sqlx::Database>(
        self,
        url: &str,
        mode: ConnectMode,
        opts: impl FnOnce(sqlx::pool::PoolOptions<D>) -> sqlx::pool::PoolOptions<D>,
    ) -> Self;
}

impl AppDbExt for App {
    fn database<D: sqlx::Database>(
        self,
        url: &str,
        opts: impl FnOnce(sqlx::pool::PoolOptions<D>) -> sqlx::pool::PoolOptions<D>,
    ) -> Self {
        self.database_with_mode(url, ConnectMode::Lazy, opts)
    }

    fn database_with_mode<D: sqlx::Database>(
        self,
        url: &str,
        mode: ConnectMode,
        opts: impl FnOnce(sqlx::pool::PoolOptions<D>) -> sqlx::pool::PoolOptions<D>,
    ) -> Self {
        let options = opts(sqlx::pool::PoolOptions::<D>::new());
        let pool = options
            .connect_lazy(url)
            .expect("Failed to initialize database pool");

        let app = self.state(pool.clone());
        match mode {
            ConnectMode::Lazy => app,
            ConnectMode::Strict => app.readiness_check(SqlxReadyCheck {
                pool,
                label: std::any::type_name::<D>(),
            }),
        }
    }
}
