use crate::fractional_number::FractionalNumber;
use serde::{Deserialize, Serialize};

/// Represents a pin (input/output point) on a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub id: u64,
    pub direction: PinDirection,
    pub node_id: u64, // ID of the node this pin belongs to
    pub item_name: Option<String>,
    pub base_rate: FractionalNumber,
    pub current_rate: FractionalNumber,
    pub locked: bool,
    pub error: bool,
    /// Optional link id this pin is connected to
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_id: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PinDirection {
    Input,
    Output,
}

impl Pin {
    pub fn new(
        id: u64,
        direction: PinDirection,
        node_id: u64,
        item_name: Option<String>,
        locked: bool,
        base_rate: FractionalNumber,
    ) -> Self {
        Self {
            id,
            direction,
            node_id,
            item_name,
            base_rate,
            current_rate: FractionalNumber::default(),
            locked,
            error: false,
            link_id: None,
        }
    }

    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    pub fn get_locked(&self) -> bool {
        self.locked
    }
}
