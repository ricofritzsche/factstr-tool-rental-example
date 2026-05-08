use sqlx::{PgPool, Row};
use time::OffsetDateTime;

use crate::events::{ToolCheckedOutPayload, ToolRegisteredPayload, ToolReturnedPayload};
use crate::features::get_inventory::inventory_item::InventoryItem;
use crate::features::get_inventory::inventory_projection::InventoryProjectionError;
use crate::features::get_inventory::inventory_status::InventoryStatus;
use crate::projection_database::ProjectionDatabase;

#[derive(Clone)]
pub struct ProjectionStore {
    pool: PgPool,
}

impl ProjectionStore {
    pub async fn open(
        projection_database: &ProjectionDatabase,
    ) -> Result<Self, InventoryProjectionError> {
        let pool = projection_database
            .connect_pool()
            .await
            .map_err(InventoryProjectionError::store_error)?;

        Ok(Self { pool })
    }

    pub async fn list_items(&self) -> Result<Vec<InventoryItem>, InventoryProjectionError> {
        let rows = sqlx::query(
            "SELECT tool_id, serial_number, name, category, manufacturer, model, home_location, current_location, status, checked_out_to, due_back_at
             FROM projections.inventory_items
             ORDER BY category, name, serial_number",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(InventoryProjectionError::store_error)?;

        rows.into_iter().map(map_inventory_row).collect()
    }

    pub async fn apply_registered(
        &self,
        payload: &ToolRegisteredPayload,
    ) -> Result<(), InventoryProjectionError> {
        sqlx::query(
            "INSERT INTO projections.inventory_items (
                tool_id, serial_number, name, category, manufacturer, model,
                home_location, current_location, status, checked_out_to, due_back_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL, NULL)
            ON CONFLICT (tool_id) DO UPDATE SET
                serial_number = EXCLUDED.serial_number,
                name = EXCLUDED.name,
                category = EXCLUDED.category,
                manufacturer = EXCLUDED.manufacturer,
                model = EXCLUDED.model,
                home_location = EXCLUDED.home_location,
                current_location = EXCLUDED.current_location,
                status = EXCLUDED.status,
                checked_out_to = NULL,
                due_back_at = NULL",
        )
        .bind(&payload.tool_id)
        .bind(&payload.serial_number)
        .bind(&payload.name)
        .bind(&payload.category)
        .bind(&payload.manufacturer)
        .bind(&payload.model)
        .bind(&payload.home_location)
        .bind(&payload.home_location)
        .bind(InventoryStatus::Available.as_str())
        .execute(&self.pool)
        .await
        .map_err(InventoryProjectionError::store_error)?;

        Ok(())
    }

    pub async fn apply_checked_out(
        &self,
        payload: &ToolCheckedOutPayload,
    ) -> Result<(), InventoryProjectionError> {
        sqlx::query(
            "UPDATE projections.inventory_items
             SET current_location = $1,
                 status = $2,
                 checked_out_to = $3,
                 due_back_at = $4
             WHERE tool_id = $5",
        )
        .bind(&payload.use_location)
        .bind(InventoryStatus::CheckedOut.as_str())
        .bind(&payload.checked_out_to)
        .bind(payload.due_back_at)
        .bind(&payload.tool_id)
        .execute(&self.pool)
        .await
        .map_err(InventoryProjectionError::store_error)?;

        Ok(())
    }

    pub async fn apply_returned(
        &self,
        payload: &ToolReturnedPayload,
    ) -> Result<(), InventoryProjectionError> {
        sqlx::query(
            "UPDATE projections.inventory_items
             SET current_location = $1,
                 status = $2,
                 checked_out_to = NULL,
                 due_back_at = NULL
             WHERE tool_id = $3",
        )
        .bind(&payload.returned_to_location)
        .bind(InventoryStatus::Available.as_str())
        .bind(&payload.tool_id)
        .execute(&self.pool)
        .await
        .map_err(InventoryProjectionError::store_error)?;

        Ok(())
    }
}

fn map_inventory_row(
    row: sqlx::postgres::PgRow,
) -> Result<InventoryItem, InventoryProjectionError> {
    let status = row
        .try_get::<String, _>("status")
        .map_err(InventoryProjectionError::store_error)?;

    Ok(InventoryItem {
        tool_id: row
            .try_get("tool_id")
            .map_err(InventoryProjectionError::store_error)?,
        serial_number: row
            .try_get("serial_number")
            .map_err(InventoryProjectionError::store_error)?,
        name: row
            .try_get("name")
            .map_err(InventoryProjectionError::store_error)?,
        category: row
            .try_get("category")
            .map_err(InventoryProjectionError::store_error)?,
        manufacturer: row
            .try_get("manufacturer")
            .map_err(InventoryProjectionError::store_error)?,
        model: row
            .try_get("model")
            .map_err(InventoryProjectionError::store_error)?,
        home_location: row
            .try_get("home_location")
            .map_err(InventoryProjectionError::store_error)?,
        current_location: row
            .try_get("current_location")
            .map_err(InventoryProjectionError::store_error)?,
        status: InventoryStatus::from_storage(&status)
            .ok_or(InventoryProjectionError::InvalidStatus(status))?,
        checked_out_to: row
            .try_get("checked_out_to")
            .map_err(InventoryProjectionError::store_error)?,
        due_back_at: row
            .try_get::<Option<OffsetDateTime>, _>("due_back_at")
            .map_err(InventoryProjectionError::store_error)?,
    })
}
