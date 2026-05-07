#[path = "../src/config.rs"]
mod config;
#[path = "../src/store.rs"]
mod store;

use std::env;
use std::error::Error;

#[tokio::test]
async fn postgres_store_initializes_and_test_database_is_cleaned_up() -> Result<(), Box<dyn Error>>
{
    let _ = dotenvy::dotenv();

    let admin_url = match env::var("FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL") {
        Ok(value) => value,
        Err(_) => {
            eprintln!(
                "skipping store integration test: FACTSTR_TOOL_RENTAL_POSTGRES_ADMIN_URL is not set"
            );
            return Ok(());
        }
    };

    let mut test_database = store::TestDatabase::create(&admin_url).await?;
    assert!(
        !store::database_exists(&admin_url, test_database.database_name()).await?,
        "test database should not exist before FACTSTR bootstrap runs"
    );

    let app_store = test_database.open_store().await?;
    assert!(
        store::database_exists(&admin_url, test_database.database_name()).await?,
        "test database should exist after FACTSTR bootstrap"
    );

    app_store.check_connectivity().await?;

    drop(app_store);
    test_database.cleanup().await?;

    assert!(
        !store::database_exists(&admin_url, test_database.database_name()).await?,
        "test database should be removed after cleanup"
    );

    Ok(())
}
