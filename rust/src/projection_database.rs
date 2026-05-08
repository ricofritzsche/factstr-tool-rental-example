use std::fmt::{Display, Formatter};
use std::str::FromStr;

use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tracing::{error, info};

#[derive(Clone)]
pub struct ProjectionDatabase {
    connect_options: PgConnectOptions,
}

impl ProjectionDatabase {
    pub async fn connect(
        postgres_admin_url: &str,
        database_name: &str,
    ) -> Result<Self, ProjectionDatabaseError> {
        let mut options = PgConnectOptions::from_str(postgres_admin_url)
            .map_err(ProjectionDatabaseError::connect)?;
        options = options.database(database_name);

        let database = Self {
            connect_options: options,
        };

        let pool = database.connect_pool().await?;
        pool.close().await;

        Ok(database)
    }

    pub async fn initialize_schema(
        &self,
        projection_name: &str,
        statements: &[&str],
    ) -> Result<(), ProjectionDatabaseError> {
        info!(projection_name, "starting projection schema initialization");
        let pool = self.connect_pool().await?;

        for statement in statements {
            if let Err(error) = sqlx::query(statement).execute(&pool).await {
                let projection_error = ProjectionDatabaseError::schema(error);
                error!(
                    projection_name,
                    error = %projection_error,
                    "failed projection schema initialization"
                );
                return Err(projection_error);
            }
        }

        pool.close().await;

        info!(
            projection_name,
            "successful projection schema initialization"
        );
        Ok(())
    }

    pub async fn connect_pool(&self) -> Result<PgPool, ProjectionDatabaseError> {
        PgPoolOptions::new()
            .max_connections(5)
            .connect_with(self.connect_options.clone())
            .await
            .map_err(ProjectionDatabaseError::connect)
    }
}

#[derive(Debug)]
pub enum ProjectionDatabaseError {
    Connect(sqlx::Error),
    Schema(sqlx::Error),
}

impl ProjectionDatabaseError {
    fn connect(error: sqlx::Error) -> Self {
        Self::Connect(error)
    }

    fn schema(error: sqlx::Error) -> Self {
        Self::Schema(error)
    }
}

impl Display for ProjectionDatabaseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(error) => {
                write!(formatter, "projection database connect failed: {error}")
            }
            Self::Schema(error) => write!(
                formatter,
                "projection database schema initialization failed: {error}"
            ),
        }
    }
}

impl std::error::Error for ProjectionDatabaseError {}
