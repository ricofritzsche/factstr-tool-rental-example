#![cfg_attr(test, allow(dead_code))]

use std::error::Error;
use std::fmt::{Display, Formatter};
#[cfg(test)]
use std::process;
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
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
    inner: Arc<Mutex<Box<dyn EventStore + Send>>>,
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

        Ok(Self::from_event_store(store))
    }

    pub fn from_event_store<T>(store: T) -> Self
    where
        T: EventStore + Send + 'static,
    {
        Self {
            inner: Arc::new(Mutex::new(Box::new(store))),
        }
    }

    pub async fn check_connectivity(&self) -> Result<(), StoreError> {
        let store = Arc::clone(&self.inner);

        task::spawn_blocking(move || {
            let store = store.lock().map_err(|_| EventStoreError::BackendFailure {
                message: "application store lock poisoned".to_owned(),
            })?;

            store.query(&EventQuery::all())
        })
        .await
        .map_err(StoreError::Join)?
        .map(|_| ())
        .map_err(StoreError::Factstr)
    }

    fn lock_store(&self) -> Result<MutexGuard<'_, Box<dyn EventStore + Send>>, EventStoreError> {
        self.inner
            .lock()
            .map_err(|_| EventStoreError::BackendFailure {
                message: "application store lock poisoned".to_owned(),
            })
    }

    fn run_store_operation<R, F>(&self, operation: F) -> Result<R, EventStoreError>
    where
        R: Send,
        F: FnOnce(&mut (dyn EventStore + Send)) -> Result<R, EventStoreError> + Send,
    {
        if tokio::runtime::Handle::try_current().is_ok() {
            let inner = Arc::clone(&self.inner);

            thread::scope(|scope| {
                scope
                    .spawn(move || {
                        let mut store =
                            inner.lock().map_err(|_| EventStoreError::BackendFailure {
                                message: "application store lock poisoned".to_owned(),
                            })?;

                        operation(store.as_mut())
                    })
                    .join()
                    .map_err(|_| EventStoreError::BackendFailure {
                        message: "application store worker thread panicked".to_owned(),
                    })?
            })
        } else {
            let mut store = self.lock_store()?;
            operation(store.as_mut())
        }
    }
}

impl EventStore for AppStore {
    fn query(&self, event_query: &EventQuery) -> Result<factstr::QueryResult, EventStoreError> {
        self.run_store_operation(|store| store.query(event_query))
    }

    fn append(
        &self,
        new_events: Vec<factstr::NewEvent>,
    ) -> Result<factstr::AppendResult, EventStoreError> {
        self.run_store_operation(|store| store.append(new_events))
    }

    fn append_if(
        &self,
        new_events: Vec<factstr::NewEvent>,
        context_query: &EventQuery,
        expected_context_version: Option<u64>,
    ) -> Result<factstr::AppendResult, EventStoreError> {
        self.run_store_operation(|store| {
            store.append_if(new_events, context_query, expected_context_version)
        })
    }

    fn stream_all(
        &self,
        handle: factstr::HandleStream,
    ) -> Result<factstr::EventStream, EventStoreError> {
        self.run_store_operation(|store| store.stream_all(handle))
    }

    fn stream_to(
        &self,
        event_query: &EventQuery,
        handle: factstr::HandleStream,
    ) -> Result<factstr::EventStream, EventStoreError> {
        self.run_store_operation(|store| store.stream_to(event_query, handle))
    }

    fn stream_all_durable(
        &self,
        durable_stream: &factstr::DurableStream,
        handle: factstr::HandleStream,
    ) -> Result<factstr::EventStream, EventStoreError> {
        self.run_store_operation(|store| store.stream_all_durable(durable_stream, handle))
    }

    fn stream_to_durable(
        &self,
        durable_stream: &factstr::DurableStream,
        event_query: &EventQuery,
        handle: factstr::HandleStream,
    ) -> Result<factstr::EventStream, EventStoreError> {
        self.run_store_operation(|store| {
            store.stream_to_durable(durable_stream, event_query, handle)
        })
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
