use crate::fractional_number::FractionalNumber;
use crate::pin::{Pin, PinDirection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of node in the production graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Craft,
    CustomSplitter,
    Merger,
    Group,
    GameSplitter,
    Sink,
}

/// Base node in the production graph
#[derive(Debug, Clone)]
pub struct Node {
    pub id: u64,
    pub kind: NodeKind,
    pub position: (f32, f32),
    pub ins: Vec<Pin>,
    pub outs: Vec<Pin>,
}

impl Node {
    pub fn new(id: u64, kind: NodeKind) -> Self {
        Self {
            id,
            kind,
            position: (0.0, 0.0),
            ins: Vec::new(),
            outs: Vec::new(),
        }
    }

    pub fn set_position(&mut self, x: f32, y: f32) {
        self.position = (x, y);
    }

    pub fn add_pin(&mut self, pin: Pin) {
        match pin.direction {
            PinDirection::Input => self.ins.push(pin),
            PinDirection::Output => self.outs.push(pin),
        }
    }

    /// Get all pins (inputs and outputs combined)
    pub fn get_pins(&self) -> Vec<&Pin> {
        self.ins.iter().chain(self.outs.iter()).collect()
    }

    /// Get all pins mutably (inputs and outputs combined)
    pub fn get_pins_mut(&mut self) -> Vec<&mut Pin> {
        let mut pins = Vec::new();
        pins.extend(self.ins.iter_mut());
        pins.extend(self.outs.iter_mut());
        pins
    }

    /// Iterator over all pins (inputs and outputs)
    pub fn all_pins(&self) -> impl Iterator<Item = &Pin> {
        self.ins.iter().chain(self.outs.iter())
    }

    /// Mutable iterator over all pins (inputs and outputs)
    pub fn all_pins_mut(&mut self) -> impl Iterator<Item = &mut Pin> {
        self.ins.iter_mut().chain(self.outs.iter_mut())
    }

    /// Find a pin by ID
    pub fn find_pin(&self, pin_id: u64) -> Option<&Pin> {
        self.all_pins().find(|p| p.id == pin_id)
    }

    /// Find a pin by ID (mutable)
    pub fn find_pin_mut(&mut self, pin_id: u64) -> Option<&mut Pin> {
        self.all_pins_mut().find(|p| p.id == pin_id)
    }

    /// Get pin by index in flattened list (for compatibility with code expecting single pins vec)
    pub fn get_pin_by_flat_index(&self, index: usize) -> Option<&Pin> {
        if index < self.ins.len() {
            self.ins.get(index)
        } else {
            self.outs.get(index - self.ins.len())
        }
    }

    /// Get mutable pin by index in flattened list
    pub fn get_pin_by_flat_index_mut(&mut self, index: usize) -> Option<&mut Pin> {
        if index < self.ins.len() {
            self.ins.get_mut(index)
        } else {
            self.outs.get_mut(index - self.ins.len())
        }
    }

    /// Get a flattened pins vector for compatibility
    pub fn pins(&self) -> Vec<&Pin> {
        self.ins.iter().chain(self.outs.iter()).collect()
    }

    /// Get mutable flattened pins vector for compatibility
    pub fn pins_mut(&mut self) -> Vec<&mut Pin> {
        let mut all = Vec::new();
        all.extend(self.ins.iter_mut());
        all.extend(self.outs.iter_mut());
        all
    }

    /// Clear all pins (both inputs and outputs)
    pub fn clear_pins(&mut self) {
        self.ins.clear();
        self.outs.clear();
    }

    /// Push a pin to the appropriate vector based on direction
    pub fn push_pin(&mut self, pin: Pin) {
        match pin.direction {
            PinDirection::Input => self.ins.push(pin),
            PinDirection::Output => self.outs.push(pin),
        }
    }

