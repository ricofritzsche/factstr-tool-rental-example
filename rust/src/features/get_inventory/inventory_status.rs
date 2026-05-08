use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventoryStatus {
    Available,
    CheckedOut,
}

impl InventoryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::CheckedOut => "checked_out",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "available" => Some(Self::Available),
            "checked_out" => Some(Self::CheckedOut),
            _ => None,
        }
    }
}
