mod config;
mod health;
mod http;
mod logging;
mod routes;
mod store;

use std::error::Error;

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

    let app = build_routes(store);
    let listener = TcpListener::bind(config.bind_address).await?;
    let listening_address = listener.local_addr()?;

    info!(address = %listening_address, "HTTP API listening");

    axum::serve(listener, app).await?;

    Ok(())
}