    /// Get the last pin mutably
    pub fn last_pin_mut(&mut self) -> Option<&mut Pin> {
        if let Some(p) = self.outs.last_mut() {
            return Some(p);
        }
        self.ins.last_mut()
    }

    /// Get all pins as a mutable Vec (for compatibility with code that needs to index mutably)
    pub fn pins_mut_vec(&mut self) -> Vec<&mut Pin> {
        let mut all = Vec::new();
        all.extend(self.ins.iter_mut());
        all.extend(self.outs.iter_mut());
        all
    }

    pub fn is_powered(&self) -> bool {
        matches!(self.kind, NodeKind::Craft | NodeKind::Group)
    }

    pub fn is_organizer(&self) -> bool {
        matches!(
            self.kind,
            NodeKind::CustomSplitter | NodeKind::Merger | NodeKind::GameSplitter
        )
    }

    pub fn is_craft(&self) -> bool {
        self.kind == NodeKind::Craft
    }

    pub fn is_group(&self) -> bool {
        self.kind == NodeKind::Group
    }

    pub fn is_merger(&self) -> bool {
        self.kind == NodeKind::Merger
    }

    pub fn is_custom_splitter(&self) -> bool {
        self.kind == NodeKind::CustomSplitter
    }

    pub fn is_game_splitter(&self) -> bool {
        self.kind == NodeKind::GameSplitter
    }

    pub fn is_sink(&self) -> bool {
        self.kind == NodeKind::Sink
    }
}

/// A craft node that produces items using a recipe
#[derive(Debug, Clone)]
pub struct CraftNode {
    pub base: Node,
    pub recipe_name: String,
    pub recipe_power: f64,
    pub power_exponent: f64,
    pub somersloop_power_exponent: f64,
    pub somersloop_mult: FractionalNumber,
    pub variable_power: bool,
    pub current_rate: FractionalNumber,
    pub same_clock_power: FractionalNumber,
    pub last_underclock_power: FractionalNumber,
    pub num_somersloop: FractionalNumber, // Somersloop boost level
    pub built: bool,                      // Tracking for factory building progress
    pub building_name: String,            // Building name from recipe
}

impl CraftNode {
    pub fn new(id: u64, recipe_name: String) -> Self {
        Self {
            base: Node::new(id, NodeKind::Craft),
            recipe_name,
            recipe_power: 0.0,
            power_exponent: 1.0,
            somersloop_power_exponent: 1.0,
            somersloop_mult: FractionalNumber::new(1, 1),
            variable_power: false,
            current_rate: FractionalNumber::default(),
            same_clock_power: FractionalNumber::default(),
            last_underclock_power: FractionalNumber::default(),
            num_somersloop: FractionalNumber::default(),
            built: false,
            building_name: String::new(),
        }
    }

    pub fn update_rate(&mut self, new_rate: FractionalNumber) {
        self.current_rate = new_rate;
        // Update pins rates based on new rate
        for pin in &mut self.base.ins {
            pin.current_rate = pin.base_rate * new_rate;
        }
        for pin in &mut self.base.outs {
            pin.current_rate = pin.base_rate * new_rate;
        }
    }

    /// Compute power usage in MW for current rate.
    pub fn compute_power_usage(&self) -> (FractionalNumber, FractionalNumber) {
        let rate_value = self.current_rate.value();
        if rate_value <= 0.0 {
            return (FractionalNumber::default(), FractionalNumber::default());
        }

        let num_machines = rate_value.ceil().max(1.0);
        let num_full_machines = rate_value.floor().max(0.0);

        // Boost from somersloop
        let boost = 1.0 + self.num_somersloop.value() * self.somersloop_mult.value();
        let boost_pow = boost.powf(self.somersloop_power_exponent);

        // Same-clock scenario: all machines at identical clock
        let same_clock_power = num_machines
            * self.recipe_power
            * boost_pow
            * (rate_value / num_machines).powf(self.power_exponent);

        // Last-underclock scenario: full machines plus one partial
        let mut last_underclock_power = num_full_machines * self.recipe_power * boost_pow;
        let fractional_machine = rate_value - num_full_machines;
        if fractional_machine > 0.0 {
            last_underclock_power +=
                self.recipe_power * boost_pow * fractional_machine.powf(self.power_exponent);
        }

        let to_fraction = |value: f64| -> FractionalNumber {
            let rounded = (value * 1000.0).round() as i64;
            FractionalNumber::new(rounded, 1000)
        };

        (
            to_fraction(same_clock_power),
            to_fraction(last_underclock_power),
        )
    }
}

