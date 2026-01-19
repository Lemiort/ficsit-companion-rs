use crate::fractional_number::FractionalNumber;
use crate::game_data::GameData;
use crate::link::Link;
use crate::node::{CraftNode, GroupNode, NodeKind, OrganizerNode, SinkNode};
use crate::pin::{Pin, PinDirection};
use crate::serialization::{
    ProductionChainFile, SerializedCraftNode, SerializedLink, SerializedLinkEndpoint,
    SerializedNode, SerializedOrganizerNode, SerializedPosition, SerializedSinkInput,
    SerializedSinkNode,
};

/// Main production app that manages the graph of nodes and links
pub struct ProductionApp {
    /// All nodes in the graph
    pub nodes: Vec<Box<dyn std::any::Any>>,
    /// All links in the graph
    pub links: Vec<Link>,
    /// Next available ID for nodes/links/pins
    pub next_id: u64,
}

impl ProductionApp {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            links: Vec::new(),
            next_id: 1,
        }
    }

    /// Get the next available ID
    pub fn get_next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Find a pin by its ID across all nodes (returns node_id, direction, pin_index_within_direction)
    pub fn find_pin_location(&self, pin_id: u64) -> Option<(u64, PinDirection, usize)> {
        for (_node_idx, node_any) in self.nodes.iter().enumerate() {
            if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                for (pin_idx, p) in n.base.ins.iter().enumerate() {
                    if p.id == pin_id {
                        return Some((n.base.id, PinDirection::Input, pin_idx));
                    }
                }
                for (pin_idx, p) in n.base.outs.iter().enumerate() {
                    if p.id == pin_id {
                        return Some((n.base.id, PinDirection::Output, pin_idx));
                    }
                }
            } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                for (pin_idx, p) in n.base.ins.iter().enumerate() {
                    if p.id == pin_id {
                        return Some((n.base.id, PinDirection::Input, pin_idx));
                    }
                }
                for (pin_idx, p) in n.base.outs.iter().enumerate() {
                    if p.id == pin_id {
                        return Some((n.base.id, PinDirection::Output, pin_idx));
                    }
                }
            } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
                for (pin_idx, p) in n.base.ins.iter().enumerate() {
                    if p.id == pin_id {
                        return Some((n.base.id, PinDirection::Input, pin_idx));
                    }
                }
                for (pin_idx, p) in n.base.outs.iter().enumerate() {
                    if p.id == pin_id {
                        return Some((n.base.id, PinDirection::Output, pin_idx));
                    }
                }
            } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
                for (pin_idx, p) in n.base.ins.iter().enumerate() {
                    if p.id == pin_id {
                        return Some((n.base.id, PinDirection::Input, pin_idx));
                    }
                }
            }
        }
        None
    }

    /// Find node index by node ID
    fn find_node_index(&self, node_id: u64) -> Option<usize> {
        for (idx, node_any) in self.nodes.iter().enumerate() {
            let id = if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                n.base.id
            } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                n.base.id
            } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
                n.base.id
            } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
                n.base.id
            } else {
                continue;
            };

            if id == node_id {
                return Some(idx);
            }
        }
        None
    }

    /// Get pin item names for a node (inputs, outputs)
    pub fn get_node_pin_item_names(
        &self,
        node_id: u64,
    ) -> Option<(Vec<Option<String>>, Vec<Option<String>>)> {
        let idx = self.find_node_index(node_id)?;
        let node_any = &self.nodes[idx];
        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
            let ins = n.base.ins.iter().map(|p| p.item_name.clone()).collect();
            let outs = n.base.outs.iter().map(|p| p.item_name.clone()).collect();
            return Some((ins, outs));
        } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
            let ins = n.base.ins.iter().map(|p| p.item_name.clone()).collect();
            let outs = n.base.outs.iter().map(|p| p.item_name.clone()).collect();
            return Some((ins, outs));
        } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
            let ins = n.base.ins.iter().map(|p| p.item_name.clone()).collect();
            let outs = n.base.outs.iter().map(|p| p.item_name.clone()).collect();
            return Some((ins, outs));
        } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
            let ins = n.base.ins.iter().map(|p| p.item_name.clone()).collect();
            let outs: Vec<Option<String>> = Vec::new();
            return Some((ins, outs));
        }
        None
    }

    /// Get pin rates for a node (inputs, outputs) as float strings (e.g., "1.000")
    pub fn get_node_pin_rates(
        &self,
        node_id: u64,
    ) -> Option<(Vec<Option<String>>, Vec<Option<String>>)> {
        let idx = self.find_node_index(node_id)?;
        let node_any = &self.nodes[idx];
        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
            let ins = n
                .base
                .ins
                .iter()
                .map(|p| Some(p.current_rate.to_float_string()))
                .collect();
            let outs = n
                .base
                .outs
                .iter()
                .map(|p| Some(p.current_rate.to_float_string()))
                .collect();
            return Some((ins, outs));
        } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
            let ins = n
                .base
                .ins
                .iter()
                .map(|p| Some(p.current_rate.to_float_string()))
                .collect();
            let outs = n
                .base
                .outs
                .iter()
                .map(|p| Some(p.current_rate.to_float_string()))
                .collect();
            return Some((ins, outs));
        } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
            let ins = n
                .base
                .ins
                .iter()
                .map(|p| Some(p.current_rate.to_float_string()))
                .collect();
            let outs = n
                .base
                .outs
                .iter()
                .map(|p| Some(p.current_rate.to_float_string()))
                .collect();
            return Some((ins, outs));
        } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
            let ins = n
                .base
                .ins
                .iter()
                .map(|p| Some(p.current_rate.to_float_string()))
                .collect();
            let outs: Vec<Option<String>> = Vec::new();
            return Some((ins, outs));
        }
        None
    }

    /// Get pin locked flags for a node (inputs, outputs)
    pub fn get_node_pin_locked_flags(&self, node_id: u64) -> Option<(Vec<bool>, Vec<bool>)> {
        let idx = self.find_node_index(node_id)?;
        let node_any = &self.nodes[idx];
        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
            let ins = n.base.ins.iter().map(|p| p.locked).collect();
            let outs = n.base.outs.iter().map(|p| p.locked).collect();
            return Some((ins, outs));
        } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
            let ins = n.base.ins.iter().map(|p| p.locked).collect();
            let outs = n.base.outs.iter().map(|p| p.locked).collect();
            return Some((ins, outs));
        } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
            let ins = n.base.ins.iter().map(|p| p.locked).collect();
            let outs = n.base.outs.iter().map(|p| p.locked).collect();
            return Some((ins, outs));
        } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
            let ins = n.base.ins.iter().map(|p| p.locked).collect();
            let outs: Vec<bool> = Vec::new();
            return Some((ins, outs));
        }
        None
    }

    pub fn get_node_building_info(&self, node_id: u64) -> Option<(String, String)> {
        let idx = self.find_node_index(node_id)?;
        let node_any = &self.nodes[idx];
        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
            // building_count_str is the current_rate formatted as a string
            let count_str = n.current_rate.to_string();
            let name = n.building_name.clone();
            return Some((count_str, name));
        }
        None
    }

    pub fn get_node_power_info(&self, node_id: u64) -> Option<(String, String, bool)> {
        let idx = self.find_node_index(node_id)?;
        let node_any = &self.nodes[idx];
        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
            let (same, last) = n.compute_power_usage();
            return Some((
                same.to_float_string(),
                last.to_float_string(),
                n.variable_power,
            ));
        }
        None
    }

    /// Get somersloop info for a craft node: (num_somersloop_str, somersloop_mult)
    pub fn get_node_somersloop_info(
        &self,
        node_id: u64,
    ) -> Option<(String, Option<crate::fractional_number::FractionalNumber>)> {
        let idx = self.find_node_index(node_id)?;
        let node_any = &self.nodes[idx];
        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
            return Some((
                n.num_somersloop.to_fraction_string(),
                Some(n.somersloop_mult),
            ));
        }
        None
    }

    /// Set the somersloop count for a craft node. Only positive whole integers allowed.
    pub fn set_node_somersloop(
        &mut self,
        node_id: u64,
        new_num: crate::fractional_number::FractionalNumber,
    ) -> Result<(), String> {
        // Validate integer non-negative
        if new_num.denominator() != 1 || new_num.numerator() < 0 {
            return Err("somersloop num can only be positive whole integers".into());
        }
        let idx = self
            .find_node_index(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;
        let node_any = &mut self.nodes[idx];
        if let Some(n) = node_any.downcast_mut::<CraftNode>() {
            // Cap to 1 / somersloop_mult if somersloop_mult isn't zero
            if n.somersloop_mult.numerator() != 0 {
                let max_num =
                    crate::fractional_number::FractionalNumber::new(1, 1) / n.somersloop_mult;
                let mut capped = new_num;
                if capped > max_num {
                    capped = max_num;
                }
                n.num_somersloop = capped;
            } else {
                n.num_somersloop = new_num;
            }
            // Recompute rates for this node based on new boost level
            n.update_rate(n.current_rate);
            return Ok(());
        }
        Err("Unsupported node kind for somersloop edit".into())
    }

    /// Get build progress for a group node: (built_count, total_craft_nodes)
    pub fn get_node_build_progress(&self, node_id: u64) -> Option<(usize, usize)> {
        let start_idx = self.find_node_index(node_id)?;
        // We will do a simple DFS over group children to count craft nodes and how many are built
        let mut stack = vec![node_id];
        let mut built = 0usize;
        let mut total = 0usize;
        while let Some(curr_id) = stack.pop() {
            if let Some(idx) = self.find_node_index(curr_id) {
                let node_any = &self.nodes[idx];
                if let Some(craft) = node_any.downcast_ref::<CraftNode>() {
                    total += 1;
                    if craft.built {
                        built += 1;
                    }
                } else if let Some(group) = node_any.downcast_ref::<GroupNode>() {
                    for child in &group.nodes {
                        stack.push(*child);
                    }
                }
            }
        }
        Some((built, total))
    }

    /// Set the built state for a node (craft or group). For groups this will propagate to contained craft nodes.
    pub fn set_node_built_state(&mut self, node_id: u64, built: bool) -> Result<(), String> {
        let idx = self
            .find_node_index(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;
        let node_any = &mut self.nodes[idx];
        if let Some(craft) = node_any.downcast_mut::<CraftNode>() {
            craft.built = built;
            return Ok(());
        }
        if let Some(group) = node_any.downcast_mut::<GroupNode>() {
            // Propagate to all contained nodes (recursive)
            let children = group.nodes.clone();
            for child_id in children {
                // ignore errors for missing children
                let _ = self.set_node_built_state(child_id, built);
            }
            return Ok(());
        }
        Err("Unsupported node kind for set built state".into())
    }

    /// Apply a new rate typed by the user into a pin. Performs simple validation
    /// and, for some node kinds (e.g., Craft), derives and applies a new node rate.
    pub fn set_pin_rate(
        &mut self,
        node_id: u64,
        direction: PinDirection,
        pin_index: usize,
        new_rate: FractionalNumber,
    ) -> Result<(), String> {
        // Validate rate first
        if !crate::rate_calculator::validate_rate(&new_rate) {
            return Err("Invalid rate".into());
        }

        let node_idx = self
            .find_node_index(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;
        let node_any = &mut self.nodes[node_idx];

        // CraftNode: if setting an output's rate, derive the node rate
        if let Some(n) = node_any.downcast_mut::<CraftNode>() {
            match direction {
                PinDirection::Output => {
                    if pin_index >= n.base.outs.len() {
                        return Err("Output pin out of range".into());
                    }
                    let base_rate = n.base.outs[pin_index].base_rate;
                    if base_rate.numerator() == 0 {
                        return Err("Base rate is zero".into());
                    }
                    let new_node_rate = new_rate / base_rate;
                    if !crate::rate_calculator::validate_rate(&new_node_rate) {
                        return Err("Derived node rate invalid".into());
                    }
                    n.update_rate(new_node_rate);
                    return Ok(());
                }
                PinDirection::Input => {
                    if pin_index >= n.base.ins.len() {
                        return Err("Input pin out of range".into());
                    }
                    n.base.ins[pin_index].current_rate = new_rate;
                    return Ok(());
                }
            }
        }

        // Organizer nodes: set directly
        if let Some(n) = node_any.downcast_mut::<OrganizerNode>() {
            match direction {
                PinDirection::Input => {
                    if pin_index >= n.base.ins.len() {
                        return Err("Input pin out of range".into());
                    }
                    n.base.ins[pin_index].current_rate = new_rate;
                    return Ok(());
                }
                PinDirection::Output => {
                    if pin_index >= n.base.outs.len() {
                        return Err("Output pin out of range".into());
                    }
                    n.base.outs[pin_index].current_rate = new_rate;
                    return Ok(());
                }
            }
        }

        // Group/Sink: set directly
        if let Some(n) = node_any.downcast_mut::<GroupNode>() {
            match direction {
                PinDirection::Input => {
                    if pin_index >= n.base.ins.len() {
                        return Err("Input pin out of range".into());
                    }
                    n.base.ins[pin_index].current_rate = new_rate;
                    return Ok(());
                }
                PinDirection::Output => {
                    if pin_index >= n.base.outs.len() {
                        return Err("Output pin out of range".into());
                    }
                    n.base.outs[pin_index].current_rate = new_rate;
                    return Ok(());
                }
            }
        }

        if let Some(n) = node_any.downcast_mut::<SinkNode>() {
            if direction != PinDirection::Input {
                return Err("Sink has no outputs".into());
            }
            if pin_index >= n.base.ins.len() {
                return Err("Input pin out of range".into());
            }
            n.base.ins[pin_index].current_rate = new_rate;
            return Ok(());
        }

        Err("Unsupported node kind for rate edit".into())
    }

    /// Add a craft node with the given recipe name
    pub fn add_craft_node(
        &mut self,
        recipe_name: &str,
        game_data: &GameData,
    ) -> Result<u64, String> {
        // Find recipe by name
        let recipe = game_data
            .recipes
            .iter()
            .find(|r| r.name == recipe_name)
            .ok_or_else(|| format!("Recipe '{}' not found", recipe_name))?;

        let node_id = self.get_next_id();
        let mut craft_node = CraftNode::new(node_id, recipe_name.to_string());

        // Set building name and recipe power from recipe
        craft_node.building_name = recipe.building_name.clone();
        craft_node.recipe_power = recipe.power;
        if let Some(building) = &recipe.building {
            craft_node.power_exponent = building.power_exponent;
            craft_node.somersloop_power_exponent = building.somersloop_power_exponent;
            craft_node.somersloop_mult = building.somersloop_mult.clone();
            craft_node.variable_power = building.variable_power;
        }

        // Create input pins (use recipe quantities as base_rate)
        for item in &recipe.ins {
            let pin_id = self.get_next_id();
            craft_node.base.ins.push(Pin::new(
                pin_id,
                PinDirection::Input,
                node_id,
                Some(item.item_name.clone()),
                false,
                item.quantity.clone(),
            ));
        }

        // Create output pins (use recipe quantities as base_rate)
        for item in &recipe.outs {
            let pin_id = self.get_next_id();
            craft_node.base.outs.push(Pin::new(
                pin_id,
                PinDirection::Output,
                node_id,
                Some(item.item_name.clone()),
                false,
                item.quantity.clone(),
            ));
        }

        // Default to 1 building (so pins show per-recipe rates immediately)
        craft_node.update_rate(FractionalNumber::new(1, 1));

        self.nodes.push(Box::new(craft_node));
        Ok(node_id)
    }

    /// Add a merger node
    pub fn add_merger_node(&mut self) -> u64 {
        let node_id = self.get_next_id();
        let mut merger = OrganizerNode::new(node_id, NodeKind::Merger, None);

        // Create 2 input pins and 1 output pin
        for _ in 0..2 {
            let pin_id = self.get_next_id();
            merger.base.ins.push(Pin::new(
                pin_id,
                PinDirection::Input,
                node_id,
                None,
                false,
                FractionalNumber::default(),
            ));
        }

        let out_pin_id = self.get_next_id();
        merger.base.outs.push(Pin::new(
            out_pin_id,
            PinDirection::Output,
            node_id,
            None,
            false,
            FractionalNumber::default(),
        ));

        self.nodes.push(Box::new(merger));
        node_id
    }

    /// Add a custom splitter node
    pub fn add_custom_splitter_node(&mut self) -> u64 {
        let node_id = self.get_next_id();
        let mut splitter = OrganizerNode::new(node_id, NodeKind::CustomSplitter, None);

        // Create 1 input pin and 2 output pins
        let in_pin_id = self.get_next_id();
        splitter.base.ins.push(Pin::new(
            in_pin_id,
            PinDirection::Input,
            node_id,
            None,
            false,
            FractionalNumber::default(),
        ));

        for _ in 0..2 {
            let pin_id = self.get_next_id();
            splitter.base.outs.push(Pin::new(
                pin_id,
                PinDirection::Output,
                node_id,
                None,
                false,
                FractionalNumber::default(),
            ));
        }

        self.nodes.push(Box::new(splitter));
        node_id
    }

    /// Add a game splitter node (equal distribution)
    pub fn add_game_splitter_node(&mut self) -> u64 {
        let node_id = self.get_next_id();
        let mut splitter = OrganizerNode::new(node_id, NodeKind::GameSplitter, None);

        // Create 1 input pin and 2 output pins
        let in_pin_id = self.get_next_id();
        splitter.base.ins.push(Pin::new(
            in_pin_id,
            PinDirection::Input,
            node_id,
            None,
            false,
            FractionalNumber::default(),
        ));

        for _ in 0..2 {
            let pin_id = self.get_next_id();
            splitter.base.outs.push(Pin::new(
                pin_id,
                PinDirection::Output,
                node_id,
                None,
                false,
                FractionalNumber::default(),
            ));
        }

        self.nodes.push(Box::new(splitter));
        node_id
    }

    /// Add a sink node
    pub fn add_sink_node(&mut self) -> u64 {
        let node_id = self.get_next_id();
        let mut sink = SinkNode::new(node_id, None);

        // Create 1 input pin
        let pin_id = self.get_next_id();
        sink.base.ins.push(Pin::new(
            pin_id,
            PinDirection::Input,
            node_id,
            None,
            false,
            FractionalNumber::default(),
        ));

        self.nodes.push(Box::new(sink));
        node_id
    }

    /// Delete a node and all its links
    pub fn delete_node(&mut self, node_id: u64) -> Result<(), String> {
        let node_idx = self
            .find_node_index(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;

        // Find all pins for this node
        let mut pin_ids = Vec::new();
        let node_any = &self.nodes[node_idx];

        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
            pin_ids.extend(n.base.ins.iter().map(|p| p.id));
            pin_ids.extend(n.base.outs.iter().map(|p| p.id));
        } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
            pin_ids.extend(n.base.ins.iter().map(|p| p.id));
            pin_ids.extend(n.base.outs.iter().map(|p| p.id));
        } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
            pin_ids.extend(n.base.ins.iter().map(|p| p.id));
            pin_ids.extend(n.base.outs.iter().map(|p| p.id));
        } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
            pin_ids.extend(n.base.ins.iter().map(|p| p.id));
        }

        // Delete all links connected to this node's pins
        let mut links_to_delete = Vec::new();
        for pin_id in pin_ids {
            if let Some(link) = self.find_link_by_pin(pin_id) {
                links_to_delete.push(link.id);
            }
        }

        for link_id in links_to_delete {
            self.delete_link(link_id)?;
        }

        // Remove the node
        self.nodes.remove(node_idx);
        Ok(())
    }

    /// Find a link by pin ID
    pub fn find_link_by_pin(&self, pin_id: u64) -> Option<&Link> {
        self.links
            .iter()
            .find(|l| l.start_pin_id == pin_id || l.end_pin_id == pin_id)
    }

    /// Create a link between two pins
    pub fn create_link(&mut self, start_pin_id: u64, end_pin_id: u64) -> Result<u64, String> {
        // Validate pins exist
        let _start_loc = self
            .find_pin_location(start_pin_id)
            .ok_or("Start pin not found")?;
        let _end_loc = self
            .find_pin_location(end_pin_id)
            .ok_or("End pin not found")?;

        // Create link
        let link_id = self.get_next_id();
        let link = Link::new(link_id, start_pin_id, end_pin_id);
        self.links.push(link);

        // Update pin link_id references
        if let Some((node_id, direction, pi)) = self.find_pin_location(start_pin_id) {
            let ni = self.find_node_index(node_id).unwrap();
            if let Some(n) = self.nodes[ni].downcast_mut::<CraftNode>() {
                let pins = match direction {
                    PinDirection::Input => &mut n.base.ins,
                    PinDirection::Output => &mut n.base.outs,
                };
                pins[pi].link_id = Some(link_id);
            } else if let Some(n) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
                let pins = match direction {
                    PinDirection::Input => &mut n.base.ins,
                    PinDirection::Output => &mut n.base.outs,
                };
                pins[pi].link_id = Some(link_id);
            } else if let Some(n) = self.nodes[ni].downcast_mut::<GroupNode>() {
                let pins = match direction {
                    PinDirection::Input => &mut n.base.ins,
                    PinDirection::Output => &mut n.base.outs,
                };
                pins[pi].link_id = Some(link_id);
            } else if let Some(n) = self.nodes[ni].downcast_mut::<SinkNode>() {
                n.base.ins[pi].link_id = Some(link_id);
            }
        }

        if let Some((node_id, direction, pi)) = self.find_pin_location(end_pin_id) {
            let ni = self.find_node_index(node_id).unwrap();
            if let Some(n) = self.nodes[ni].downcast_mut::<CraftNode>() {
                let pins = match direction {
                    PinDirection::Input => &mut n.base.ins,
                    PinDirection::Output => &mut n.base.outs,
                };
                pins[pi].link_id = Some(link_id);
            } else if let Some(n) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
                let pins = match direction {
                    PinDirection::Input => &mut n.base.ins,
                    PinDirection::Output => &mut n.base.outs,
                };
                pins[pi].link_id = Some(link_id);
            } else if let Some(n) = self.nodes[ni].downcast_mut::<GroupNode>() {
                let pins = match direction {
                    PinDirection::Input => &mut n.base.ins,
                    PinDirection::Output => &mut n.base.outs,
                };
                pins[pi].link_id = Some(link_id);
            } else if let Some(n) = self.nodes[ni].downcast_mut::<SinkNode>() {
                n.base.ins[pi].link_id = Some(link_id);
            }
        }

        Ok(link_id)
    }

    /// Delete a link by ID
    pub fn delete_link(&mut self, link_id: u64) -> Result<(), String> {
        let link_idx = self
            .links
            .iter()
            .position(|l| l.id == link_id)
            .ok_or("Link not found")?;

        let link = self.links.remove(link_idx);

        // Clear pin link_id references
        if let Some((node_id, direction, pi)) = self.find_pin_location(link.start_pin_id) {
            let ni = self.find_node_index(node_id).unwrap();
            if let Some(n) = self.nodes[ni].downcast_mut::<CraftNode>() {
                let pins = match direction {
                    PinDirection::Input => &mut n.base.ins,
                    PinDirection::Output => &mut n.base.outs,
                };
                pins[pi].link_id = None;
            } else if let Some(n) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
                let pins = match direction {
                    PinDirection::Input => &mut n.base.ins,
                    PinDirection::Output => &mut n.base.outs,
                };
                pins[pi].link_id = None;
            } else if let Some(n) = self.nodes[ni].downcast_mut::<GroupNode>() {
                let pins = match direction {
                    PinDirection::Input => &mut n.base.ins,
                    PinDirection::Output => &mut n.base.outs,
                };
                pins[pi].link_id = None;
            } else if let Some(n) = self.nodes[ni].downcast_mut::<SinkNode>() {
                n.base.ins[pi].link_id = None;
            }
        }

        if let Some((node_id, direction, pi)) = self.find_pin_location(link.end_pin_id) {
            let ni = self.find_node_index(node_id).unwrap();
            if let Some(n) = self.nodes[ni].downcast_mut::<CraftNode>() {
                let pins = match direction {
                    PinDirection::Input => &mut n.base.ins,
                    PinDirection::Output => &mut n.base.outs,
                };
                pins[pi].link_id = None;
            } else if let Some(n) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
                let pins = match direction {
                    PinDirection::Input => &mut n.base.ins,
                    PinDirection::Output => &mut n.base.outs,
                };
                pins[pi].link_id = None;
            } else if let Some(n) = self.nodes[ni].downcast_mut::<GroupNode>() {
                let pins = match direction {
                    PinDirection::Input => &mut n.base.ins,
                    PinDirection::Output => &mut n.base.outs,
                };
                pins[pi].link_id = None;
            } else if let Some(n) = self.nodes[ni].downcast_mut::<SinkNode>() {
                n.base.ins[pi].link_id = None;
            }
        }

        Ok(())
    }

    /// Add an input pin to a node (used by UI + button)
    pub fn add_input_pin_to_node(&mut self, node_id: u64) -> Result<(), String> {
        let ni = self
            .find_node_index(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;
        // Allocate id before borrowing node mutably
        let pin_id = self.get_next_id();
        if let Some(n) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
            let locked = n.base.outs.get(0).map(|p| p.locked).unwrap_or(false);
            n.base.ins.push(Pin::new(
                pin_id,
                PinDirection::Input,
                node_id,
                n.item_name.clone(),
                locked,
                FractionalNumber::default(),
            ));
            Ok(())
        } else if let Some(n) = self.nodes[ni].downcast_mut::<SinkNode>() {
            n.base.ins.push(Pin::new(
                pin_id,
                PinDirection::Input,
                node_id,
                None,
                false,
                FractionalNumber::default(),
            ));
            Ok(())
        } else {
            Err("Unsupported node kind for add input".into())
        }
    }

    /// Add an output pin to a node (used by UI + button)
    pub fn add_output_pin_to_node(&mut self, node_id: u64) -> Result<(), String> {
        let ni = self
            .find_node_index(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;
        // Allocate id before borrowing node mutably
        let pin_id = self.get_next_id();
        if let Some(n) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
            let locked = n.base.ins.get(0).map(|p| p.locked).unwrap_or(false);
            n.base.outs.push(Pin::new(
                pin_id,
                PinDirection::Output,
                node_id,
                n.item_name.clone(),
                locked,
                FractionalNumber::default(),
            ));
            Ok(())
        } else {
            Err("Unsupported node kind for add output".into())
        }
    }

    /// Set the position of a node by id
    pub fn set_node_position(&mut self, node_id: u64, pos: (f32, f32)) -> Result<(), String> {
        let ni = self
            .find_node_index(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;
        if let Some(n) = self.nodes[ni].downcast_mut::<CraftNode>() {
            n.base.position = pos;
            Ok(())
        } else if let Some(n) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
            n.base.position = pos;
            Ok(())
        } else if let Some(n) = self.nodes[ni].downcast_mut::<GroupNode>() {
            n.base.position = pos;
            Ok(())
        } else if let Some(n) = self.nodes[ni].downcast_mut::<SinkNode>() {
            n.base.position = pos;
            Ok(())
        } else {
            Err("Unsupported node kind for set position".into())
        }
    }

    /// Get a node position by index (used for testing)
    pub fn get_node_position(&self, index: usize) -> Option<(f32, f32)> {
        if index >= self.nodes.len() {
            return None;
        }
        let n = &self.nodes[index];
        if let Some(craft) = n.downcast_ref::<CraftNode>() {
            Some(craft.base.position)
        } else if let Some(org) = n.downcast_ref::<OrganizerNode>() {
            Some(org.base.position)
        } else if let Some(group) = n.downcast_ref::<GroupNode>() {
            Some(group.base.position)
        } else if let Some(sink) = n.downcast_ref::<SinkNode>() {
            Some(sink.base.position)
        } else {
            None
        }
    }

    /// Find a node id by its index in the internal nodes array (used for testing)
    pub fn find_node_by_index(&self, index: usize) -> Option<u64> {
        if index >= self.nodes.len() {
            return None;
        }
        let n = &self.nodes[index];
        if let Some(craft) = n.downcast_ref::<CraftNode>() {
            Some(craft.base.id)
        } else if let Some(org) = n.downcast_ref::<OrganizerNode>() {
            Some(org.base.id)
        } else if let Some(group) = n.downcast_ref::<GroupNode>() {
            Some(group.base.id)
        } else if let Some(sink) = n.downcast_ref::<SinkNode>() {
            Some(sink.base.id)
        } else {
            None
        }
    }

    /// Remove an input pin from a node (used by UI x button)
    pub fn remove_input_pin_from_node(&mut self, node_id: u64, idx: usize) -> Result<(), String> {
        let ni = self
            .find_node_index(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;
        // Read pin id immutably first to avoid double borrowing self
        if let Some(n) = self.nodes[ni].downcast_ref::<OrganizerNode>() {
            if idx >= n.base.ins.len() {
                return Err("Input index out of range".into());
            }
            let pin_id = n.base.ins[idx].id;
            if let Some(link) = self.find_link_by_pin(pin_id) {
                let lid = link.id;
                self.delete_link(lid)?;
            }
            // now remove mutably
            if let Some(nm) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
                nm.base.ins.remove(idx);
                return Ok(());
            }
            unreachable!();
        } else if let Some(n) = self.nodes[ni].downcast_ref::<SinkNode>() {
            if idx >= n.base.ins.len() {
                return Err("Input index out of range".into());
            }
            let pin_id = n.base.ins[idx].id;
            if let Some(link) = self.find_link_by_pin(pin_id) {
                let lid = link.id;
                self.delete_link(lid)?;
            }
            if let Some(nm) = self.nodes[ni].downcast_mut::<SinkNode>() {
                nm.base.ins.remove(idx);
                return Ok(());
            }
            unreachable!();
        } else {
            Err("Unsupported node kind for remove input".into())
        }
    }

    /// Remove an output pin from a node (used by UI x button)
    pub fn remove_output_pin_from_node(&mut self, node_id: u64, idx: usize) -> Result<(), String> {
        let ni = self
            .find_node_index(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;
        // Read pin id immutably first
        if let Some(n) = self.nodes[ni].downcast_ref::<OrganizerNode>() {
            if idx >= n.base.outs.len() {
                return Err("Output index out of range".into());
            }
            let pin_id = n.base.outs[idx].id;
            if let Some(link) = self.find_link_by_pin(pin_id) {
                let lid = link.id;
                self.delete_link(lid)?;
            }
            if let Some(nm) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
                nm.base.outs.remove(idx);
                return Ok(());
            }
            unreachable!();
        } else {
            Err("Unsupported node kind for remove output".into())
        }
    }

    /// Set pin locked state with propagation logic from C++
    pub fn set_pin_locked(&mut self, pin_id: u64, locked: bool) -> Result<(), String> {
        let (node_id, _direction, pin_idx) =
            self.find_pin_location(pin_id).ok_or("Pin not found")?;
        let node_idx = self.find_node_index(node_id).unwrap();

        // Get current locked state, direction, and node kind
        let (current_locked, direction, node_kind) =
            if let Some(n) = self.nodes[node_idx].downcast_ref::<CraftNode>() {
                let pin = n.base.get_pin_by_flat_index(pin_idx).unwrap();
                (pin.locked, pin.direction, n.base.kind)
            } else if let Some(n) = self.nodes[node_idx].downcast_ref::<OrganizerNode>() {
                let pin = n.base.get_pin_by_flat_index(pin_idx).unwrap();
                (pin.locked, pin.direction, n.base.kind)
            } else if let Some(n) = self.nodes[node_idx].downcast_ref::<GroupNode>() {
                let pin = n.base.get_pin_by_flat_index(pin_idx).unwrap();
                (pin.locked, pin.direction, n.base.kind)
            } else if let Some(n) = self.nodes[node_idx].downcast_ref::<SinkNode>() {
                let pin = n.base.get_pin_by_flat_index(pin_idx).unwrap();
                (pin.locked, pin.direction, n.base.kind)
            } else {
                return Err("Invalid node type".to_string());
            };

        // No change needed
        if current_locked == locked {
            return Ok(());
        }

        // Set the pin locked state
        if let Some(n) = self.nodes[node_idx].downcast_mut::<CraftNode>() {
            n.base.get_pin_by_flat_index_mut(pin_idx).unwrap().locked = locked;
        } else if let Some(n) = self.nodes[node_idx].downcast_mut::<OrganizerNode>() {
            n.base.get_pin_by_flat_index_mut(pin_idx).unwrap().locked = locked;
        } else if let Some(n) = self.nodes[node_idx].downcast_mut::<GroupNode>() {
            n.base.get_pin_by_flat_index_mut(pin_idx).unwrap().locked = locked;
        } else if let Some(n) = self.nodes[node_idx].downcast_mut::<SinkNode>() {
            n.base.get_pin_by_flat_index_mut(pin_idx).unwrap().locked = locked;
        }

        // Propagate lock to linked pin
        if let Some(link) = self.find_link_by_pin(pin_id).cloned() {
            let linked_pin_id = if link.start_pin_id == pin_id {
                link.end_pin_id
            } else {
                link.start_pin_id
            };

            // Get linked pin's locked state
            if let Some((linked_node_id, _linked_direction, linked_pin_idx)) =
                self.find_pin_location(linked_pin_id)
            {
                let linked_node_idx = self.find_node_index(linked_node_id).unwrap();
                let linked_locked = if let Some(n) =
                    self.nodes[linked_node_idx].downcast_ref::<CraftNode>()
                {
                    n.base.get_pin_by_flat_index(linked_pin_idx).unwrap().locked
                } else if let Some(n) = self.nodes[linked_node_idx].downcast_ref::<OrganizerNode>()
                {
                    n.base.get_pin_by_flat_index(linked_pin_idx).unwrap().locked
                } else if let Some(n) = self.nodes[linked_node_idx].downcast_ref::<GroupNode>() {
                    n.base.get_pin_by_flat_index(linked_pin_idx).unwrap().locked
                } else if let Some(n) = self.nodes[linked_node_idx].downcast_ref::<SinkNode>() {
                    n.base.get_pin_by_flat_index(linked_pin_idx).unwrap().locked
                } else {
                    false
                };

                if linked_locked != locked {
                    self.set_pin_locked(linked_pin_id, locked)?;
                }
            }
        }

        // Apply node-specific locking rules
        use crate::node::NodeKind;
        use crate::pin::PinDirection;

        match node_kind {
            NodeKind::Craft | NodeKind::Group | NodeKind::GameSplitter => {
                // Lock all pins in the node
                let all_pin_ids: Vec<u64> =
                    if let Some(n) = self.nodes[node_idx].downcast_ref::<CraftNode>() {
                        n.base.all_pins().map(|p| p.id).collect()
                    } else if let Some(n) = self.nodes[node_idx].downcast_ref::<OrganizerNode>() {
                        n.base.all_pins().map(|p| p.id).collect()
                    } else if let Some(n) = self.nodes[node_idx].downcast_ref::<GroupNode>() {
                        n.base.all_pins().map(|p| p.id).collect()
                    } else {
                        Vec::new()
                    };

                for pid in all_pin_ids {
                    if pid != pin_id {
                        let (pid_node_id, _pid_direction, pi) =
                            self.find_pin_location(pid).unwrap();
                        let pid_node_idx = self.find_node_index(pid_node_id).unwrap();
                        let p_locked = if let Some(n) =
                            self.nodes[pid_node_idx].downcast_ref::<CraftNode>()
                        {
                            n.base.get_pin_by_flat_index(pi).unwrap().locked
                        } else if let Some(n) =
                            self.nodes[pid_node_idx].downcast_ref::<OrganizerNode>()
                        {
                            n.base.get_pin_by_flat_index(pi).unwrap().locked
                        } else if let Some(n) = self.nodes[pid_node_idx].downcast_ref::<GroupNode>()
                        {
                            n.base.get_pin_by_flat_index(pi).unwrap().locked
                        } else {
                            false
                        };

                        if p_locked != locked {
                            self.set_pin_locked(pid, locked)?;
                        }
                    }
                }
            }
            NodeKind::Merger | NodeKind::CustomSplitter => {
                // Complex multi-pin logic
                let is_custom_splitter = node_kind == NodeKind::CustomSplitter;

                // Get multi-pin side (outs for CustomSplitter, ins for Merger)
                let (multi_pin_ids, single_pin_id) =
                    if let Some(n) = self.nodes[node_idx].downcast_ref::<OrganizerNode>() {
                        let multi: Vec<u64> = if is_custom_splitter {
                            n.base.outs.iter().map(|p| p.id).collect()
                        } else {
                            n.base.ins.iter().map(|p| p.id).collect()
                        };

                        let single = if is_custom_splitter {
                            n.base.ins.first().map(|p| p.id)
                        } else {
                            n.base.outs.first().map(|p| p.id)
                        };

                        (multi, single)
                    } else {
                        (Vec::new(), None)
                    };

                // Count locked/unlocked multi pins
                let mut all_locked_ids = Vec::new();
                let mut all_unlocked_ids = Vec::new();

                for &mpid in &multi_pin_ids {
                    if let Some((mpid_node_id, _mpid_direction, pi)) = self.find_pin_location(mpid)
                    {
                        let mpid_node_idx = self.find_node_index(mpid_node_id).unwrap();
                        let is_locked = if let Some(n) =
                            self.nodes[mpid_node_idx].downcast_ref::<OrganizerNode>()
                        {
                            n.base.get_pin_by_flat_index(pi).unwrap().locked
                        } else {
                            false
                        };

                        if is_locked {
                            all_locked_ids.push(mpid);
                        } else {
                            all_unlocked_ids.push(mpid);
                        }
                    }
                }

                // Single pin updated (input for CustomSplitter, output for Merger)
                let is_single_side = (direction == PinDirection::Input && is_custom_splitter)
                    || (direction == PinDirection::Output && !is_custom_splitter);

                if is_single_side {
                    // If locked and only one unlocked multi pin remaining, lock it
                    if locked && all_unlocked_ids.len() == 1 {
                        self.set_pin_locked(all_unlocked_ids[0], locked)?;
                    }
                    // If unlocked and all multi pins are locked, unlock all multi pins
                    else if !locked && all_unlocked_ids.is_empty() {
                        for &mpid in &multi_pin_ids {
                            self.set_pin_locked(mpid, locked)?;
                        }
                    }
                } else {
                    // Multi pin updated
                    if let Some(spid) = single_pin_id {
                        let single_locked = if let Some((spid_node_id, _spid_direction, pi)) =
                            self.find_pin_location(spid)
                        {
                            let spid_node_idx = self.find_node_index(spid_node_id).unwrap();
                            if let Some(n) =
                                self.nodes[spid_node_idx].downcast_ref::<OrganizerNode>()
                            {
                                n.base.get_pin_by_flat_index(pi).unwrap().locked
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        // If all multi pins locked, lock single pin
                        if all_unlocked_ids.is_empty() {
                            if !single_locked {
                                self.set_pin_locked(spid, true)?;
                            }
                        }
                        // If we just locked and single is locked with only one unlocked, lock last one
                        else if locked && single_locked && all_unlocked_ids.len() == 1 {
                            self.set_pin_locked(all_unlocked_ids[0], locked)?;
                        }
                        // If we just unlocked, single was locked, and now one unlocked, unlock single
                        else if !locked && single_locked && all_unlocked_ids.len() == 1 {
                            self.set_pin_locked(spid, locked)?;
                        }
                    }
                }
            }
            NodeKind::Sink => {
                // No special logic for sink nodes
            }
        }

        Ok(())
    }

    /// Get count of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get all recipe names (stub for UI)
    pub fn get_recipe_names(&self) -> Vec<String> {
        // TODO: Implement recipe tracking
        Vec::new()
    }

    /// Check if there are unsaved changes (stub for UI)
    pub fn has_unsaved_changes(&self) -> bool {
        // TODO: Implement change tracking
        false
    }

    /// Load production chain from JSON string
    pub fn load_from_json(
        &mut self,
        json: &str,
        game_data: Option<&GameData>,
    ) -> Result<(), String> {
        let file: ProductionChainFile =
            serde_json::from_str(json).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        self.load_from_file(file, game_data)
    }

    /// Load production chain from file structure
    pub fn load_from_file(
        &mut self,
        file: ProductionChainFile,
        game_data: Option<&GameData>,
    ) -> Result<(), String> {
        // Clear existing data
        self.nodes.clear();
        self.links.clear();
        self.next_id = 1;

        // Load nodes
        for (idx, serialized_node) in file.nodes.iter().enumerate() {
            self.load_node(serialized_node, idx, game_data)?;
        }

        // Load links
        for serialized_link in &file.links {
            self.load_link(serialized_link)?;
        }

        Ok(())
    }

    /// Load a single node from serialized format
    fn load_node(
        &mut self,
        serialized: &SerializedNode,
        _node_index: usize,
        game_data: Option<&GameData>,
    ) -> Result<(), String> {
        match serialized {
            SerializedNode::Craft(craft) => {
                let node_id = self.get_next_id();
                let mut node = CraftNode::new(node_id, craft.recipe.clone());
                node.base.position = (craft.pos.x, craft.pos.y);
                node.current_rate = craft.rate.clone().into();
                node.built = craft.built;
                node.num_somersloop = FractionalNumber::from(craft.num_somersloop as i64);

                // Create pins from recipe if game data is available
                if let Some(gd) = game_data {
                    if let Some(recipe) = gd.recipes().iter().find(|r| r.name == craft.recipe) {
                        node.building_name = recipe.building_name.clone();
                        node.recipe_power = recipe.power;
                        if let Some(building) = &recipe.building {
                            node.power_exponent = building.power_exponent;
                            node.somersloop_power_exponent = building.somersloop_power_exponent;
                            node.somersloop_mult = building.somersloop_mult.clone();
                            node.variable_power = building.variable_power;
                        }
                        // Create input pins
                        for counted_item in &recipe.ins {
                            let pin_id = self.get_next_id();
                            let pin = Pin::new(
                                pin_id,
                                PinDirection::Input,
                                node_id,
                                Some(counted_item.item_name.clone()),
                                false,
                                counted_item.quantity,
                            );
                            node.base.ins.push(pin);
                        }

                        // Create output pins
                        for counted_item in &recipe.outs {
                            let pin_id = self.get_next_id();
                            let pin = Pin::new(
                                pin_id,
                                PinDirection::Output,
                                node_id,
                                Some(counted_item.item_name.clone()),
                                false,
                                counted_item.quantity,
                            );
                            node.base.outs.push(pin);
                        }
                    }
                }

                self.nodes.push(Box::new(node));
            }
            SerializedNode::Sink(sink) => {
                let node_id = self.get_next_id();
                let mut node = SinkNode::new(node_id, None);
                node.base.position = (sink.pos.x, sink.pos.y);

                // Create input pins for each sink input
                for input in &sink.ins {
                    let pin_id = self.get_next_id();
                    let rate = FractionalNumber::new(input.num, input.den);
                    let pin = Pin::new(
                        pin_id,
                        PinDirection::Input,
                        node_id,
                        Some(input.item.clone()),
                        input.locked,
                        rate,
                    );
                    node.base.ins.push(pin);
                }

                self.nodes.push(Box::new(node));
            }
            SerializedNode::Organizer(org) => {
                let kind = NodeKind::from_kind_id(org.kind)
                    .ok_or_else(|| format!("Invalid node kind: {}", org.kind))?;
                let node_id = self.get_next_id();
                let mut node = OrganizerNode::new(node_id, kind, org.item.clone());
                node.base.position = (org.pos.x, org.pos.y);

                // Pins for organizers will be dynamically created based on connections
                self.nodes.push(Box::new(node));
            }
        }

        Ok(())
    }

    /// Load a link from serialized format
    fn load_link(&mut self, serialized: &SerializedLink) -> Result<(), String> {
        let start_node_idx = serialized.start.node;
        let end_node_idx = serialized.end.node;

        if start_node_idx >= self.nodes.len() {
            return Err(format!("Invalid start node index: {}", start_node_idx));
        }
        if end_node_idx >= self.nodes.len() {
            return Err(format!("Invalid end node index: {}", end_node_idx));
        }

        // Get pin IDs from node indices and pin indices
        let start_pin_id =
            self.get_pin_id_by_indices(start_node_idx, serialized.start.pin, PinDirection::Output)?;
        let end_pin_id =
            self.get_pin_id_by_indices(end_node_idx, serialized.end.pin, PinDirection::Input)?;

        // Create the link
        let link_id = self.get_next_id();
        let link = Link::new(link_id, start_pin_id, end_pin_id);

        // Update pin references
        self.set_pin_link_id(start_pin_id, Some(link_id))?;
        self.set_pin_link_id(end_pin_id, Some(link_id))?;

        self.links.push(link);
        Ok(())
    }

    /// Get pin ID by node index and pin index within that node
    fn get_pin_id_by_indices(
        &mut self,
        node_idx: usize,
        pin_idx: usize,
        direction: PinDirection,
    ) -> Result<u64, String> {
        // We allow Organizer nodes to create pins dynamically when restoring links

        // Craft nodes: must already have pins
        if let Some(craft) = self.nodes[node_idx].downcast_ref::<CraftNode>() {
            let pins = match direction {
                PinDirection::Input => &craft.base.ins,
                PinDirection::Output => &craft.base.outs,
            };
            return pins
                .get(pin_idx)
                .map(|p| p.id)
                .ok_or_else(|| format!("Pin index {} out of bounds", pin_idx));
        }
        // Sink nodes: must already have pins
        if let Some(sink) = self.nodes[node_idx].downcast_ref::<SinkNode>() {
            return sink
                .base
                .ins
                .get(pin_idx)
                .map(|p| p.id)
                .ok_or_else(|| format!("Pin index {} out of bounds", pin_idx));
        }
        // Organizer nodes: create missing pins up to requested index
        if self.nodes[node_idx].downcast_ref::<OrganizerNode>().is_some() {
            // Read current length without holding a mutable borrow
            let current_len = {
                let org_ref = self.nodes[node_idx].downcast_ref::<OrganizerNode>().unwrap();
                match direction {
                    PinDirection::Input => org_ref.base.ins.len(),
                    PinDirection::Output => org_ref.base.outs.len(),
                }
            };
            if pin_idx < current_len {
                let org_ref = self.nodes[node_idx].downcast_ref::<OrganizerNode>().unwrap();
                let id = match direction {
                    PinDirection::Input => org_ref.base.ins[pin_idx].id,
                    PinDirection::Output => org_ref.base.outs[pin_idx].id,
                };
                return Ok(id);
            }
            // Need to create missing pins: compute how many
            let needed = pin_idx + 1 - current_len;
            let mut new_ids = Vec::new();
            for _ in 0..needed {
                new_ids.push(self.get_next_id());
            }
            // Now mutate the organizer to push pins
            let org_mut = self.nodes[node_idx].downcast_mut::<OrganizerNode>().unwrap();
            for new_pin_id in new_ids {
                match direction {
                    PinDirection::Input => {
                        let locked = org_mut.base.outs.get(0).map(|p| p.locked).unwrap_or(false);
                        org_mut.base.ins.push(Pin::new(new_pin_id, PinDirection::Input, org_mut.base.id, org_mut.item_name.clone(), locked, FractionalNumber::default()));
                    }
                    PinDirection::Output => {
                        let locked = org_mut.base.ins.get(0).map(|p| p.locked).unwrap_or(false);
                        org_mut.base.outs.push(Pin::new(new_pin_id, PinDirection::Output, org_mut.base.id, org_mut.item_name.clone(), locked, FractionalNumber::default()));
                    }
                }
            }
            let id = match direction {
                PinDirection::Input => org_mut.base.ins[pin_idx].id,
                PinDirection::Output => org_mut.base.outs[pin_idx].id,
            };
            return Ok(id);
        }
        // Group nodes: must already have pins
        if let Some(group) = self.nodes[node_idx].downcast_ref::<GroupNode>() {
            let pins = match direction {
                PinDirection::Input => &group.base.ins,
                PinDirection::Output => &group.base.outs,
            };
            return pins
                .get(pin_idx)
                .map(|p| p.id)
                .ok_or_else(|| format!("Pin index {} out of bounds", pin_idx));
        }

        Err("Unknown node type".to_string())
    }

    /// Set link_id on a pin
    fn set_pin_link_id(&mut self, pin_id: u64, link_id: Option<u64>) -> Result<(), String> {
        for node_box in &mut self.nodes {
            if let Some(craft) = node_box.downcast_mut::<CraftNode>() {
                for pin in craft.base.ins.iter_mut().chain(craft.base.outs.iter_mut()) {
                    if pin.id == pin_id {
                        pin.link_id = link_id;
                        return Ok(());
                    }
                }
            } else if let Some(sink) = node_box.downcast_mut::<SinkNode>() {
                for pin in &mut sink.base.ins {
                    if pin.id == pin_id {
                        pin.link_id = link_id;
                        return Ok(());
                    }
                }
            } else if let Some(org) = node_box.downcast_mut::<OrganizerNode>() {
                for pin in org.base.ins.iter_mut().chain(org.base.outs.iter_mut()) {
                    if pin.id == pin_id {
                        pin.link_id = link_id;
                        return Ok(());
                    }
                }
            } else if let Some(group) = node_box.downcast_mut::<GroupNode>() {
                for pin in group.base.ins.iter_mut().chain(group.base.outs.iter_mut()) {
                    if pin.id == pin_id {
                        pin.link_id = link_id;
                        return Ok(());
                    }
                }
            }
        }
        Err(format!("Pin {} not found", pin_id))
    }

    /// Save production chain to JSON string
    pub fn save_to_json(&self) -> Result<String, String> {
        let file = self.save_to_file();
        serde_json::to_string_pretty(&file).map_err(|e| format!("Failed to serialize JSON: {}", e))
    }

    /// Save production chain to file structure
    pub fn save_to_file(&self) -> ProductionChainFile {
        let mut nodes = Vec::new();
        let mut links = Vec::new();

        // Build node index map
        let mut node_id_to_index = std::collections::HashMap::new();
        for (idx, node_box) in self.nodes.iter().enumerate() {
            let node_id = self.get_node_id(node_box);
            node_id_to_index.insert(node_id, idx);
        }

        // Serialize nodes
        for node_box in &self.nodes {
            if let Some(serialized) = self.serialize_node(node_box) {
                nodes.push(serialized);
            }
        }

        // Serialize links
        for link in &self.links {
            if let Some(serialized) = self.serialize_link(link, &node_id_to_index) {
                links.push(serialized);
            }
        }

        ProductionChainFile {
            game_version: "1.0".to_string(),
            save_version: 5,
            nodes,
            links,
        }
    }

    /// Get node ID from any node type
    fn get_node_id(&self, node_box: &Box<dyn std::any::Any>) -> u64 {
        if let Some(craft) = node_box.downcast_ref::<CraftNode>() {
            craft.base.id
        } else if let Some(sink) = node_box.downcast_ref::<SinkNode>() {
            sink.base.id
        } else if let Some(org) = node_box.downcast_ref::<OrganizerNode>() {
            org.base.id
        } else if let Some(group) = node_box.downcast_ref::<GroupNode>() {
            group.base.id
        } else {
            0 // Should never happen
        }
    }

    /// Serialize a node to the file format
    fn serialize_node(&self, node_box: &Box<dyn std::any::Any>) -> Option<SerializedNode> {
        if let Some(craft) = node_box.downcast_ref::<CraftNode>() {
            Some(SerializedNode::Craft(SerializedCraftNode {
                kind: 0,
                recipe: craft.recipe_name.clone(),
                rate: craft.current_rate.into(),
                pos: SerializedPosition {
                    x: craft.base.position.0,
                    y: craft.base.position.1,
                },
                built: craft.built,
                locked: false, // TODO: Track node-level lock
                num_somersloop: craft.num_somersloop.numerator() as u8,
            }))
        } else if let Some(sink) = node_box.downcast_ref::<SinkNode>() {
            let ins = sink
                .base
                .ins
                .iter()
                .map(|pin| SerializedSinkInput {
                    item: pin.item_name.clone().unwrap_or_default(),
                    num: pin.base_rate.numerator(),
                    den: pin.base_rate.denominator(),
                    locked: pin.locked,
                })
                .collect();

            Some(SerializedNode::Sink(SerializedSinkNode {
                kind: 5,
                pos: SerializedPosition {
                    x: sink.base.position.0,
                    y: sink.base.position.1,
                },
                ins,
            }))
        } else if let Some(org) = node_box.downcast_ref::<OrganizerNode>() {
            Some(SerializedNode::Organizer(SerializedOrganizerNode {
                kind: org.base.kind.to_kind_id(),
                pos: SerializedPosition {
                    x: org.base.position.0,
                    y: org.base.position.1,
                },
                item: org.item_name.clone(),
            }))
        } else {
            None
        }
    }

    /// Serialize a link to the file format
    fn serialize_link(
        &self,
        link: &Link,
        node_id_to_index: &std::collections::HashMap<u64, usize>,
    ) -> Option<SerializedLink> {
        // Find start and end pins
        let start_loc = self.find_pin_location(link.start_pin_id)?;
        let end_loc = self.find_pin_location(link.end_pin_id)?;

        let start_node_idx = *node_id_to_index.get(&start_loc.0)?;
        let end_node_idx = *node_id_to_index.get(&end_loc.0)?;

        Some(SerializedLink {
            start: SerializedLinkEndpoint {
                node: start_node_idx,
                pin: start_loc.2, // Use direction-specific pin index
            },
            end: SerializedLinkEndpoint {
                node: end_node_idx,
                pin: end_loc.2, // Use direction-specific pin index
            },
        })
    }
}

impl Default for ProductionApp {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_pin_rate_updates_craft_node_output() {
        let mut app = ProductionApp::new();
        // Create a craft node without relying on game data
        let node_id = app.get_next_id();
        let mut craft = CraftNode::new(node_id, "test_recipe".to_string());
        // Create one output pin with base rate 2
        let pin_id = app.get_next_id();
        craft.base.outs.push(Pin::new(
            pin_id,
            PinDirection::Output,
            node_id,
            None,
            false,
            FractionalNumber::new(2, 1),
        ));
        app.nodes.push(Box::new(craft));

        let node_idx = app.find_node_index(node_id).expect("find node");

        // Now set output pin rate to 6 -> node rate should become 3
        app.set_pin_rate(
            node_id,
            PinDirection::Output,
            0,
            FractionalNumber::new(6, 1),
        )
        .expect("set_pin_rate");

        // Re-borrow immutably to assert
        let n = app.nodes[node_idx]
            .downcast_ref::<CraftNode>()
            .expect("expected craft node");
        assert_eq!(n.current_rate, FractionalNumber::new(3, 1));
        assert_eq!(n.base.outs[0].current_rate, FractionalNumber::new(6, 1));
    }
}
