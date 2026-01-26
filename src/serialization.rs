use crate::fractional_number::FractionalNumber;
use crate::node::NodeKind;
use serde::{Deserialize, Serialize};

/// Serialized representation of a node for .fcs files
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SerializedNode {
    Craft(SerializedCraftNode),
    Organizer(SerializedOrganizerNode),
    Sink(SerializedSinkNode),
}

/// Craft node serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedCraftNode {
    pub kind: u8, // 0 for Craft
    pub recipe: String,
    pub rate: SerializedRate,
    pub pos: SerializedPosition,
    pub built: bool,
    pub locked: bool,
    pub num_somersloop: u8,
}

/// Sink node serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedSinkNode {
    pub kind: u8, // 5 for Sink
    pub pos: SerializedPosition,
    pub ins: Vec<SerializedSinkInput>,
}

/// Sink input item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedSinkInput {
    pub item: String,
    pub num: i64,
    pub den: i64,
    pub locked: bool,
}

/// Simple representation for organizer pin entries (used in some file exports)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedPinEntry {
    #[serde(default)]
    pub item: Option<String>,
    pub num: i64,
    pub den: i64,
    pub locked: bool,
}

/// Organizer node (splitters/mergers)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedOrganizerNode {
    pub kind: u8, // 1=CustomSplitter, 2=Merger, 3=Group, 4=GameSplitter
    pub pos: SerializedPosition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// Some exported files (from C++ app) include ins/outs arrays for organizers; support them for compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ins: Option<Vec<SerializedPinEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outs: Option<Vec<SerializedPinEntry>>,
}

/// Position in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedPosition {
    pub x: f32,
    pub y: f32,
}

/// Rate (fractional number)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedRate {
    pub num: i64,
    pub den: i64,
}

impl From<FractionalNumber> for SerializedRate {
    fn from(f: FractionalNumber) -> Self {
        Self {
            num: f.numerator(),
            den: f.denominator(),
        }
    }
}

impl From<SerializedRate> for FractionalNumber {
    fn from(r: SerializedRate) -> Self {
        FractionalNumber::new(r.num, r.den)
    }
}

/// Link endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedLinkEndpoint {
    pub node: usize,
    pub pin: usize,
}

/// Link between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedLink {
    pub start: SerializedLinkEndpoint,
    pub end: SerializedLinkEndpoint,
}

/// Complete production chain file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionChainFile {
    pub game_version: String,
    pub save_version: u32,
    pub nodes: Vec<SerializedNode>,
    pub links: Vec<SerializedLink>,
}

impl NodeKind {
    pub fn to_kind_id(&self) -> u8 {
        match self {
            NodeKind::Craft => 0,
            NodeKind::CustomSplitter => 1,
            NodeKind::Merger => 2,
            NodeKind::Group => 3,
            NodeKind::GameSplitter => 4,
            NodeKind::Sink => 5,
        }
    }

    pub fn from_kind_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(NodeKind::Craft),
            1 => Some(NodeKind::CustomSplitter),
            2 => Some(NodeKind::Merger),
            3 => Some(NodeKind::Group),
            4 => Some(NodeKind::GameSplitter),
            5 => Some(NodeKind::Sink),
            _ => None,
        }
    }
}