/// An organizer node (splitter, merger, game splitter)
#[derive(Debug, Clone)]
pub struct OrganizerNode {
    pub base: Node,
    pub item_name: Option<String>,
}

impl OrganizerNode {
    pub fn new(id: u64, kind: NodeKind, item_name: Option<String>) -> Self {
        Self {
            base: Node::new(id, kind),
            item_name,
        }
    }

    pub fn is_balanced(&self) -> bool {
        let input_sum: FractionalNumber = self
            .base
            .ins
            .iter()
            .map(|p| p.current_rate)
            .fold(FractionalNumber::default(), |a, b| a + b);

        let output_sum: FractionalNumber = self
            .base
            .outs
            .iter()
            .map(|p| p.current_rate)
            .fold(FractionalNumber::default(), |a, b| a + b);

        input_sum == output_sum
    }
}

/// Serialized representation of a node stored within a group.
/// This stores the full node data (not just ID) so groups are self-contained.
#[derive(Debug, Clone)]
pub struct GroupedNode {
    pub node_data: GroupedNodeData,
    /// Relative position within the group (offset from group origin)
    pub relative_pos: (f32, f32),
}

/// The actual node data stored in a group
#[derive(Debug, Clone)]
pub enum GroupedNodeData {
    Craft {
        recipe_name: String,
        current_rate: FractionalNumber,
        num_somersloop: FractionalNumber,
        built: bool,
        building_name: String,
        recipe_power: f64,
        power_exponent: f64,
        somersloop_power_exponent: f64,
        somersloop_mult: FractionalNumber,
        variable_power: bool,
        ins: Vec<GroupedPin>,
        outs: Vec<GroupedPin>,
    },
    Organizer {
        kind: NodeKind,
        item_name: Option<String>,
        ins: Vec<GroupedPin>,
        outs: Vec<GroupedPin>,
    },
    Sink {
        item_name: Option<String>,
        ins: Vec<GroupedPin>,
    },
    Group {
        name: String,
        current_rate: FractionalNumber,
        nodes: Vec<GroupedNode>,
        links: Vec<GroupedLink>,
        ins: Vec<GroupedPin>,
        outs: Vec<GroupedPin>,
    },
}

/// Serialized pin for grouped nodes
#[derive(Debug, Clone)]
pub struct GroupedPin {
    pub item_name: Option<String>,
    pub base_rate: FractionalNumber,
    pub current_rate: FractionalNumber,
    pub locked: bool,
}

/// Link between nodes within a group (uses indices into the group's nodes vec)
#[derive(Debug, Clone)]
pub struct GroupedLink {
    pub start_node_idx: usize,
    pub start_pin_idx: usize,
    pub end_node_idx: usize,
    pub end_pin_idx: usize,
}

