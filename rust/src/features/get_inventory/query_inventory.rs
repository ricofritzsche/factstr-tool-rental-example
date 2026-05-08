use crate::features::get_inventory::inventory_item::InventoryItem;
use crate::features::get_inventory::inventory_projection::InventoryProjectionError;
use crate::features::get_inventory::start_projection::InventoryProjection;

pub fn get_inventory(
    projection: &InventoryProjection,
) -> Result<Vec<InventoryItem>, InventoryProjectionError> {
    projection.snapshot()
}
