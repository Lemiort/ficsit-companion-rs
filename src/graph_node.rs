//! Unified graph node type for the Snarl editor.
//!
//! This module provides a `GraphNode` that stores just a node ID, and provides
//! helper methods to access data from the ProductionApp. This eliminates the need
//! for separate EditorNode copies and reduces data duplication.
//!
//! The GraphNode is essentially a lightweight handle that can be cloned freely.
//!
//! `NodeDisplayData` is the per-frame cache enum that holds all display information
//! for rendering. Each variant contains only the fields relevant to that node type.
//! It's rebuilt from ProductionApp each frame before rendering.

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
        matches!(
            self,
            GraphNodeType::Merger | GraphNodeType::GameSplitter | GraphNodeType::CustomSplitter
        )
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

#[derive(Clone, Debug, Default)]
pub struct ItemData {
    pub name: String,
    pub icon: egui::TextureId,
}

impl PartialEq for ItemData {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// Common pin data shared by all node types
#[derive(Clone, Debug, Default)]
pub struct PinData {
    pub input_items: Vec<Option<ItemData>>,
    pub input_rates: Vec<Option<FractionalNumber>>,
    pub input_locked: Vec<bool>,
    pub output_items: Vec<Option<ItemData>>,
    pub output_rates: Vec<Option<FractionalNumber>>,
    pub output_locked: Vec<bool>,
}

/// Craft node specific data
#[derive(Clone, Debug)]
pub struct CraftData {
    pub building_count: FractionalNumber,
    pub building_name: String,
    pub same_clock_power: FractionalNumber,
    pub last_underclock_power: FractionalNumber,
    pub variable_power: bool,
    pub num_somersloop: FractionalNumber,
    pub somersloop_mult: FractionalNumber,
    pub somersloop_icon: Option<egui::TextureId>,
    pub is_power_generator: bool,
    pub built: bool,
}

impl Default for CraftData {
    fn default() -> Self {
        Self {
            building_count: FractionalNumber::default(),
            building_name: String::new(),
            same_clock_power: FractionalNumber::default(),
            last_underclock_power: FractionalNumber::default(),
            variable_power: false,
            num_somersloop: FractionalNumber::default(),
            somersloop_mult: FractionalNumber::default(),
            somersloop_icon: None,
            is_power_generator: false,
            built: false,
        }
    }
}

/// Organizer (Merger/Splitter) node specific data
#[derive(Clone, Debug, Default)]
pub struct OrganizerData {
    pub item_type: Option<ItemData>,
}

/// Sink node specific data
#[derive(Clone, Debug, Default)]
pub struct SinkData {
    pub sink_points: FractionalNumber,
    pub sink_points_fraction_str: String,
    pub item_type: Option<ItemData>,
}

/// Group node specific data
#[derive(Clone, Debug, Default)]
pub struct GroupData {
    pub is_built: bool,
}

/// Per-frame display data cache for a node.
/// Each variant contains only the fields relevant to that node type.
/// This is rebuilt from ProductionApp each frame before rendering.
#[derive(Clone, Debug)]
pub enum NodeDisplayData {
    Craft {
        id: u64,
        label: String,
        pins: PinData,
        craft: CraftData,
    },
    Merger {
        id: u64,
        label: String,
        pins: PinData,
        organizer: OrganizerData,
    },
    GameSplitter {
        id: u64,
        label: String,
        pins: PinData,
        organizer: OrganizerData,
    },
    CustomSplitter {
        id: u64,
        label: String,
        pins: PinData,
        organizer: OrganizerData,
    },
    Sink {
        id: u64,
        label: String,
        pins: PinData,
        sink: SinkData,
    },
    Group {
        id: u64,
        label: String,
        pins: PinData,
        group: GroupData,
    },
}

impl NodeDisplayData {
    /// Get the node ID
    pub fn id(&self) -> u64 {
        match self {
            NodeDisplayData::Craft { id, .. } => *id,
            NodeDisplayData::Merger { id, .. } => *id,
            NodeDisplayData::GameSplitter { id, .. } => *id,
            NodeDisplayData::CustomSplitter { id, .. } => *id,
            NodeDisplayData::Sink { id, .. } => *id,
            NodeDisplayData::Group { id, .. } => *id,
        }
    }

    /// Get the display label
    pub fn label(&self) -> &str {
        match self {
            NodeDisplayData::Craft { label, .. } => label,
            NodeDisplayData::Merger { label, .. } => label,
            NodeDisplayData::GameSplitter { label, .. } => label,
            NodeDisplayData::CustomSplitter { label, .. } => label,
            NodeDisplayData::Sink { label, .. } => label,
            NodeDisplayData::Group { label, .. } => label,
        }
    }

    /// Get the pin data
    pub fn pins(&self) -> &PinData {
        match self {
            NodeDisplayData::Craft { pins, .. } => pins,
            NodeDisplayData::Merger { pins, .. } => pins,
            NodeDisplayData::GameSplitter { pins, .. } => pins,
            NodeDisplayData::CustomSplitter { pins, .. } => pins,
            NodeDisplayData::Sink { pins, .. } => pins,
            NodeDisplayData::Group { pins, .. } => pins,
        }
    }

    /// Get mutable pin data
    pub fn pins_mut(&mut self) -> &mut PinData {
        match self {
            NodeDisplayData::Craft { pins, .. } => pins,
            NodeDisplayData::Merger { pins, .. } => pins,
            NodeDisplayData::GameSplitter { pins, .. } => pins,
            NodeDisplayData::CustomSplitter { pins, .. } => pins,
            NodeDisplayData::Sink { pins, .. } => pins,
            NodeDisplayData::Group { pins, .. } => pins,
        }
    }

