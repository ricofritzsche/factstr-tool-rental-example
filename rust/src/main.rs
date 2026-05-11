mod config;
mod health;
mod http;
mod logging;
mod routes;
mod store;

use std::error::Error;

use factstr_tool_rental_rust::features::get_inventory::{
    InventoryChangeNotifier, start_projection_with_notifier,
};
use factstr_tool_rental_rust::projection_database::ProjectionDatabase;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::config::AppConfig;
use crate::routes::build_routes;
use crate::store::AppStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    logging::init_logging();
    info!("starting FACTSTR tool rental Rust API");

    let config = match AppConfig::from_env() {
        Ok(config) => {
            info!(
                database_name = %config.database_name,
                bind_address = %config.bind_address,
                "configuration loaded"
            );
            config
        }
        Err(config_error) => {
            error!(error = %config_error, "failed to load configuration");
            return Err(config_error.into());
        }
    };

    let store = match AppStore::initialize(&config).await {
        Ok(store) => store,
        Err(store_error) => {
            error!(error = %store_error, "failed to initialize PostgreSQL/FACTSTR store");
            return Err(store_error.into());
        }
    };

    let projection_database = match ProjectionDatabase::connect(
        &config.postgres_admin_url,
        &config.database_name,
    )
    .await
    {
        Ok(database) => database,
        Err(projection_database_error) => {
            error!(
                error = %projection_database_error,
                "failed to initialize projection database infrastructure"
            );
            return Err(projection_database_error.into());
        }
    };

    let inventory_change_notifier = InventoryChangeNotifier::new();
    let inventory_projection = match start_projection_with_notifier(
        &store,
        projection_database,
        inventory_change_notifier.clone(),
    )
    .await
    {
        Ok(projection) => {
            info!("get inventory durable projection started");
            projection
        }
        Err(projection_error) => {
            error!(error = %projection_error, "failed to start get inventory durable projection");
            return Err(projection_error.into());
        }
    };

    let app = build_routes(store, inventory_projection, inventory_change_notifier);
    let listener = TcpListener::bind(config.bind_address).await?;
    let listening_address = listener.local_addr()?;

    info!(address = %listening_address, "HTTP API listening");

    axum::serve(listener, app).await?;

    Ok(())
}