/// A group node that contains other nodes
#[derive(Debug, Clone)]
pub struct GroupNode {
    pub base: Node,
    pub current_rate: FractionalNumber,
    pub same_clock_power: FractionalNumber,
    pub last_underclock_power: FractionalNumber,
    /// Stored nodes within the group
    pub grouped_nodes: Vec<GroupedNode>,
    /// The rate of each node when this group was created (to preserve info when rate is 0)
    pub nodes_base_rate: Vec<FractionalNumber>,
    /// Links between nodes within the group
    pub grouped_links: Vec<GroupedLink>,
    /// Aggregate inputs (item_name -> rate)
    pub inputs: HashMap<String, FractionalNumber>,
    /// Aggregate outputs (item_name -> rate)
    pub outputs: HashMap<String, FractionalNumber>,
    /// Aggregate sinked points (item_name -> points)
    pub detailed_sinked_points: HashMap<String, FractionalNumber>,
    /// Group name
    pub name: String,
    /// Whether this group has variable power (cached)
    pub variable_power: bool,
    /// Total machines per building type
    pub total_machines: HashMap<String, FractionalNumber>,
    /// Built machines per building type
    pub built_machines: HashMap<String, FractionalNumber>,
    /// Whether there was an error loading this group
    pub loading_error: bool,
}

