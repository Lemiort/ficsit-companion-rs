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
    pub built: bool, // Tracking for factory building progress
    pub building_name: String, // Building name from recipe
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
        let boost = 1.0
            + self.num_somersloop.value() * self.somersloop_mult.value();
        let boost_pow = boost.powf(self.somersloop_power_exponent);

        // Same-clock scenario: all machines at identical clock
        let same_clock_power = num_machines
            * self.recipe_power
            * boost_pow
            * (rate_value / num_machines).powf(self.power_exponent);

        // Last-underclock scenario: full machines plus one partial
        let mut last_underclock_power = num_full_machines
            * self.recipe_power
            * boost_pow;
        let fractional_machine = rate_value - num_full_machines;
        if fractional_machine > 0.0 {
            last_underclock_power += self.recipe_power
                * boost_pow
                * fractional_machine.powf(self.power_exponent);
        }

        let to_fraction = |value: f64| -> FractionalNumber {
            let rounded = (value * 1000.0).round() as i64;
            FractionalNumber::new(rounded, 1000)
        };

        (to_fraction(same_clock_power), to_fraction(last_underclock_power))
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

/// A group node that contains other nodes
#[derive(Debug, Clone)]
pub struct GroupNode {
    pub base: Node,
    pub current_rate: FractionalNumber,
    pub same_clock_power: FractionalNumber,
    pub last_underclock_power: FractionalNumber,
    pub nodes: Vec<u64>, // IDs of contained nodes
    pub inputs: HashMap<String, FractionalNumber>,
    pub outputs: HashMap<String, FractionalNumber>,
    pub detailed_sinked_points: HashMap<String, FractionalNumber>,
    pub loading_error: bool,
}

impl GroupNode {
    pub fn new(id: u64) -> Self {
        Self {
            base: Node::new(id, NodeKind::Group),
            current_rate: FractionalNumber::default(),
            same_clock_power: FractionalNumber::default(),
            last_underclock_power: FractionalNumber::default(),
            nodes: Vec::new(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            detailed_sinked_points: HashMap::new(),
            loading_error: false,
        }
    }

    pub fn add_contained_node(&mut self, node_id: u64) {
        self.nodes.push(node_id);
    }

    pub fn set_built_state(&mut self, _built: bool) {
        // Implementation for tracking group building progress
    }

    pub fn update_details(&mut self) {
        // Recalculate input/output requirements
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
