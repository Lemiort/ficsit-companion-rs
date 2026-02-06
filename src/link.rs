use serde::{Deserialize, Serialize};

/// Direction of flow along a link (mirrors ax::NodeEditor::FlowDirection)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FlowDirection {
    Forward,
    Backward,
}

/// Represents a connection between two pins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: u64,
    pub start_pin_id: u64,
    pub end_pin_id: u64,
    /// Optional flow direction when visualizing; None means no flow
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flow: Option<FlowDirection>,
}

impl Link {
    pub fn new(id: u64, start_pin_id: u64, end_pin_id: u64) -> Self {
        Self {
            id,
            start_pin_id,
            end_pin_id,
            flow: None,
        }
    }
}