impl GroupNode {
    pub fn new(id: u64) -> Self {
        Self {
            base: Node::new(id, NodeKind::Group),
            current_rate: FractionalNumber::new(1, 1),
            same_clock_power: FractionalNumber::default(),
            last_underclock_power: FractionalNumber::default(),
            grouped_nodes: Vec::new(),
            nodes_base_rate: Vec::new(),
            grouped_links: Vec::new(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            detailed_sinked_points: HashMap::new(),
            name: String::new(),
            variable_power: false,
            total_machines: HashMap::new(),
            built_machines: HashMap::new(),
            loading_error: false,
        }
    }

    /// Create a group from existing nodes and links
    pub fn from_nodes_and_links(
        id: u64,
        name: String,
        grouped_nodes: Vec<GroupedNode>,
        nodes_base_rate: Vec<FractionalNumber>,
        grouped_links: Vec<GroupedLink>,
    ) -> Self {
        let mut group = Self::new(id);
        group.name = name;
        group.grouped_nodes = grouped_nodes;
        group.nodes_base_rate = nodes_base_rate;
        group.grouped_links = grouped_links;
        group.current_rate = FractionalNumber::new(1, 1);
        group.create_pins_from_grouped_nodes();
        group.compute_power_usage();
        group.update_details();
        group
    }

    /// Create input/output pins based on net consumed/produced items in the group
    fn create_pins_from_grouped_nodes(&mut self) {
        self.inputs.clear();
        self.outputs.clear();
        self.base.ins.clear();
        self.base.outs.clear();

        // Collect all inputs and outputs from grouped nodes
        for grouped_node in &self.grouped_nodes {
            match &grouped_node.node_data {
                GroupedNodeData::Craft { ins, outs, .. } => {
                    for pin in ins {
                        if let Some(name) = &pin.item_name {
                            *self.inputs.entry(name.clone()).or_insert(FractionalNumber::default()) += pin.current_rate;
                        }
                    }
                    for pin in outs {
                        if let Some(name) = &pin.item_name {
                            *self.outputs.entry(name.clone()).or_insert(FractionalNumber::default()) += pin.current_rate;
                        }
                    }
                }
                GroupedNodeData::Group { ins: sub_inputs, outs: sub_outputs, .. } => {
                    // For nested groups, we'd need to recursively get inputs/outputs
                    // For simplicity, use the stored group pins
                    for pin in sub_inputs {
                        if let Some(name) = &pin.item_name {
                            *self.inputs.entry(name.clone()).or_insert(FractionalNumber::default()) += pin.current_rate;
                        }
                    }
                    for pin in sub_outputs {
                        if let Some(name) = &pin.item_name {
                            *self.outputs.entry(name.clone()).or_insert(FractionalNumber::default()) += pin.current_rate;
                        }
                    }
                }
                GroupedNodeData::Sink { ins, .. } => {
                    for pin in ins {
                        if let Some(name) = &pin.item_name {
                            *self.inputs.entry(name.clone()).or_insert(FractionalNumber::default()) += pin.current_rate;
                        }
                    }
                }
                GroupedNodeData::Organizer { .. } => {
                    // Organizers don't contribute net inputs/outputs
                }
            }
        }

        // Create input pins for items that are net consumed (sorted alphabetically)
        let mut pin_id = self.base.id * 1000; // Simple ID generation for pins
        let mut input_keys: Vec<_> = self.inputs.keys().cloned().collect();
        input_keys.sort();
        for item_name in input_keys {
            let consumed = self.inputs.get(&item_name).cloned().unwrap_or_default();
            let produced = self.outputs.get(&item_name).cloned().unwrap_or_default();
            if consumed > produced {
                let net = consumed - produced;
                let mut pin = crate::pin::Pin::new(
                    pin_id,
                    crate::pin::PinDirection::Input,
                    self.base.id,
                    Some(item_name.clone()),
                    false,
                    net,
                );
                pin.current_rate = net;
                self.base.ins.push(pin);
                pin_id += 1;
            }
        }

        // Create output pins for items that are net produced (sorted alphabetically)
        let mut output_keys: Vec<_> = self.outputs.keys().cloned().collect();
        output_keys.sort();
        for item_name in output_keys {
            let produced = self.outputs.get(&item_name).cloned().unwrap_or_default();
            let consumed = self.inputs.get(&item_name).cloned().unwrap_or_default();
            if produced > consumed {
                let net = produced - consumed;
                let mut pin = crate::pin::Pin::new(
                    pin_id,
                    crate::pin::PinDirection::Output,
                    self.base.id,
                    Some(item_name.clone()),
                    false,
                    net,
                );
                pin.current_rate = net;
                self.base.outs.push(pin);
                pin_id += 1;
            }
        }
    }

    /// Create input/output pins based on net consumed/produced items in the group
    /// Uses provided ID generator function for pin IDs
    pub fn create_pins_from_grouped_nodes_with_id_gen<F>(&mut self, mut get_id: F)
    where
        F: FnMut() -> u64,
    {
        self.inputs.clear();
        self.outputs.clear();
        self.base.ins.clear();
        self.base.outs.clear();

        // Collect all inputs and outputs from grouped nodes
        for grouped_node in &self.grouped_nodes {
            match &grouped_node.node_data {
                GroupedNodeData::Craft { ins, outs, .. } => {
                    for pin in ins {
                        if let Some(name) = &pin.item_name {
                            *self.inputs.entry(name.clone()).or_insert(FractionalNumber::default()) += pin.current_rate;
                        }
                    }
                    for pin in outs {
                        if let Some(name) = &pin.item_name {
                            *self.outputs.entry(name.clone()).or_insert(FractionalNumber::default()) += pin.current_rate;
                        }
                    }
                }
                GroupedNodeData::Group { ins, outs, .. } => {
                    // Use the stored group pins for nested groups
                    for pin in ins {
                        if let Some(name) = &pin.item_name {
                            *self.inputs.entry(name.clone()).or_insert(FractionalNumber::default()) += pin.current_rate;
                        }
                    }
                    for pin in outs {
                        if let Some(name) = &pin.item_name {
                            *self.outputs.entry(name.clone()).or_insert(FractionalNumber::default()) += pin.current_rate;
                        }
                    }
                }
                GroupedNodeData::Sink { ins, .. } => {
                    for pin in ins {
                        if let Some(name) = &pin.item_name {
                            *self.inputs.entry(name.clone()).or_insert(FractionalNumber::default()) += pin.current_rate;
                        }
                    }
                }
                GroupedNodeData::Organizer { .. } => {
                    // Organizers don't contribute net inputs/outputs
                }
            }
        }

        // Create input pins for items that are net consumed (sorted alphabetically)
        let mut input_keys: Vec<_> = self.inputs.keys().cloned().collect();
        input_keys.sort();
        for item_name in input_keys {
            let consumed = self.inputs.get(&item_name).cloned().unwrap_or_default();
            let produced = self.outputs.get(&item_name).cloned().unwrap_or_default();
            if consumed > produced {
                let net = consumed - produced;
                let mut pin = crate::pin::Pin::new(
                    get_id(),
                    crate::pin::PinDirection::Input,
                    self.base.id,
                    Some(item_name.clone()),
                    false,
                    net,
                );
                pin.current_rate = net;
                self.base.ins.push(pin);
            }
        }

        // Create output pins for items that are net produced (sorted alphabetically)
        let mut output_keys: Vec<_> = self.outputs.keys().cloned().collect();
        output_keys.sort();
        for item_name in output_keys {
            let produced = self.outputs.get(&item_name).cloned().unwrap_or_default();
            let consumed = self.inputs.get(&item_name).cloned().unwrap_or_default();
            if produced > consumed {
                let net = produced - consumed;
                let mut pin = crate::pin::Pin::new(
                    get_id(),
                    crate::pin::PinDirection::Output,
                    self.base.id,
                    Some(item_name.clone()),
                    false,
                    net,
                );
                pin.current_rate = net;
                self.base.outs.push(pin);
            }
        }
    }

    /// Update the group rate and propagate to contained nodes
    pub fn update_rate(&mut self, new_rate: FractionalNumber) {
        self.current_rate = new_rate;
        
        // Update pins proportionally
        for pin in &mut self.base.ins {
            pin.current_rate = pin.base_rate * new_rate;
        }
        for pin in &mut self.base.outs {
            pin.current_rate = pin.base_rate * new_rate;
        }

        // Update internal grouped nodes' rates
        for (i, grouped_node) in self.grouped_nodes.iter_mut().enumerate() {
            let base_rate = self.nodes_base_rate.get(i).cloned().unwrap_or_default();
            let scaled_rate = base_rate * new_rate;
            
            match &mut grouped_node.node_data {
                GroupedNodeData::Craft { current_rate, ins, outs, .. } => {
                    *current_rate = scaled_rate;
                    // Scale internal pins too
                    for pin in ins.iter_mut() {
                        pin.current_rate = pin.base_rate * new_rate;
                    }
                    for pin in outs.iter_mut() {
                        pin.current_rate = pin.base_rate * new_rate;
                    }
                }
                GroupedNodeData::Organizer { ins, outs, .. } => {
                    for pin in ins.iter_mut() {
                        pin.current_rate = pin.base_rate * new_rate;
                    }
                    for pin in outs.iter_mut() {
                        pin.current_rate = pin.base_rate * new_rate;
                    }
                }
                GroupedNodeData::Sink { ins, .. } => {
                    for pin in ins.iter_mut() {
                        pin.current_rate = pin.base_rate * new_rate;
                    }
                }
                GroupedNodeData::Group { current_rate, ins, outs, .. } => {
                    *current_rate = scaled_rate;
                    for pin in ins.iter_mut() {
                        pin.current_rate = pin.base_rate * new_rate;
                    }
                    for pin in outs.iter_mut() {
                        pin.current_rate = pin.base_rate * new_rate;
                    }
                }
            }
        }

        self.compute_power_usage();
        self.update_details();
    }

    /// Compute total power usage from all contained nodes
    pub fn compute_power_usage(&mut self) {
        self.same_clock_power = FractionalNumber::default();
        self.last_underclock_power = FractionalNumber::default();
        self.variable_power = false;

        for (i, grouped_node) in self.grouped_nodes.iter().enumerate() {
            if let GroupedNodeData::Craft { 
                recipe_power, 
                power_exponent, 
                somersloop_power_exponent,
                somersloop_mult,
                num_somersloop,
                variable_power,
                ..
            } = &grouped_node.node_data {
                // Compute power for this craft node at its current rate
                let base_rate = self.nodes_base_rate.get(i).cloned().unwrap_or_default();
                let rate_value = (base_rate * self.current_rate).value();
                
                if rate_value > 0.0 {
                    let num_machines = rate_value.ceil().max(1.0);
                    let num_full_machines = rate_value.floor().max(0.0);
                    
                    let boost = 1.0 + num_somersloop.value() * somersloop_mult.value();
                    let boost_pow = boost.powf(*somersloop_power_exponent);
                    
                    let same_clock = num_machines * recipe_power * boost_pow * (rate_value / num_machines).powf(*power_exponent);
                    let mut last_underclock = num_full_machines * recipe_power * boost_pow;
                    let fractional = rate_value - num_full_machines;
                    if fractional > 0.0 {
                        last_underclock += recipe_power * boost_pow * fractional.powf(*power_exponent);
                    }
                    
                    self.same_clock_power += FractionalNumber::new((same_clock * 1000.0).round() as i64, 1000);
                    self.last_underclock_power += FractionalNumber::new((last_underclock * 1000.0).round() as i64, 1000);
                    self.variable_power |= *variable_power;
                }
            }
        }
    }

    /// Update detailed machine counts and other statistics
    pub fn update_details(&mut self) {
        self.total_machines.clear();
        self.built_machines.clear();
        self.detailed_sinked_points.clear();

        for (i, grouped_node) in self.grouped_nodes.iter().enumerate() {
            match &grouped_node.node_data {
                GroupedNodeData::Craft { building_name, built, .. } => {
                    let base_rate = self.nodes_base_rate.get(i).cloned().unwrap_or_default();
                    let current = base_rate * self.current_rate;
                    
                    *self.total_machines.entry(building_name.clone()).or_default() += current;
                    if *built {
                        *self.built_machines.entry(building_name.clone()).or_default() += current;
                    }
                }
                GroupedNodeData::Group { .. } => {
                    // TODO: Recursively accumulate from nested groups
                }
                _ => {}
            }
        }
    }

    /// Set built state on all contained craft nodes
    pub fn set_built_state(&mut self, built: bool) {
        for grouped_node in &mut self.grouped_nodes {
            if let GroupedNodeData::Craft { built: node_built, .. } = &mut grouped_node.node_data {
                *node_built = built;
            } else if let GroupedNodeData::Group { nodes, .. } = &mut grouped_node.node_data {
                // Recursively set built state on nested groups
                for nested in nodes {
                    if let GroupedNodeData::Craft { built: nested_built, .. } = &mut nested.node_data {
                        *nested_built = built;
                    }
                }
            }
        }
        self.update_details();
    }

    /// Check if all contained craft nodes are built
    pub fn is_fully_built(&self) -> bool {
        for grouped_node in &self.grouped_nodes {
            match &grouped_node.node_data {
                GroupedNodeData::Craft { built, .. } => {
                    if !built {
                        return false;
                    }
                }
                GroupedNodeData::Group { nodes, .. } => {
                    for nested in nodes {
                        if let GroupedNodeData::Craft { built, .. } = &nested.node_data {
                            if !built {
                                return false;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        true
    }

    /// Count (built, total) craft nodes in this group
    pub fn count_craft_nodes(&self) -> (usize, usize) {
        let mut built = 0usize;
        let mut total = 0usize;
        
        for grouped_node in &self.grouped_nodes {
            match &grouped_node.node_data {
                GroupedNodeData::Craft { built: is_built, .. } => {
                    total += 1;
                    if *is_built {
                        built += 1;
                    }
                }
                GroupedNodeData::Group { nodes, .. } => {
                    for nested in nodes {
                        if let GroupedNodeData::Craft { built: is_built, .. } = &nested.node_data {
                            total += 1;
                            if *is_built {
                                built += 1;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        
        (built, total)
    }
}

/// A sink node that collects items
#[derive(Debug, Clone)]
pub struct SinkNode {
    pub base: Node,
    pub item_name: Option<String>,
}

impl SinkNode {
    pub fn new(id: u64, item_name: Option<String>) -> Self {
        Self {
            base: Node::new(id, NodeKind::Sink),
            item_name,
        }
    }
}
