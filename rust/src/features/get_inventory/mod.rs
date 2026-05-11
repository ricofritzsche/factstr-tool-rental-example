pub mod apply_fact;
pub mod inventory_change_notifier;
pub mod inventory_item;
pub mod inventory_projection;
pub mod inventory_status;
pub mod projection_schema;
pub mod projection_store;
pub mod query_inventory;
pub mod start_projection;

pub use inventory_change_notifier::InventoryChangeNotifier;
pub use inventory_item::InventoryItem;
pub use inventory_projection::InventoryProjectionError;
pub use inventory_status::InventoryStatus;
pub use query_inventory::get_inventory;
pub use start_projection::{
    InventoryProjection, start_projection, start_projection_in_memory,
    start_projection_in_memory_with_notifier, start_projection_with_notifier,
};
