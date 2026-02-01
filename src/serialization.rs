use crate::fractional_number::FractionalNumber;
use crate::node::NodeKind;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Serialized representation of a node for .fcs files
#[derive(Debug, Clone)]
pub enum SerializedNode {
    Group(SerializedGroupNode),
    Craft(SerializedCraftNode),
    Organizer(SerializedOrganizerNode),
    Sink(SerializedSinkNode),
}

impl Serialize for SerializedNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            SerializedNode::Group(g) => g.serialize(serializer),
            SerializedNode::Craft(c) => c.serialize(serializer),
            SerializedNode::Organizer(o) => o.serialize(serializer),
            SerializedNode::Sink(s) => s.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for SerializedNode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // First deserialize as a generic JSON value to inspect the kind field
        let value = serde_json::Value::deserialize(deserializer)?;
        
        let kind = value.get("kind")
            .and_then(|k| k.as_u64())
            .ok_or_else(|| serde::de::Error::custom("missing 'kind' field"))?;
        
        match kind {
            0 => {
                let craft: SerializedCraftNode = serde_json::from_value(value)
                    .map_err(serde::de::Error::custom)?;
                Ok(SerializedNode::Craft(craft))
            }
            1 | 2 | 4 => {
                // CustomSplitter, Merger, GameSplitter
                let org: SerializedOrganizerNode = serde_json::from_value(value)
                    .map_err(serde::de::Error::custom)?;
                Ok(SerializedNode::Organizer(org))
            }
            3 => {
                let group: SerializedGroupNode = serde_json::from_value(value)
                    .map_err(serde::de::Error::custom)?;
                Ok(SerializedNode::Group(group))
            }
            5 => {
                let sink: SerializedSinkNode = serde_json::from_value(value)
                    .map_err(serde::de::Error::custom)?;
                Ok(SerializedNode::Sink(sink))
            }
            _ => Err(serde::de::Error::custom(format!("unknown node kind: {}", kind))),
        }
    }
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

/// Group node serialization (kind=3)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedGroupNode {
    pub kind: u8, // 3 for Group
    pub pos: SerializedPosition,
    pub rate: SerializedRate,
    pub locked: bool,
    pub name: String,
    /// Nodes contained within this group
    pub nodes: Vec<SerializedNode>,
    /// Links between nodes within this group
    pub links: Vec<SerializedLink>,
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
