#![cfg_attr(test, allow(dead_code))]

use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::net::SocketAddr;

const POSTGRES_ADMIN_URL_ENV: &str = "FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL";
const DATABASE_NAME_ENV: &str = "FACTSTR_TOOL_RENTAL_DATABASE_NAME";
const BIND_ADDRESS_ENV: &str = "FACTSTR_TOOL_RENTAL_BIND_ADDRESS";
const DEFAULT_DATABASE_NAME: &str = "factstr_tool_rental";
const DEFAULT_BIND_ADDRESS: &str = "127.0.0.1:3000";

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub postgres_admin_url: String,
    pub database_name: String,
    pub bind_address: SocketAddr,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();

        let postgres_admin_url = env::var(POSTGRES_ADMIN_URL_ENV)
            .map_err(|_| ConfigError::MissingEnvironmentVariable(POSTGRES_ADMIN_URL_ENV))?;

        let database_name =
            env::var(DATABASE_NAME_ENV).unwrap_or_else(|_| DEFAULT_DATABASE_NAME.to_owned());

        let bind_address = env::var(BIND_ADDRESS_ENV)
            .unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.to_owned())
            .parse()
            .map_err(|source| ConfigError::InvalidBindAddress {
                value: env::var(BIND_ADDRESS_ENV)
                    .unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.to_owned()),
                source,
            })?;

        Ok(Self {
            postgres_admin_url,
            database_name,
            bind_address,
        })
    }
}

#[derive(Debug)]
pub enum ConfigError {
    MissingEnvironmentVariable(&'static str),
    InvalidBindAddress {
        value: String,
        source: std::net::AddrParseError,
    },
}

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEnvironmentVariable(name) => {
                write!(formatter, "missing required environment variable {name}")
            }
            Self::InvalidBindAddress { value, source } => {
                write!(formatter, "invalid bind address '{value}': {source}")
            }
        }
    }
}

impl Error for ConfigError {}