    /// Get the node type
    pub fn node_type(&self) -> GraphNodeType {
        match self {
            NodeDisplayData::Craft { .. } => GraphNodeType::Craft,
            NodeDisplayData::Merger { .. } => GraphNodeType::Merger,
            NodeDisplayData::GameSplitter { .. } => GraphNodeType::GameSplitter,
            NodeDisplayData::CustomSplitter { .. } => GraphNodeType::CustomSplitter,
            NodeDisplayData::Sink { .. } => GraphNodeType::Sink,
            NodeDisplayData::Group { .. } => GraphNodeType::Group,
        }
    }

    /// Check if this is an organizer node (merger/splitter)
    pub fn is_organizer(&self) -> bool {
        matches!(
            self,
            NodeDisplayData::Merger { .. }
                | NodeDisplayData::GameSplitter { .. }
                | NodeDisplayData::CustomSplitter { .. }
        )
    }

    /// Check if this is a splitter node
    pub fn is_splitter(&self) -> bool {
        matches!(
            self,
            NodeDisplayData::GameSplitter { .. } | NodeDisplayData::CustomSplitter { .. }
        )
    }

    pub fn item_data(&self) -> Option<&ItemData> {
        match self {
            NodeDisplayData::Merger { organizer, .. } => organizer.item_type.as_ref(),
            NodeDisplayData::GameSplitter { organizer, .. } => organizer.item_type.as_ref(),
            NodeDisplayData::CustomSplitter { organizer, .. } => organizer.item_type.as_ref(),
            NodeDisplayData::Sink { sink, .. } => sink.item_type.as_ref(),
            _ => None,
        }
    }

    /// Get build progress status for Craft and Group nodes
    pub fn built(&self) -> Option<bool> {
        match self {
            NodeDisplayData::Craft { craft, .. } => Some(craft.built),
            NodeDisplayData::Group { group, .. } => Some(group.is_built),
            _ => None,
        }
    }

    /// Create a new Craft display data
    pub fn new_craft(id: u64, label: impl Into<String>) -> Self {
        NodeDisplayData::Craft {
            id,
            label: label.into(),
            pins: PinData::default(),
            craft: CraftData::default(),
        }
    }

    /// Create a new Merger display data
    pub fn new_merger(id: u64, label: impl Into<String>) -> Self {
        NodeDisplayData::Merger {
            id,
            label: label.into(),
            pins: PinData::default(),
            organizer: OrganizerData::default(),
        }
    }

    /// Create a new GameSplitter display data
    pub fn new_game_splitter(id: u64, label: impl Into<String>) -> Self {
        NodeDisplayData::GameSplitter {
            id,
            label: label.into(),
            pins: PinData::default(),
            organizer: OrganizerData::default(),
        }
    }

    /// Create a new CustomSplitter display data
    pub fn new_custom_splitter(id: u64, label: impl Into<String>) -> Self {
        NodeDisplayData::CustomSplitter {
            id,
            label: label.into(),
            pins: PinData::default(),
            organizer: OrganizerData::default(),
        }
    }

    /// Create a new Sink display data
    pub fn new_sink(id: u64, label: impl Into<String>) -> Self {
        NodeDisplayData::Sink {
            id,
            label: label.into(),
            pins: PinData::default(),
            sink: SinkData::default(),
        }
    }

    /// Create a new Group display data
    pub fn new_group(id: u64, label: impl Into<String>) -> Self {
        NodeDisplayData::Group {
            id,
            label: label.into(),
            pins: PinData::default(),
            group: GroupData::default(),
        }
    }

    /// Create a display data from node type
    pub fn from_type(id: u64, label: impl Into<String>, node_type: GraphNodeType) -> Self {
        match node_type {
            GraphNodeType::Craft => Self::new_craft(id, label),
            GraphNodeType::Merger => Self::new_merger(id, label),
            GraphNodeType::GameSplitter => Self::new_game_splitter(id, label),
            GraphNodeType::CustomSplitter => Self::new_custom_splitter(id, label),
            GraphNodeType::Sink => Self::new_sink(id, label),
            GraphNodeType::Group => Self::new_group(id, label),
        }
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
    NodeBuilt { node_id: u64, built: bool },
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
    NodeLock { node_id: u64, locked: bool },
    /// Node item type change (for organizers/sinks)
    NodeItem { node_id: u64, item: Option<String> },
    /// Sink pin item type change (for individual sink input pins)
    SinkPinItem {
        node_id: u64,
        pin_idx: usize,
        item: Option<String>,
    },
    /// Individual pin lock state change (for custom splitters/mergers)
    PinLock {
        node_id: u64,
        direction: PinDirection,
        pin_index: usize,
        locked: bool,
    },
    /// Group name change
    GroupName {
        node_id: u64,
        name: String,
    },
}

impl PendingChange {
    /// Create a pin rate change
    pub fn pin_rate(
        node_id: u64,
        direction: PinDirection,
        pin_index: usize,
        value: FractionalNumber,
    ) -> Self {
        Self::PinRate {
            node_id,
            direction,
            pin_index,
            value,
        }
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
        Self::PinRemove {
            node_id,
            direction,
            index,
        }
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

    /// Create a pin lock change
    pub fn pin_lock(node_id: u64, direction: PinDirection, pin_index: usize, locked: bool) -> Self {
        Self::PinLock {
            node_id,
            direction,
            pin_index,
            locked,
        }
    }

    /// Create a group name change
    pub fn group_name(node_id: u64, name: String) -> Self {
        Self::GroupName { node_id, name }
    }
}
