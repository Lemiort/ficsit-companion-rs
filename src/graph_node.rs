//! Unified graph node type for the Snarl editor.
//! 
//! This module provides a `GraphNode` that stores just a node ID, and provides
//! helper methods to access data from the ProductionApp. This eliminates the need
//! for separate EditorNode copies and reduces data duplication.
//!
//! The GraphNode is essentially a lightweight handle that can be cloned freely.
//! 
//! `NodeDisplayData` is the per-frame cache that holds all display information
//! for rendering. It's rebuilt from ProductionApp each frame before rendering.

use crate::fractional_number::FractionalNumber;
use crate::node::NodeKind;
use crate::pin::PinDirection;

/// Lightweight node reference for the graph editor.
/// Stores just the node ID - all data is accessed through ProductionApp.
#[derive(Clone, Debug)]
pub struct GraphNode {
    /// The node ID in ProductionApp
    pub id: u64,
    /// The node type (cached for quick access without ProductionApp lookup)
    pub node_type: GraphNodeType,
}

impl GraphNode {
    pub fn new(id: u64, node_type: GraphNodeType) -> Self {
        Self { id, node_type }
    }
}

/// Node type enum for UI purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphNodeType {
    Craft,
    Merger,
    GameSplitter,
    CustomSplitter,
    Sink,
    Group,
}

impl GraphNodeType {
    pub fn from_node_kind(kind: NodeKind) -> Self {
        match kind {
            NodeKind::Craft => GraphNodeType::Craft,
            NodeKind::Merger => GraphNodeType::Merger,
            NodeKind::GameSplitter => GraphNodeType::GameSplitter,
            NodeKind::CustomSplitter => GraphNodeType::CustomSplitter,
            NodeKind::Sink => GraphNodeType::Sink,
            NodeKind::Group => GraphNodeType::Group,
        }
    }
    
    pub fn is_organizer(&self) -> bool {
        matches!(self, GraphNodeType::Merger | GraphNodeType::GameSplitter | GraphNodeType::CustomSplitter)
    }
}

impl std::fmt::Display for GraphNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphNodeType::Craft => write!(f, "Craft"),
            GraphNodeType::Merger => write!(f, "Merger"),
            GraphNodeType::GameSplitter => write!(f, "GameSplitter"),
            GraphNodeType::CustomSplitter => write!(f, "CustomSplitter"),
            GraphNodeType::Sink => write!(f, "Sink"),
            GraphNodeType::Group => write!(f, "Group"),
        }
    }
}

/// Per-frame display data cache for a node.
/// This is rebuilt from ProductionApp each frame before rendering.
/// It mirrors what C++ stores directly on nodes, but in Rust we keep
/// ProductionApp as the single source of truth and rebuild this cache each frame.
#[derive(Clone, Debug)]
pub struct NodeDisplayData {
    /// The production node ID
    pub id: u64,
    /// Display label
    pub label: String,
    /// Node type for UI decisions
    pub node_type: GraphNodeType,

    // Pin metadata for icons, labels, rates and locked state
    pub input_names: Vec<Option<String>>,
    pub input_icons: Vec<Option<egui::TextureId>>,
    pub input_rates: Vec<Option<FractionalNumber>>,
    pub input_locked: Vec<bool>,
    pub output_names: Vec<Option<String>>,
    pub output_icons: Vec<Option<egui::TextureId>>,
    pub output_rates: Vec<Option<FractionalNumber>>,
    pub output_locked: Vec<bool>,

    // Building info for craft nodes
    pub building_count: Option<FractionalNumber>,
    pub building_name: String,
    pub same_clock_power: Option<FractionalNumber>,
    pub last_underclock_power: Option<FractionalNumber>,
    pub variable_power: bool,

    // Somersloop info
    pub num_somersloop: Option<FractionalNumber>,
    pub somersloop_mult: Option<FractionalNumber>,
    pub somersloop_icon: Option<egui::TextureId>,

    // For group nodes: whether all contained craft nodes are built
    pub group_built: Option<bool>,

    // For sink nodes: total sink points
    pub sink_points: Option<FractionalNumber>,
    pub sink_points_fraction_str: String,

    // Optional item type for merger/splitter nodes
    pub item_type: Option<String>,
    pub item_type_icon: Option<egui::TextureId>,
}

