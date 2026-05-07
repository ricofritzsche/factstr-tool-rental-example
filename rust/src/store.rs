#![cfg_attr(test, allow(dead_code))]

use std::error::Error;
use std::fmt::{Display, Formatter};
#[cfg(test)]
use std::process;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(test)]
use std::time::{SystemTime, UNIX_EPOCH};

use factstr::{EventQuery, EventStore, EventStoreError};
use factstr_postgres::{PostgresBootstrapOptions, PostgresStore};
#[cfg(test)]
use sqlx::{Connection, Executor, PgConnection};
use tokio::task;
use tracing::info;

use crate::config::AppConfig;

#[cfg(test)]
static NEXT_TEST_DATABASE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct AppStore {
    inner: Arc<PostgresStore>,
}

impl AppStore {
    pub async fn initialize(config: &AppConfig) -> Result<Self, StoreError> {
        info!(
            database_name = %config.database_name,
            "bootstrapping FACTSTR PostgreSQL store"
        );

        Self::bootstrap(&config.postgres_admin_url, &config.database_name).await
    }

    pub async fn bootstrap(server_url: &str, database_name: &str) -> Result<Self, StoreError> {
        validate_database_name(database_name)?;

        let options = PostgresBootstrapOptions {
            server_url: server_url.to_owned(),
            database_name: database_name.to_owned(),
        };

        let store = task::spawn_blocking(move || PostgresStore::bootstrap(options))
            .await
            .map_err(StoreError::Join)?
            .map_err(StoreError::Factstr)?;

        Ok(Self {
            inner: Arc::new(store),
        })
    }

    pub async fn check_connectivity(&self) -> Result<(), StoreError> {
        let store = Arc::clone(&self.inner);

        task::spawn_blocking(move || store.query(&EventQuery::all()))
            .await
            .map_err(StoreError::Join)?
            .map(|_| ())
            .map_err(StoreError::Factstr)
    }
}

#[cfg(test)]
pub struct TestDatabase {
    admin_url: String,
    database_name: String,
    cleaned_up: bool,
}

#[cfg(test)]
impl TestDatabase {
    pub async fn create(admin_url: &str) -> Result<Self, StoreError> {
        let database_name = format!("factstr_tool_rental_test_{}", unique_test_database_suffix());
        validate_database_name(&database_name)?;

        Ok(Self {
            admin_url: admin_url.to_owned(),
            database_name,
            cleaned_up: false,
        })
    }

    pub fn database_name(&self) -> &str {
        &self.database_name
    }

    pub async fn open_store(&self) -> Result<AppStore, StoreError> {
        AppStore::bootstrap(&self.admin_url, &self.database_name).await
    }

    pub async fn cleanup(&mut self) -> Result<(), StoreError> {
        if self.cleaned_up {
            return Ok(());
        }

        drop_database(&self.admin_url, &self.database_name).await?;
        self.cleaned_up = true;
        Ok(())
    }
}

#[cfg(test)]
impl Drop for TestDatabase {
    fn drop(&mut self) {
        if self.cleaned_up {
            return;
        }

        let admin_url = self.admin_url.clone();
        let database_name = self.database_name.clone();

        let _ = std::thread::Builder::new()
            .name("factstr-tool-rental-test-db-cleanup".to_owned())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();

                if let Ok(runtime) = runtime {
                    let _ = runtime.block_on(async move {
                        let _ = drop_database(&admin_url, &database_name).await;
                    });
                }
            })
            .and_then(|handle| {
                handle
                    .join()
                    .map_err(|_| std::io::Error::other("join failed"))
            });
    }
}

#[cfg(test)]
pub async fn database_exists(admin_url: &str, database_name: &str) -> Result<bool, StoreError> {
    validate_database_name(database_name)?;

    let mut connection = PgConnection::connect(admin_url)
        .await
        .map_err(StoreError::Sqlx)?;

    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)",
    )
    .bind(database_name)
    .fetch_one(&mut connection)
    .await
    .map_err(StoreError::Sqlx)?;

    Ok(exists)
}

#[cfg(test)]
pub async fn drop_database(admin_url: &str, database_name: &str) -> Result<(), StoreError> {
    validate_database_name(database_name)?;

    info!(database_name = %database_name, "dropping PostgreSQL database");

    let mut connection = PgConnection::connect(admin_url)
        .await
        .map_err(StoreError::Sqlx)?;

    sqlx::query(
        "SELECT pg_terminate_backend(pid)
         FROM pg_stat_activity
         WHERE datname = $1 AND pid <> pg_backend_pid()",
    )
    .bind(database_name)
    .execute(&mut connection)
    .await
    .map_err(StoreError::Sqlx)?;

    let drop_statement = format!("DROP DATABASE IF EXISTS \"{database_name}\"");
    connection
        .execute(drop_statement.as_str())
        .await
        .map_err(StoreError::Sqlx)?;

    Ok(())
}

fn validate_database_name(database_name: &str) -> Result<(), StoreError> {
    let valid = !database_name.is_empty()
        && database_name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && database_name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');

    if valid {
        Ok(())
    } else {
        Err(StoreError::InvalidDatabaseName(database_name.to_owned()))
    }
}

#[cfg(test)]
fn unique_test_database_suffix() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let next_id = NEXT_TEST_DATABASE_ID.fetch_add(1, Ordering::Relaxed);

    format!("{timestamp}_{}_{}", process::id(), next_id)
}

#[derive(Debug)]
pub enum StoreError {
    InvalidDatabaseName(String),
    Factstr(EventStoreError),
    Join(task::JoinError),
    #[cfg(test)]
    Sqlx(sqlx::Error),
}

impl Display for StoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDatabaseName(name) => write!(
                formatter,
                "invalid PostgreSQL database name '{name}': use lowercase ASCII letters, digits, and underscores, starting with a letter"
            ),
            Self::Factstr(error) => write!(formatter, "FACTSTR store error: {error}"),
            Self::Join(error) => write!(formatter, "task join error: {error}"),
            #[cfg(test)]
            Self::Sqlx(error) => write!(formatter, "PostgreSQL admin error: {error}"),
        }
    }
}

impl Error for StoreError {}