impl NodeDisplayData {
    /// Create a new display data with just ID and type (pins empty)
    pub fn new(id: u64, label: impl Into<String>, node_type: GraphNodeType) -> Self {
        Self {
            id,
            label: label.into(),
            node_type,
            input_names: Vec::new(),
            input_icons: Vec::new(),
            input_rates: Vec::new(),
            input_locked: Vec::new(),
            output_names: Vec::new(),
            output_icons: Vec::new(),
            output_rates: Vec::new(),
            output_locked: Vec::new(),
            building_count: None,
            building_name: String::new(),
            same_clock_power: None,
            last_underclock_power: None,
            variable_power: false,
            num_somersloop: None,
            somersloop_mult: None,
            somersloop_icon: None,
            group_built: None,
            sink_points: None,
            sink_points_fraction_str: String::new(),
            item_type: None,
            item_type_icon: None,
        }
    }

    /// Check if this is an organizer node (merger/splitter)
    pub fn is_organizer(&self) -> bool {
        self.node_type.is_organizer()
    }
}

/// Unified pending change type - consolidates all the separate pending queues
#[derive(Clone, Debug)]
pub enum PendingChange {
    /// Pin rate edit: (node_id, direction, pin_index, new_value)
    PinRate {
        node_id: u64,
        direction: PinDirection,
        pin_index: usize,
        value: FractionalNumber,
    },
    /// Node building count edit
    NodeBuilding {
        node_id: u64,
        count: FractionalNumber,
    },
    /// Node somersloop edit
    NodeSomersloop {
        node_id: u64,
        value: FractionalNumber,
    },
    /// Node built state edit
    NodeBuilt {
        node_id: u64,
        built: bool,
    },
    /// Add a pin to a node
    PinAdd {
        node_id: u64,
        direction: PinDirection,
    },
    /// Remove a pin from a node
    PinRemove {
        node_id: u64,
        direction: PinDirection,
        index: usize,
    },
    /// Connect two pins
    Connect {
        out_pin: egui_snarl::OutPinId,
        in_pin: egui_snarl::InPinId,
    },
    /// Disconnect two pins
    Disconnect {
        out_pin: egui_snarl::OutPinId,
        in_pin: egui_snarl::InPinId,
    },
    /// Node lock state change
    NodeLock {
        node_id: u64,
        locked: bool,
    },
    /// Node item type change (for organizers/sinks)
    NodeItem {
        node_id: u64,
        item: Option<String>,
    },
    /// Sink pin item type change (for individual sink input pins)
    SinkPinItem {
        node_id: u64,
        pin_idx: usize,
        item: Option<String>,
    },
}

impl PendingChange {
    /// Create a pin rate change
    pub fn pin_rate(node_id: u64, direction: PinDirection, pin_index: usize, value: FractionalNumber) -> Self {
        Self::PinRate { node_id, direction, pin_index, value }
    }

    /// Create a building count change
    pub fn building(node_id: u64, count: FractionalNumber) -> Self {
        Self::NodeBuilding { node_id, count }
    }

    /// Create a somersloop change
    pub fn somersloop(node_id: u64, value: FractionalNumber) -> Self {
        Self::NodeSomersloop { node_id, value }
    }

    /// Create a built state change
    pub fn built(node_id: u64, built: bool) -> Self {
        Self::NodeBuilt { node_id, built }
    }

    /// Create a pin add change
    pub fn add_pin(node_id: u64, direction: PinDirection) -> Self {
        Self::PinAdd { node_id, direction }
    }

    /// Create a pin remove change
    pub fn remove_pin(node_id: u64, direction: PinDirection, index: usize) -> Self {
        Self::PinRemove { node_id, direction, index }
    }

    /// Create a connect change
    pub fn connect(out_pin: egui_snarl::OutPinId, in_pin: egui_snarl::InPinId) -> Self {
        Self::Connect { out_pin, in_pin }
    }

    /// Create a disconnect change
    pub fn disconnect(out_pin: egui_snarl::OutPinId, in_pin: egui_snarl::InPinId) -> Self {
        Self::Disconnect { out_pin, in_pin }
    }

    /// Create a lock change
    pub fn lock(node_id: u64, locked: bool) -> Self {
        Self::NodeLock { node_id, locked }
    }

    /// Create an item type change
    pub fn item(node_id: u64, item: Option<String>) -> Self {
        Self::NodeItem { node_id, item }
    }
}
