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

    /// Get the node kind for an organizer node (Merger, CustomSplitter, GameSplitter)
    /// Returns None for non-organizer nodes (CraftNode, GroupNode, SinkNode)
    pub fn get_node_kind(&self, node_id: u64) -> Option<crate::node::NodeKind> {
        let idx = self.find_node_index(node_id)?;
        let node_any = &self.nodes[idx];
        if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
            Some(n.base.kind)
        } else {
            None
        }
    }

    /// Get the label/title for a node
    /// - CraftNode: recipe_name
    /// - OrganizerNode: kind-specific default (e.g., "Merger", "Splitter*", "Splitter")
    /// - GroupNode: "Group"
    /// - SinkNode: "Sink"
    pub fn get_node_label(&self, node_id: u64) -> Option<String> {
        let idx = self.find_node_index(node_id)?;
        let node_any = &self.nodes[idx];
        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
            Some(n.recipe_name.clone())
        } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
            let label = match n.base.kind {
                crate::node::NodeKind::Merger => "Merger",
                crate::node::NodeKind::CustomSplitter => "Splitter*",
                crate::node::NodeKind::GameSplitter => "Splitter",
                _ => "Organizer",
            };
            Some(label.to_string())
        } else if let Some(_n) = node_any.downcast_ref::<GroupNode>() {
            Some("Group".to_string())
        } else if let Some(_n) = node_any.downcast_ref::<SinkNode>() {
            Some("Sink".to_string())
        } else {
            None
        }
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

    /// Get pin rates for a node (inputs, outputs) as fraction strings (e.g., "9/16")
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
                .map(|p| Some(p.current_rate.to_fraction_string()))
                .collect();
            let outs = n
                .base
                .outs
                .iter()
                .map(|p| Some(p.current_rate.to_fraction_string()))
                .collect();
            return Some((ins, outs));
        } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
            let ins = n
                .base
                .ins
                .iter()
                .map(|p| Some(p.current_rate.to_fraction_string()))
                .collect();
            let outs = n
                .base
                .outs
                .iter()
                .map(|p| Some(p.current_rate.to_fraction_string()))
                .collect();
            return Some((ins, outs));
        } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
            let ins = n
                .base
                .ins
                .iter()
                .map(|p| Some(p.current_rate.to_fraction_string()))
                .collect();
            let outs = n
                .base
                .outs
                .iter()
                .map(|p| Some(p.current_rate.to_fraction_string()))
                .collect();
            return Some((ins, outs));
        } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
            let ins = n
                .base
                .ins
                .iter()
                .map(|p| Some(p.current_rate.to_fraction_string()))
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

    /// Map node/direction/index to the internal pin id (if present)
    pub fn get_pin_id(&self, node_id: u64, direction: PinDirection, idx: usize) -> Option<u64> {
        let node_idx = self.find_node_index(node_id)?;
        let node_any = &self.nodes[node_idx];
        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
            match direction {
                PinDirection::Input => n.base.ins.get(idx).map(|p| p.id),
                PinDirection::Output => n.base.outs.get(idx).map(|p| p.id),
            }
        } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
            match direction {
                PinDirection::Input => n.base.ins.get(idx).map(|p| p.id),
                PinDirection::Output => n.base.outs.get(idx).map(|p| p.id),
            }
        } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
            match direction {
                PinDirection::Input => n.base.ins.get(idx).map(|p| p.id),
                PinDirection::Output => n.base.outs.get(idx).map(|p| p.id),
            }
        } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
            if direction == PinDirection::Input {
                n.base.ins.get(idx).map(|p| p.id)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_node_building_info(&self, node_id: u64) -> Option<(String, String)> {
        let idx = self.find_node_index(node_id)?;
        let node_any = &self.nodes[idx];
        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
            // building_count_str is the current_rate formatted as a fraction string to preserve precision
            let count_str = n.current_rate.to_fraction_string();
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
                same.to_fraction_string(),
                last.to_fraction_string(),
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

    /// Check if a craft node is a power generator (negative recipe power)
    pub fn get_node_is_power_generator(&self, node_id: u64) -> bool {
        let idx = match self.find_node_index(node_id) {
            Some(i) => i,
            None => return false,
        };
        let node_any = &self.nodes[idx];
        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
            return n.recipe_power < 0.0;
        }
        false
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

    /// Set organizer node item name and propagate to its pins
    pub fn set_node_item_name(&mut self, node_id: u64, item: Option<String>) -> Result<(), String> {
        let idx = self
            .find_node_index(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;
        let node_any = &mut self.nodes[idx];
        if let Some(n) = node_any.downcast_mut::<OrganizerNode>() {
            n.item_name = item.clone();
            for p in n.base.ins.iter_mut().chain(n.base.outs.iter_mut()) {
                p.item_name = item.clone();
            }
            return Ok(());
        }
        Err("Node is not an organizer".into())
    }

    /// Get organizer node item name
    pub fn get_node_item_name(&self, node_id: u64) -> Option<String> {
        let idx = self.find_node_index(node_id)?;
        let node_any = &self.nodes[idx];
        if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
            return n.item_name.clone();
        }
        None
    }

    /// Set the current number of buildings for a craft node (node rate). This triggers propagation to connected graph.
    pub fn set_node_building_count(
        &mut self,
        node_id: u64,
        new_count: crate::fractional_number::FractionalNumber,
    ) -> Result<(), String> {
        // Validate non-negative
        if new_count.numerator() < 0 {
            return Err("Invalid count".into());
        }
        let idx = self
            .find_node_index(node_id)
            .ok_or_else(|| format!("Node {} not found", node_id))?;
        let node_any = &mut self.nodes[idx];
        if let Some(n) = node_any.downcast_mut::<CraftNode>() {
            // Apply new node rate
            let pin_id = {
                n.update_rate(new_count);
                // prefer an output pin when available
                if !n.base.outs.is_empty() {
                    n.base.outs[0].id
                } else if !n.base.ins.is_empty() {
                    n.base.ins[0].id
                } else {
                    // no pins to propagate from - nothing to do
                    return Ok(());
                }
            };
            // Re-query the updated pin rate after borrow drop
            let cur = if let Some(n2) = self.nodes[idx].downcast_ref::<CraftNode>() {
                if !n2.base.outs.is_empty() {
                    n2.base.outs[0].current_rate
                } else if !n2.base.ins.is_empty() {
                    n2.base.ins[0].current_rate
                } else {
                    crate::fractional_number::FractionalNumber::default()
                }
            } else {
                crate::fractional_number::FractionalNumber::default()
            };
            // Propagate through graph using the selected pin as constraint
            self.update_nodes_rate(pin_id, cur)
                .map_err(|e| format!("Failed to propagate rates: {}", e))?;
            return Ok(());
        }
        Err("Unsupported node kind for building count edit".into())
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

        // CraftNode: if setting an output's rate, derive the node rate
        if let Some(n) = self.nodes[node_idx].downcast_mut::<CraftNode>() {
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
                    // Update node then propagate after dropping the borrow
                    let pin_id = {
                        n.update_rate(new_node_rate);
                        n.base.outs[pin_index].id
                    };
                    self.update_nodes_rate(pin_id, {
                        // re-query current_rate after borrow dropped
                        let cur = if let Some(n2) = self.nodes[node_idx].downcast_ref::<CraftNode>()
                        {
                            n2.base.outs[pin_index].current_rate
                        } else {
                            FractionalNumber::default()
                        };
                        cur
                    })
                    .map_err(|e| format!("Failed to propagate rates: {}", e))?;
                    return Ok(());
                }
                PinDirection::Input => {
                    if pin_index >= n.base.ins.len() {
                        return Err("Input pin out of range".into());
                    }
                    let pin_id = {
                        n.base.ins[pin_index].current_rate = new_rate;
                        n.base.ins[pin_index].id
                    };
                    self.update_nodes_rate(pin_id, {
                        let cur = if let Some(n2) = self.nodes[node_idx].downcast_ref::<CraftNode>()
                        {
                            n2.base.ins[pin_index].current_rate
                        } else {
                            FractionalNumber::default()
                        };
                        cur
                    })
                    .map_err(|e| format!("Failed to propagate rates: {}", e))?;
                    return Ok(());
                }
            }
        }

        // Organizer nodes: set directly
        if let Some(n) = self.nodes[node_idx].downcast_mut::<OrganizerNode>() {
            match direction {
                PinDirection::Input => {
                    if pin_index >= n.base.ins.len() {
                        return Err("Input pin out of range".into());
                    }
                    let pin_id = {
                        n.base.ins[pin_index].current_rate = new_rate;
                        n.base.ins[pin_index].id
                    };
                    // drop mutable borrow
                    drop(n);
                    let cur = if let Some(n2) = self.nodes[self.find_node_index(node_id).unwrap()]
                        .downcast_ref::<OrganizerNode>()
                    {
                        n2.base.ins[pin_index].current_rate
                    } else {
                        FractionalNumber::default()
                    };
                    self.update_nodes_rate(pin_id, cur)
                        .map_err(|e| format!("Failed to propagate rates: {}", e))?;
                    return Ok(());
                }
                PinDirection::Output => {
                    if pin_index >= n.base.outs.len() {
                        return Err("Output pin out of range".into());
                    }
                    let pin_id = {
                        n.base.outs[pin_index].current_rate = new_rate;
                        n.base.outs[pin_index].id
                    };
                    // drop mutable borrow
                    drop(n);
                    let cur = if let Some(n2) = self.nodes[self.find_node_index(node_id).unwrap()]
                        .downcast_ref::<OrganizerNode>()
                    {
                        n2.base.outs[pin_index].current_rate
                    } else {
                        FractionalNumber::default()
                    };
                    self.update_nodes_rate(pin_id, cur)
                        .map_err(|e| format!("Failed to propagate rates: {}", e))?;
                    return Ok(());
                }
            }
        }

        // Group/Sink: set directly
        if let Some(n) = self.nodes[node_idx].downcast_mut::<GroupNode>() {
            match direction {
                PinDirection::Input => {
                    if pin_index >= n.base.ins.len() {
                        return Err("Input pin out of range".into());
                    }
                    let pin_id = {
                        n.base.ins[pin_index].current_rate = new_rate;
                        n.base.ins[pin_index].id
                    };
                    // drop mutable borrow
                    drop(n);
                    let cur = if let Some(n2) = self.nodes[self.find_node_index(node_id).unwrap()]
                        .downcast_ref::<GroupNode>()
                    {
                        n2.base.ins[pin_index].current_rate
                    } else {
                        FractionalNumber::default()
                    };
                    self.update_nodes_rate(pin_id, cur)
                        .map_err(|e| format!("Failed to propagate rates: {}", e))?;
                    return Ok(());
                }
                PinDirection::Output => {
                    if pin_index >= n.base.outs.len() {
                        return Err("Output pin out of range".into());
                    }
                    let pin_id = {
                        n.base.outs[pin_index].current_rate = new_rate;
                        n.base.outs[pin_index].id
                    };
                    // drop mutable borrow
                    drop(n);
                    let cur = if let Some(n2) = self.nodes[self.find_node_index(node_id).unwrap()]
                        .downcast_ref::<GroupNode>()
                    {
                        n2.base.outs[pin_index].current_rate
                    } else {
                        FractionalNumber::default()
                    };
                    self.update_nodes_rate(pin_id, cur)
                        .map_err(|e| format!("Failed to propagate rates: {}", e))?;
                    return Ok(());
                }
            }
        }

        if let Some(n) = self.nodes[node_idx].downcast_mut::<SinkNode>() {
            if direction != PinDirection::Input {
                return Err("Sink has no outputs".into());
            }
            if pin_index >= n.base.ins.len() {
                return Err("Input pin out of range".into());
            }
            let pin_id = {
                n.base.ins[pin_index].current_rate = new_rate;
                n.base.ins[pin_index].id
            };
            // drop mutable borrow
            drop(n);
            let cur = if let Some(n2) =
                self.nodes[self.find_node_index(node_id).unwrap()].downcast_ref::<SinkNode>()
            {
                n2.base.ins[pin_index].current_rate
            } else {
                FractionalNumber::default()
            };
            self.update_nodes_rate(pin_id, cur)
                .map_err(|e| format!("Failed to propagate rates: {}", e))?;
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

            // If recipe doesn't have explicit power, use building power (for power generators)
            if craft_node.recipe_power == 0.0 {
                craft_node.recipe_power = building.power;
            }
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

        // For GameSplitter, we use one variable per node (like craft nodes).
        // The rate propagation treats them as: pin_rate = var * base_rate
        // For equal distribution with 2 outputs:
        //   - Input base_rate = 1 (full flow)
        //   - Each output base_rate = 1/num_outputs = 1/2 (equal split)
        // Sum constraint: input = output1 + output2 → var * 1 = var * 0.5 + var * 0.5 ✓
        let num_outputs = 2;

        let in_pin_id = self.get_next_id();
        splitter.base.ins.push(Pin::new(
            in_pin_id,
            PinDirection::Input,
            node_id,
            None,
            false,
            FractionalNumber::new(1, 1),
        ));

        for _ in 0..num_outputs {
            let pin_id = self.get_next_id();
            splitter.base.outs.push(Pin::new(
                pin_id,
                PinDirection::Output,
                node_id,
                None,
                false,
                FractionalNumber::new(1, num_outputs as i64),
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
    /// Returns (link_id, optional_propagation_warning)
    pub fn create_link(
        &mut self,
        start_pin_id: u64,
        end_pin_id: u64,
    ) -> Result<(u64, Option<String>), String> {
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

        // Attempt guarded propagation on link creation: snapshot pin rates and lock states,
        // try to propagate, and if propagation fails restore the snapshot but keep the link.
        //
        // IMPORTANT: We need to find a locked pin in the connected graph to use as the constraint
        // source. If we just use start_pin's rate (which may be 0), we'll get contradictions
        // when connecting to a graph that already has locked pins with non-zero rates.

        // Helper to get a pin's (rate, locked) status
        let get_pin_info = |this: &Self, pin_id: u64| -> Option<(FractionalNumber, bool)> {
            if let Some((node_id, direction, pi)) = this.find_pin_location(pin_id) {
                let ni = this.find_node_index(node_id)?;
                let node_any = &this.nodes[ni];
                if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                    let p = match direction {
                        PinDirection::Input => &n.base.ins[pi],
                        PinDirection::Output => &n.base.outs[pi],
                    };
                    return Some((p.current_rate, p.locked));
                } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                    let p = match direction {
                        PinDirection::Input => &n.base.ins[pi],
                        PinDirection::Output => &n.base.outs[pi],
                    };
                    return Some((p.current_rate, p.locked));
                } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
                    let p = match direction {
                        PinDirection::Input => &n.base.ins[pi],
                        PinDirection::Output => &n.base.outs[pi],
                    };
                    return Some((p.current_rate, p.locked));
                } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
                    let p = &n.base.ins[pi];
                    return Some((p.current_rate, p.locked));
                }
            }
            None
        };

        // Find a locked pin in the connected graph starting from both endpoints
        // The connected graph now includes both sides since we just created the link
        let connected_pins = self.get_connected_pins(start_pin_id);

        // Prioritize LOCKED pins as constraint source - their rates are fixed.
        // If either endpoint is locked, use its rate as the constraint.
        // This is critical when connecting to a locked graph.
        let mut constraint_pin_id = start_pin_id;
        let mut constraint_value = FractionalNumber::default();

        // First priority: If end_pin is locked, use its rate
        if let Some((rate, locked)) = get_pin_info(self, end_pin_id) {
            if locked && !rate.is_zero() {
                constraint_pin_id = end_pin_id;
                constraint_value = rate;
            }
        }

        // Second priority: If start_pin is locked (and we didn't already pick end_pin)
        if constraint_value.is_zero() {
            if let Some((rate, locked)) = get_pin_info(self, start_pin_id) {
                if locked && !rate.is_zero() {
                    constraint_pin_id = start_pin_id;
                    constraint_value = rate;
                }
            }
        }

        // Third priority: search for any locked pin in connected graph
        if constraint_value.is_zero() {
            for pid in &connected_pins {
                if let Some((rate, locked)) = get_pin_info(self, *pid) {
                    if locked && !rate.is_zero() {
                        constraint_pin_id = *pid;
                        constraint_value = rate;
                        break;
                    }
                }
            }
        }

        // Fourth priority: use start_pin or end_pin with non-zero rate (even if not locked)
        if constraint_value.is_zero() {
            if let Some((rate, _)) = get_pin_info(self, start_pin_id) {
                if !rate.is_zero() {
                    constraint_pin_id = start_pin_id;
                    constraint_value = rate;
                }
            }
        }
        if constraint_value.is_zero() {
            if let Some((rate, _)) = get_pin_info(self, end_pin_id) {
                if !rate.is_zero() {
                    constraint_pin_id = end_pin_id;
                    constraint_value = rate;
                }
            }
        }

        // Snapshot all pin rates and locks so we can restore if propagation fails
        let mut snapshot: std::collections::HashMap<u64, (FractionalNumber, bool)> =
            std::collections::HashMap::new();
        for node_any in &self.nodes {
            if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                    snapshot.insert(p.id, (p.current_rate, p.locked));
                }
            } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                    snapshot.insert(p.id, (p.current_rate, p.locked));
                }
            } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
                for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                    snapshot.insert(p.id, (p.current_rate, p.locked));
                }
            } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
                for p in n.base.ins.iter() {
                    snapshot.insert(p.id, (p.current_rate, p.locked));
                }
            }
        }

        let mut propagate_warning: Option<String> = None;
        if let Err(e) = self.update_nodes_rate(constraint_pin_id, constraint_value) {
            // Restore snapshot (rates and locks) to avoid partial propagation state
            for (pid, (rate, locked)) in snapshot.into_iter() {
                if let Some((node_id, direction, idx)) = self.find_pin_location(pid) {
                    let ni = self.find_node_index(node_id).unwrap();
                    if let Some(n) = self.nodes[ni].downcast_mut::<CraftNode>() {
                        match direction {
                            PinDirection::Input => {
                                n.base.ins[idx].current_rate = rate;
                                n.base.ins[idx].locked = locked;
                            }
                            PinDirection::Output => {
                                n.base.outs[idx].current_rate = rate;
                                n.base.outs[idx].locked = locked;
                            }
                        }
                    } else if let Some(n) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
                        match direction {
                            PinDirection::Input => {
                                n.base.ins[idx].current_rate = rate;
                                n.base.ins[idx].locked = locked;
                            }
                            PinDirection::Output => {
                                n.base.outs[idx].current_rate = rate;
                                n.base.outs[idx].locked = locked;
                            }
                        }
                    } else if let Some(n) = self.nodes[ni].downcast_mut::<GroupNode>() {
                        match direction {
                            PinDirection::Input => {
                                n.base.ins[idx].current_rate = rate;
                                n.base.ins[idx].locked = locked;
                            }
                            PinDirection::Output => {
                                n.base.outs[idx].current_rate = rate;
                                n.base.outs[idx].locked = locked;
                            }
                        }
                    } else if let Some(n) = self.nodes[ni].downcast_mut::<SinkNode>() {
                        n.base.ins[idx].current_rate = rate;
                        n.base.ins[idx].locked = locked;
                    }
                }
            }
            // Keep the link but record the propagation failure so caller can surface it
            propagate_warning = Some(format!("Propagation failed: {}", e));
        }

        // Set lock state
        if let Some((_n, _d, _pi)) = self.find_pin_location(start_pin_id) {
            if let Some((node_id, direction, pi)) = self.find_pin_location(start_pin_id) {
                let ni = self.find_node_index(node_id).unwrap();
                let start_locked = if let Some(n) = self.nodes[ni].downcast_ref::<CraftNode>() {
                    match direction {
                        PinDirection::Input => n.base.ins[pi].locked,
                        PinDirection::Output => n.base.outs[pi].locked,
                    }
                } else if let Some(n) = self.nodes[ni].downcast_ref::<OrganizerNode>() {
                    match direction {
                        PinDirection::Input => n.base.ins[pi].locked,
                        PinDirection::Output => n.base.outs[pi].locked,
                    }
                } else if let Some(n) = self.nodes[ni].downcast_ref::<GroupNode>() {
                    match direction {
                        PinDirection::Input => n.base.ins[pi].locked,
                        PinDirection::Output => n.base.outs[pi].locked,
                    }
                } else if let Some(n) = self.nodes[ni].downcast_ref::<SinkNode>() {
                    n.base.ins[pi].locked
                } else {
                    false
                };

                if let Some((node_id, direction, pi)) = self.find_pin_location(end_pin_id) {
                    let ni = self.find_node_index(node_id).unwrap();
                    let end_locked = if let Some(n) = self.nodes[ni].downcast_ref::<CraftNode>() {
                        match direction {
                            PinDirection::Input => n.base.ins[pi].locked,
                            PinDirection::Output => n.base.outs[pi].locked,
                        }
                    } else if let Some(n) = self.nodes[ni].downcast_ref::<OrganizerNode>() {
                        match direction {
                            PinDirection::Input => n.base.ins[pi].locked,
                            PinDirection::Output => n.base.outs[pi].locked,
                        }
                    } else if let Some(n) = self.nodes[ni].downcast_ref::<GroupNode>() {
                        match direction {
                            PinDirection::Input => n.base.ins[pi].locked,
                            PinDirection::Output => n.base.outs[pi].locked,
                        }
                    } else if let Some(n) = self.nodes[ni].downcast_ref::<SinkNode>() {
                        n.base.ins[pi].locked
                    } else {
                        false
                    };

                    if start_locked || end_locked {
                        if let Some((node_id, direction, pi)) = self.find_pin_location(start_pin_id)
                        {
                            let ni = self.find_node_index(node_id).unwrap();
                            if let Some(n) = self.nodes[ni].downcast_mut::<CraftNode>() {
                                match direction {
                                    PinDirection::Input => n.base.ins[pi].locked = true,
                                    PinDirection::Output => n.base.outs[pi].locked = true,
                                }
                            } else if let Some(n) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
                                match direction {
                                    PinDirection::Input => n.base.ins[pi].locked = true,
                                    PinDirection::Output => n.base.outs[pi].locked = true,
                                }
                            } else if let Some(n) = self.nodes[ni].downcast_mut::<GroupNode>() {
                                match direction {
                                    PinDirection::Input => n.base.ins[pi].locked = true,
                                    PinDirection::Output => n.base.outs[pi].locked = true,
                                }
                            } else if let Some(n) = self.nodes[ni].downcast_mut::<SinkNode>() {
                                n.base.ins[pi].locked = true;
                            }
                        }

                        if let Some((node_id, direction, pi)) = self.find_pin_location(end_pin_id) {
                            let ni = self.find_node_index(node_id).unwrap();
                            if let Some(n) = self.nodes[ni].downcast_mut::<CraftNode>() {
                                match direction {
                                    PinDirection::Input => n.base.ins[pi].locked = true,
                                    PinDirection::Output => n.base.outs[pi].locked = true,
                                }
                            } else if let Some(n) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
                                match direction {
                                    PinDirection::Input => n.base.ins[pi].locked = true,
                                    PinDirection::Output => n.base.outs[pi].locked = true,
                                }
                            } else if let Some(n) = self.nodes[ni].downcast_mut::<GroupNode>() {
                                match direction {
                                    PinDirection::Input => n.base.ins[pi].locked = true,
                                    PinDirection::Output => n.base.outs[pi].locked = true,
                                }
                            } else if let Some(n) = self.nodes[ni].downcast_mut::<SinkNode>() {
                                n.base.ins[pi].locked = true;
                            }
                        }
                    }
                }
            }
        }

        // Set items for organizer nodes (compute candidate items first to avoid overlapping borrows)
        if let Some((org_node_id, _dir, _)) = self.find_pin_location(start_pin_id) {
            // Candidate item from the other endpoint
            if let Some((other_node_id, _other_dir, pi)) = self.find_pin_location(end_pin_id) {
                let candidate_item = if let Some(end_n) = self.nodes
                    [self.find_node_index(other_node_id).unwrap()]
                .downcast_ref::<CraftNode>()
                {
                    end_n.base.outs.get(pi).and_then(|p| p.item_name.clone())
                } else if let Some(end_n) = self.nodes[self.find_node_index(other_node_id).unwrap()]
                    .downcast_ref::<OrganizerNode>()
                {
                    end_n.base.outs.get(pi).and_then(|p| p.item_name.clone())
                } else {
                    None
                };

                if let Some(item) = candidate_item {
                    let org_idx = self.find_node_index(org_node_id).unwrap();
                    if let Some(n) = self.nodes[org_idx].downcast_mut::<OrganizerNode>() {
                        if n.item_name.is_none() {
                            n.item_name = Some(item);
                        }
                    }
                }
            }
        }

        if let Some((org_node_id, _dir, _)) = self.find_pin_location(end_pin_id) {
            if let Some((other_node_id, _other_dir, pi)) = self.find_pin_location(start_pin_id) {
                let candidate_item = if let Some(start_n) = self.nodes
                    [self.find_node_index(other_node_id).unwrap()]
                .downcast_ref::<CraftNode>()
                {
                    start_n.base.outs.get(pi).and_then(|p| p.item_name.clone())
                } else if let Some(start_n) = self.nodes
                    [self.find_node_index(other_node_id).unwrap()]
                .downcast_ref::<OrganizerNode>()
                {
                    start_n.base.outs.get(pi).and_then(|p| p.item_name.clone())
                } else {
                    None
                };

                if let Some(item) = candidate_item {
                    let org_idx = self.find_node_index(org_node_id).unwrap();
                    if let Some(n) = self.nodes[org_idx].downcast_mut::<OrganizerNode>() {
                        if n.item_name.is_none() {
                            n.item_name = Some(item);
                        }
                    }
                }
            }
        }

        if let Some((node_id, _direction, _)) = self.find_pin_location(end_pin_id) {
            if self.nodes[self.find_node_index(node_id).unwrap()]
                .downcast_ref::<SinkNode>()
                .is_some()
            {
                if let Some((other_node_id, _other_dir, pi)) = self.find_pin_location(start_pin_id)
                {
                    if let Some(start_n) = self.nodes[self.find_node_index(other_node_id).unwrap()]
                        .downcast_ref::<CraftNode>()
                    {
                        if let Some(item) =
                            start_n.base.outs.get(pi).and_then(|p| p.item_name.clone())
                        {
                            let sink_idx = self.find_node_index(node_id).unwrap();
                            if let Some(n) = self.nodes[sink_idx].downcast_mut::<SinkNode>() {
                                n.item_name = Some(item);
                            }
                        }
                    }
                }
            }
        }

        // Auto-lock organizer pins when solution becomes unique
        self.auto_lock_organizer_pins();

        Ok((link_id, propagate_warning))
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

    /// Set pin locked state by locking/unlocking the entire connected component.
    /// This ensures the set of locked pins equals the set returned by `get_connected_pins`.
    pub fn set_pin_locked(&mut self, pin_id: u64, locked: bool) -> Result<(), String> {
        // Validate pin exists
        self.find_pin_location(pin_id).ok_or("Pin not found")?;

        // Compute the connected component for this pin
        let connected = self.get_connected_pins(pin_id);
        let connected_set: std::collections::HashSet<u64> = connected.into_iter().collect();

        // Apply the lock state to every pin in the connected set
        for node_any in &mut self.nodes {
            if let Some(n) = node_any.downcast_mut::<CraftNode>() {
                for p in n.base.ins.iter_mut().chain(n.base.outs.iter_mut()) {
                    if connected_set.contains(&p.id) {
                        p.locked = locked;
                    }
                }
            } else if let Some(n) = node_any.downcast_mut::<OrganizerNode>() {
                for p in n.base.ins.iter_mut().chain(n.base.outs.iter_mut()) {
                    if connected_set.contains(&p.id) {
                        p.locked = locked;
                    }
                }
            } else if let Some(n) = node_any.downcast_mut::<GroupNode>() {
                for p in n.base.ins.iter_mut().chain(n.base.outs.iter_mut()) {
                    if connected_set.contains(&p.id) {
                        p.locked = locked;
                    }
                }
            } else if let Some(n) = node_any.downcast_mut::<SinkNode>() {
                for p in n.base.ins.iter_mut() {
                    if connected_set.contains(&p.id) {
                        p.locked = locked;
                    }
                }
            }
        }

        // After locking, check for auto-locking on Merger/CustomSplitter nodes
        if locked {
            self.auto_lock_organizer_pins();
        }

        Ok(())
    }

    /// Auto-lock pins on Merger/CustomSplitter nodes when the solution becomes unique.
    /// For Merger: if output is locked and only one input is unlocked, lock that input.
    /// For CustomSplitter: if input is locked and only one output is unlocked, lock that output.
    /// Also: if all multi-pins are locked, lock the single pin.
    fn auto_lock_organizer_pins(&mut self) {
        // Collect pins that need to be locked along with their computed rates
        let mut pins_to_lock: Vec<(u64, FractionalNumber)> = Vec::new();

        for node_any in &self.nodes {
            if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                let is_merger = n.base.kind == NodeKind::Merger;
                let is_custom_splitter = n.base.kind == NodeKind::CustomSplitter;

                if !is_merger && !is_custom_splitter {
                    continue;
                }

                // For Merger: single_pin = output, multi_pins = inputs
                // For CustomSplitter: single_pin = input, multi_pins = outputs
                let single_pin = if is_custom_splitter {
                    n.base.ins.first()
                } else {
                    n.base.outs.first()
                };
                let multi_pins: Vec<&Pin> = if is_custom_splitter {
                    n.base.outs.iter().collect()
                } else {
                    n.base.ins.iter().collect()
                };

                let single_pin = match single_pin {
                    Some(p) => p,
                    None => continue,
                };

                let single_locked = single_pin.locked;
                let unlocked_multi: Vec<&Pin> =
                    multi_pins.iter().filter(|p| !p.locked).copied().collect();
                let locked_multi: Vec<&Pin> =
                    multi_pins.iter().filter(|p| p.locked).copied().collect();

                // Case 1: Single pin is locked and only one multi-pin is unlocked -> compute rate and lock it
                // For Merger: unlocked_input_rate = output - sum(locked_inputs)
                // For CustomSplitter: unlocked_output_rate = input - sum(locked_outputs)
                if single_locked && unlocked_multi.len() == 1 {
                    let single_rate = single_pin.current_rate;
                    let sum_locked: FractionalNumber = locked_multi
                        .iter()
                        .map(|p| p.current_rate)
                        .fold(FractionalNumber::new(0, 1), |acc, r| acc + r);
                    let computed_rate = single_rate - sum_locked;
                    // Only lock if rate is non-negative
                    if computed_rate.numerator() >= 0 {
                        pins_to_lock.push((unlocked_multi[0].id, computed_rate));
                    }
                }

                // Case 2: All multi-pins are locked -> compute rate and lock single pin
                // For Merger: output = sum(inputs)
                // For CustomSplitter: input = sum(outputs)
                if unlocked_multi.is_empty()
                    && !single_locked
                    && locked_multi.len() == multi_pins.len()
                {
                    let sum_multi: FractionalNumber = locked_multi
                        .iter()
                        .map(|p| p.current_rate)
                        .fold(FractionalNumber::new(0, 1), |acc, r| acc + r);
                    pins_to_lock.push((single_pin.id, sum_multi));
                }
            }
        }

        // Early return if nothing to lock
        if pins_to_lock.is_empty() {
            return;
        }

        // Now apply the locks and set rates
        for (pin_id, rate) in &pins_to_lock {
            if let Some((node_id, direction, pi)) = self.find_pin_location(*pin_id) {
                let ni = match self.find_node_index(node_id) {
                    Some(i) => i,
                    None => continue,
                };
                if let Some(n) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
                    match direction {
                        PinDirection::Input => {
                            n.base.ins[pi].current_rate = *rate;
                            n.base.ins[pi].locked = true;
                        }
                        PinDirection::Output => {
                            n.base.outs[pi].current_rate = *rate;
                            n.base.outs[pi].locked = true;
                        }
                    }
                }
            }
        }

        // Recurse since we locked some pins (they might trigger more auto-locks)
        self.auto_lock_organizer_pins();
    }

    /// Return the union of connected pins for all pins on a node.
    /// Useful for node-level locking that should affect *all* graph components touching the node.
    pub fn get_all_connected_pins_for_node(&self, node_id: u64) -> Vec<u64> {
        let mut set: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let node_idx = match self.find_node_index(node_id) {
            Some(i) => i,
            None => return Vec::new(),
        };
        let node_any = &self.nodes[node_idx];
        let mut pin_ids: Vec<u64> = Vec::new();
        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
            for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                pin_ids.push(p.id);
            }
        } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
            for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                pin_ids.push(p.id);
            }
        } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
            for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                pin_ids.push(p.id);
            }
        } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
            for p in n.base.ins.iter() {
                pin_ids.push(p.id);
            }
        }

        for pid in pin_ids {
            for p in self.get_connected_pins(pid) {
                set.insert(p);
            }
        }

        set.into_iter().collect()
    }

    /// Set node-level lock by locking/unlocking every connected component that touches the node.
    pub fn set_node_locked(&mut self, node_id: u64, locked: bool) -> Result<(), String> {
        // Validate node exists
        self.find_node_index(node_id).ok_or("Node not found")?;

        let all_pins = self.get_all_connected_pins_for_node(node_id);
        let all_set: std::collections::HashSet<u64> = all_pins.into_iter().collect();

        // Apply lock flag to all pins in the union
        for node_any in &mut self.nodes {
            if let Some(n) = node_any.downcast_mut::<CraftNode>() {
                for p in n.base.ins.iter_mut().chain(n.base.outs.iter_mut()) {
                    if all_set.contains(&p.id) {
                        p.locked = locked;
                    }
                }
            } else if let Some(n) = node_any.downcast_mut::<OrganizerNode>() {
                for p in n.base.ins.iter_mut().chain(n.base.outs.iter_mut()) {
                    if all_set.contains(&p.id) {
                        p.locked = locked;
                    }
                }
            } else if let Some(n) = node_any.downcast_mut::<GroupNode>() {
                for p in n.base.ins.iter_mut().chain(n.base.outs.iter_mut()) {
                    if all_set.contains(&p.id) {
                        p.locked = locked;
                    }
                }
            } else if let Some(n) = node_any.downcast_mut::<SinkNode>() {
                for p in n.base.ins.iter_mut() {
                    if all_set.contains(&p.id) {
                        p.locked = locked;
                    }
                }
            }
        }

        Ok(())
    }

    /// Set node-level lock and return the set of affected node ids (for UI sync)
    pub fn set_node_locked_and_get_affected(
        &mut self,
        node_id: u64,
        locked: bool,
    ) -> Result<Vec<u64>, String> {
        // Apply the locks to pins (this will validate node existence)
        self.set_node_locked(node_id, locked)?;

        // Collect affected nodes from all connected pins on the node
        let mut affected: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let connected_pins = self.get_all_connected_pins_for_node(node_id);
        for pid in connected_pins {
            if let Some((nid, _d, _i)) = self.find_pin_location(pid) {
                affected.insert(nid);
            }
        }
        // If no affected nodes found, include the original node
        if affected.is_empty() {
            affected.insert(node_id);
        }
        Ok(affected.into_iter().collect())
    }

    /// Return all pins connected to a given pin (via links and same-node pins).
    /// Avoid cycles by tracking visited nodes.
    pub fn get_connected_pins(&self, start_pin_id: u64) -> Vec<u64> {
        use std::collections::{HashSet, VecDeque};

        let mut queue: VecDeque<u64> = VecDeque::new();
        let mut visited_pins: HashSet<u64> = HashSet::new();
        let mut visited_nodes: HashSet<u64> = HashSet::new();

        queue.push_back(start_pin_id);
        visited_pins.insert(start_pin_id);
        log::trace!("[GCP] start with pin {}", start_pin_id);

        while let Some(pid) = queue.pop_front() {
            log::trace!("[GCP] processing pin {}", pid);
            // Follow links to other pins
            for link in &self.links {
                if link.start_pin_id == pid {
                    if visited_pins.insert(link.end_pin_id) {
                        log::trace!(
                            "[GCP] link {} -> {}, adding {}",
                            pid,
                            link.end_pin_id,
                            link.end_pin_id
                        );
                        queue.push_back(link.end_pin_id);
                    }
                } else if link.end_pin_id == pid {
                    if visited_pins.insert(link.start_pin_id) {
                        log::trace!(
                            "[GCP] link {} <- {}, adding {}",
                            pid,
                            link.start_pin_id,
                            link.start_pin_id
                        );
                        queue.push_back(link.start_pin_id);
                    }
                }
            }

            // Add all pins of the same node (only once per node to avoid cycles)
            if let Some((node_id, _dir, _idx)) = self.find_pin_location(pid) {
                if visited_nodes.insert(node_id) {
                    log::trace!("[GCP] visiting node {} for pin {}", node_id, pid);
                    if let Some(node_idx) = self.find_node_index(node_id) {
                        let node_any = &self.nodes[node_idx];
                        if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                            log::trace!("[GCP] node {} is CraftNode, adding all pins", node_id);
                            for p in n.base.all_pins() {
                                if visited_pins.insert(p.id) {
                                    queue.push_back(p.id);
                                }
                            }
                        } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                            // Merger and custom splitters are special: do NOT traverse to other pins
                            // on the same node (they represent independent flows). For other organizer
                            // kinds (including GameSplitter) include all pins of the node as before.
                            match n.base.kind {
                                NodeKind::Merger | NodeKind::CustomSplitter => {
                                    log::trace!(
                                        "[GCP] node {} is Merger/CustomSplitter, NOT adding other pins",
                                        node_id
                                    );
                                    // Do nothing: only the starting pin remains in visited_pins
                                }
                                _ => {
                                    log::trace!(
                                        "[GCP] node {} is GameSplitter, adding all pins",
                                        node_id
                                    );
                                    for p in n.base.all_pins() {
                                        if visited_pins.insert(p.id) {
                                            queue.push_back(p.id);
                                        }
                                    }
                                }
                            }
                        } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
                            log::trace!("[GCP] node {} is GroupNode, adding all pins", node_id);
                            for p in n.base.all_pins() {
                                if visited_pins.insert(p.id) {
                                    queue.push_back(p.id);
                                }
                            }
                        } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
                            log::trace!("[GCP] node {} is SinkNode, adding input pins", node_id);
                            for p in n.base.ins.iter() {
                                if visited_pins.insert(p.id) {
                                    queue.push_back(p.id);
                                }
                            }
                        }
                    }
                }
            }
        }

        log::trace!("[GCP] result: {:?}", visited_pins);
        visited_pins.into_iter().collect()
    }

    /// Propagate production rate changes through the graph starting at a pin.
    /// Returns Err(...) if constraints are contradictory or solver fails.
    pub fn update_nodes_rate(
        &mut self,
        constraint_pin_id: u64,
        constraint_value: FractionalNumber,
    ) -> Result<(), String> {
        use std::collections::{HashSet, VecDeque};

        // Reset pin errors for all pins
        for node_any in &mut self.nodes {
            if let Some(n) = node_any.downcast_mut::<CraftNode>() {
                for p in &mut n.base.ins {
                    p.error = false;
                }
                for p in &mut n.base.outs {
                    p.error = false;
                }
            } else if let Some(n) = node_any.downcast_mut::<OrganizerNode>() {
                for p in &mut n.base.ins {
                    p.error = false;
                }
                for p in &mut n.base.outs {
                    p.error = false;
                }
            } else if let Some(n) = node_any.downcast_mut::<GroupNode>() {
                for p in &mut n.base.ins {
                    p.error = false;
                }
                for p in &mut n.base.outs {
                    p.error = false;
                }
            } else if let Some(n) = node_any.downcast_mut::<SinkNode>() {
                for p in &mut n.base.ins {
                    p.error = false;
                }
            }
        }

        // Collect relevant pins via BFS starting from constraint_pin_id
        let mut queue: VecDeque<u64> = VecDeque::new();
        let mut visited: HashSet<u64> = HashSet::new();
        queue.push_back(constraint_pin_id);

        while let Some(pid) = queue.pop_front() {
            if !visited.insert(pid) {
                continue;
            }

            // Add pins from the same node depending on node kind
            if let Some((node_id, dir, _idx)) = self.find_pin_location(pid) {
                if let Some(ni) = self.find_node_index(node_id) {
                    let node_any = &self.nodes[ni];
                    if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                        for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                            if !visited.contains(&p.id) {
                                queue.push_back(p.id);
                            }
                        }
                    } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                        // For Merger/CustomSplitter: only add opposite-side pins that are CONNECTED
                        // This matches the C++ behavior where each multi-pin side is independent
                        // and we only propagate through established links, not to disconnected pins
                        match n.base.kind {
                            NodeKind::Merger | NodeKind::CustomSplitter => {
                                // Only propagate to opposite side, but only if the pin is connected
                                let opposite_pins: &[Pin] = match dir {
                                    PinDirection::Input => &n.base.outs,
                                    PinDirection::Output => &n.base.ins,
                                };
                                for p in opposite_pins {
                                    // Only add if this pin has a link (is connected to something)
                                    let has_link = self
                                        .links
                                        .iter()
                                        .any(|l| l.start_pin_id == p.id || l.end_pin_id == p.id);
                                    if has_link && !visited.contains(&p.id) {
                                        queue.push_back(p.id);
                                    }
                                }
                            }
                            _ => {
                                // GameSplitter: add all pins (ratio-based, one variable per node)
                                for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                                    if !visited.contains(&p.id) {
                                        queue.push_back(p.id);
                                    }
                                }
                            }
                        }
                    } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
                        for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                            if !visited.contains(&p.id) {
                                queue.push_back(p.id);
                            }
                        }
                    } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
                        for p in n.base.ins.iter() {
                            if !visited.contains(&p.id) {
                                queue.push_back(p.id);
                            }
                        }
                    }
                }
            }

            // Follow links to connected pins
            if let Some(link) = self.find_link_by_pin(pid) {
                let other = if link.start_pin_id == pid {
                    link.end_pin_id
                } else {
                    link.start_pin_id
                };
                if !visited.contains(&other) {
                    queue.push_back(other);
                }
            }
        }

        let relevant_pins: Vec<u64> = visited.into_iter().collect();

        // Build variable mapping: for craft/group/game-splitter -> one variable per NODE (ratio = pin.base_rate)
        // for Merger/CustomSplitter/Sink -> one variable per PIN (ratio = 1)
        // Map: pin_id -> (var_index, ratio)
        let mut pin_to_var: std::collections::HashMap<u64, (usize, FractionalNumber)> =
            std::collections::HashMap::new();
        let mut var_idx = 0usize;
        let mut locked_rates: std::collections::HashMap<u64, FractionalNumber> =
            std::collections::HashMap::new();

        // Track nodes that already have a variable (for craft/group/game splitter)
        let mut node_has_var: std::collections::HashMap<u64, usize> =
            std::collections::HashMap::new();

        for pid in &relevant_pins {
            if let Some((node_id, direction, idx)) = self.find_pin_location(*pid) {
                if let Some(ni) = self.find_node_index(node_id) {
                    let node_any = &self.nodes[ni];
                    let p_locked = if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                        match direction {
                            PinDirection::Input => n.base.ins[idx].locked,
                            PinDirection::Output => n.base.outs[idx].locked,
                        }
                    } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                        match direction {
                            PinDirection::Input => n.base.ins[idx].locked,
                            PinDirection::Output => n.base.outs[idx].locked,
                        }
                    } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
                        match direction {
                            PinDirection::Input => n.base.ins[idx].locked,
                            PinDirection::Output => n.base.outs[idx].locked,
                        }
                    } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
                        n.base.ins[idx].locked
                    } else {
                        false
                    };

                    let current_rate = if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                        match direction {
                            PinDirection::Input => n.base.ins[idx].current_rate,
                            PinDirection::Output => n.base.outs[idx].current_rate,
                        }
                    } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                        match direction {
                            PinDirection::Input => n.base.ins[idx].current_rate,
                            PinDirection::Output => n.base.outs[idx].current_rate,
                        }
                    } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
                        match direction {
                            PinDirection::Input => n.base.ins[idx].current_rate,
                            PinDirection::Output => n.base.outs[idx].current_rate,
                        }
                    } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
                        n.base.ins[idx].current_rate
                    } else {
                        FractionalNumber::default()
                    };

                    if p_locked {
                        locked_rates.insert(*pid, current_rate);
                        log::debug!(
                            "[SOLVER] pin {} is locked with rate {}",
                            *pid,
                            current_rate.to_fraction_string()
                        );
                        continue;
                    }

                    // For craft/group/game splitter, use one variable per node with ratio = pin.base_rate
                    if node_any.downcast_ref::<CraftNode>().is_some()
                        || node_any.downcast_ref::<GroupNode>().is_some()
                        || (node_any.downcast_ref::<OrganizerNode>().is_some() && {
                            let n = node_any.downcast_ref::<OrganizerNode>().unwrap();
                            n.base.kind == NodeKind::GameSplitter
                        })
                    {
                        if let Some(&vi) = node_has_var.get(&node_id) {
                            // Use existing var index
                            let ratio = if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                                match direction {
                                    PinDirection::Input => n.base.ins[idx].base_rate,
                                    PinDirection::Output => n.base.outs[idx].base_rate,
                                }
                            } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
                                match direction {
                                    PinDirection::Input => n.base.ins[idx].base_rate,
                                    PinDirection::Output => n.base.outs[idx].base_rate,
                                }
                            } else {
                                // GameSplitter treated like other nodes
                                let n = node_any.downcast_ref::<OrganizerNode>().unwrap();
                                match direction {
                                    PinDirection::Input => n.base.ins[idx].base_rate,
                                    PinDirection::Output => n.base.outs[idx].base_rate,
                                }
                            };
                            pin_to_var.insert(*pid, (vi, ratio));
                            log::debug!(
                                "[SOLVER] pin {} (craft/group existing var) assigned var {} with ratio {}",
                                *pid,
                                vi,
                                ratio.to_fraction_string()
                            );
                        } else {
                            // Assign new var for this node
                            let vi = var_idx;
                            var_idx += 1;
                            node_has_var.insert(node_id, vi);
                            log::debug!(
                                "[SOLVER] pin {} (craft/group new var) assigned var {} for node {}",
                                *pid,
                                vi,
                                node_id
                            );
                            let ratio = if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                                match direction {
                                    PinDirection::Input => n.base.ins[idx].base_rate,
                                    PinDirection::Output => n.base.outs[idx].base_rate,
                                }
                            } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
                                match direction {
                                    PinDirection::Input => n.base.ins[idx].base_rate,
                                    PinDirection::Output => n.base.outs[idx].base_rate,
                                }
                            } else {
                                let n = node_any.downcast_ref::<OrganizerNode>().unwrap();
                                match direction {
                                    PinDirection::Input => n.base.ins[idx].base_rate,
                                    PinDirection::Output => n.base.outs[idx].base_rate,
                                }
                            };
                            pin_to_var.insert(*pid, (vi, ratio));
                        }
                    }
                    // For Merger/CustomSplitter/Sink: one variable per pin, ratio = 1
                    else if node_any.downcast_ref::<OrganizerNode>().is_some()
                        || node_any.downcast_ref::<SinkNode>().is_some()
                    {
                        let vi = var_idx;
                        var_idx += 1;
                        pin_to_var.insert(*pid, (vi, FractionalNumber::new(1, 1)));
                        log::debug!(
                            "[SOLVER] pin {} (organizer/sink) assigned var {} with ratio 1",
                            *pid,
                            vi
                        );
                    }
                }
            }
        }

        log::debug!(
            "[SOLVER] relevant_pins={:?} locked_rates={:?} pin_to_var={:?}",
            relevant_pins,
            locked_rates,
            pin_to_var
        );

        // If there are no variables, just check locked consistency across links and still update touched nodes
        if pin_to_var.is_empty() {
            for link in &self.links {
                if relevant_pins.contains(&link.start_pin_id)
                    && relevant_pins.contains(&link.end_pin_id)
                {
                    let (s_node, s_dir, s_idx) = self.find_pin_location(link.start_pin_id).unwrap();
                    let (e_node, e_dir, e_idx) = self.find_pin_location(link.end_pin_id).unwrap();
                    let s_rate = if let Some(n) = self.nodes[self.find_node_index(s_node).unwrap()]
                        .downcast_ref::<CraftNode>()
                    {
                        match s_dir {
                            PinDirection::Input => n.base.ins[s_idx].current_rate,
                            PinDirection::Output => n.base.outs[s_idx].current_rate,
                        }
                    } else if let Some(n) = self.nodes[self.find_node_index(s_node).unwrap()]
                        .downcast_ref::<OrganizerNode>()
                    {
                        match s_dir {
                            PinDirection::Input => n.base.ins[s_idx].current_rate,
                            PinDirection::Output => n.base.outs[s_idx].current_rate,
                        }
                    } else if let Some(n) = self.nodes[self.find_node_index(s_node).unwrap()]
                        .downcast_ref::<GroupNode>()
                    {
                        match s_dir {
                            PinDirection::Input => n.base.ins[s_idx].current_rate,
                            PinDirection::Output => n.base.outs[s_idx].current_rate,
                        }
                    } else if let Some(n) =
                        self.nodes[self.find_node_index(s_node).unwrap()].downcast_ref::<SinkNode>()
                    {
                        n.base.ins[s_idx].current_rate
                    } else {
                        FractionalNumber::default()
                    };

                    let e_rate = if let Some(n) = self.nodes[self.find_node_index(e_node).unwrap()]
                        .downcast_ref::<CraftNode>()
                    {
                        match e_dir {
                            PinDirection::Input => n.base.ins[e_idx].current_rate,
                            PinDirection::Output => n.base.outs[e_idx].current_rate,
                        }
                    } else if let Some(n) = self.nodes[self.find_node_index(e_node).unwrap()]
                        .downcast_ref::<OrganizerNode>()
                    {
                        match e_dir {
                            PinDirection::Input => n.base.ins[e_idx].current_rate,
                            PinDirection::Output => n.base.outs[e_idx].current_rate,
                        }
                    } else if let Some(n) = self.nodes[self.find_node_index(e_node).unwrap()]
                        .downcast_ref::<GroupNode>()
                    {
                        match e_dir {
                            PinDirection::Input => n.base.ins[e_idx].current_rate,
                            PinDirection::Output => n.base.outs[e_idx].current_rate,
                        }
                    } else if let Some(n) =
                        self.nodes[self.find_node_index(e_node).unwrap()].downcast_ref::<SinkNode>()
                    {
                        n.base.ins[e_idx].current_rate
                    } else {
                        FractionalNumber::default()
                    };

                    if s_rate != e_rate {
                        return Err("Contradictory locked rates".into());
                    }
                }
            }

            // Even without variables, some nodes may be touched (single-node edits). Recompute node rates from pins for touched nodes
            for node_any in &mut self.nodes {
                if let Some(n) = node_any.downcast_mut::<CraftNode>() {
                    // Prefer outputs for craft nodes when available
                    if !n.base.outs.is_empty() {
                        let mut chosen: Option<(FractionalNumber, FractionalNumber)> = None;
                        for p in &n.base.outs {
                            if relevant_pins.contains(&p.id) {
                                let denom = p.base_rate
                                    * (FractionalNumber::new(1, 1)
                                        + n.num_somersloop * n.somersloop_mult);
                                if denom.numerator() != 0 {
                                    chosen = Some((p.current_rate / denom, p.current_rate));
                                    break;
                                }
                            }
                        }
                        if chosen.is_none() {
                            for p in &n.base.outs {
                                if p.base_rate.numerator() != 0 {
                                    chosen = Some((p.current_rate / p.base_rate, p.current_rate));
                                    break;
                                }
                            }
                        }
                        if let Some((new_node_rate, _)) = chosen {
                            if new_node_rate != n.current_rate {
                                n.update_rate(new_node_rate);
                            }
                        }
                    } else if !n.base.ins.is_empty() {
                        for p in &n.base.ins {
                            if p.base_rate.numerator() != 0 {
                                let new_node_rate = p.current_rate / p.base_rate;
                                if new_node_rate != n.current_rate {
                                    n.update_rate(new_node_rate);
                                }
                                break;
                            }
                        }
                    }
                } else if let Some(n) = node_any.downcast_mut::<GroupNode>() {
                    let mut chosen: Option<FractionalNumber> = None;
                    for p in &n.base.outs {
                        if relevant_pins.contains(&p.id) && p.base_rate.numerator() != 0 {
                            chosen = Some(p.current_rate / p.base_rate);
                            break;
                        }
                    }
                    if chosen.is_none() {
                        for p in &n.base.ins {
                            if relevant_pins.contains(&p.id) && p.base_rate.numerator() != 0 {
                                chosen = Some(p.current_rate / p.base_rate);
                                break;
                            }
                        }
                    }
                    if let Some(new_node_rate) = chosen {
                        if new_node_rate != n.current_rate {
                            n.current_rate = new_node_rate;
                            // Propagate to pins
                            for p in &mut n.base.ins {
                                p.current_rate = p.base_rate * n.current_rate;
                            }
                            for p in &mut n.base.outs {
                                p.current_rate = p.base_rate * n.current_rate;
                            }
                        }
                    }
                }
            }

            return Ok(());
        }

        // Build equations and constants
        let num_vars = var_idx; // number of variables (we assigned indices sequentially)
        let mut equations: Vec<Vec<FractionalNumber>> = Vec::new();
        let mut constants: Vec<FractionalNumber> = Vec::new();

        // Equality per link: start_rate - end_rate = 0 (taking ratios into account)
        for link in &self.links {
            if !(relevant_pins.contains(&link.start_pin_id)
                && relevant_pins.contains(&link.end_pin_id))
            {
                continue;
            }
            let s = link.start_pin_id;
            let e = link.end_pin_id;
            let mut eq = vec![FractionalNumber::new(0, 1); num_vars];
            let mut constant = FractionalNumber::new(0, 1);

            if let Some((si, sratio)) = pin_to_var.get(&s).copied() {
                eq[si] = eq[si].clone() + sratio;
            } else if let Some(rate) = locked_rates.get(&s) {
                constant = constant - rate.clone();
            }

            if let Some((ei, eratio)) = pin_to_var.get(&e).copied() {
                eq[ei] = eq[ei].clone() + (FractionalNumber::new(-1, 1) * eratio);
            } else if let Some(rate) = locked_rates.get(&e) {
                constant = constant + rate.clone();
            }

            // If equation is all zeros and constant non-zero -> contradiction
            if eq.iter().all(|c| c.numerator() == 0) && constant.numerator() != 0 {
                return Err("Contradictory locked rates".into());
            }

            equations.push(eq);
            constants.push(constant);
        }

        // Organizer node: sum(inputs) - sum(outputs) = 0 (use ratios)
        // For Merger/CustomSplitter, only build the constraint if ALL pins are part of the
        // relevant set (i.e., all pins are connected and involved in this propagation).
        // Otherwise, skip the constraint since each pin flow is independent.
        for node_any in &self.nodes {
            if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                // For Merger/CustomSplitter, check if ALL pins are in relevant set
                let is_pin_level =
                    matches!(n.base.kind, NodeKind::Merger | NodeKind::CustomSplitter);

                if is_pin_level {
                    // Only build sum constraint if ALL pins are in relevant set
                    let all_pins_relevant = n
                        .base
                        .ins
                        .iter()
                        .chain(n.base.outs.iter())
                        .all(|p| relevant_pins.contains(&p.id));
                    if !all_pins_relevant {
                        // Skip sum constraint for partial connections - pins are independent
                        continue;
                    }
                } else {
                    // For other organizers (GameSplitter), check if any pin is touched
                    let mut touched = false;
                    for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                        if relevant_pins.contains(&p.id) {
                            touched = true;
                            break;
                        }
                    }
                    if !touched {
                        continue;
                    }
                }

                let mut eq = vec![FractionalNumber::new(0, 1); num_vars];
                let mut constant = FractionalNumber::new(0, 1);

                for p in &n.base.ins {
                    if let Some((vi, ratio)) = pin_to_var.get(&p.id).copied() {
                        eq[vi] = eq[vi].clone() + ratio;
                    } else {
                        constant = constant - p.current_rate.clone();
                    }
                }
                for p in &n.base.outs {
                    if let Some((vi, ratio)) = pin_to_var.get(&p.id).copied() {
                        eq[vi] = eq[vi].clone() + (FractionalNumber::new(-1, 1) * ratio);
                    } else {
                        constant = constant + p.current_rate.clone();
                    }
                }

                if eq.iter().all(|c| c.numerator() == 0) && constant.numerator() != 0 {
                    return Err("Contradictory locked rates on organizer".into());
                }

                equations.push(eq);
                constants.push(constant);
            }
        }

        // Add the constraint for the user-provided pin value (pin = constraint_value)
        if let Some((vi, ratio)) = pin_to_var.get(&constraint_pin_id).copied() {
            let mut eq = vec![FractionalNumber::new(0, 1); num_vars];
            // vi * ratio = constraint_value
            eq[vi] = ratio;
            equations.push(eq);
            constants.push(constraint_value.clone());
        } else if let Some(r) = locked_rates.get(&constraint_pin_id) {
            if *r != constraint_value {
                return Err("Contradiction with locked constraint value".into());
            }
        } else {
            // Should not happen: constraint pin not in relevant set
            return Err("Constraint pin not part of propagation set".into());
        }

        if equations.is_empty() {
            return Err("No equations to solve".into());
        }

        // Solve system
        let solver =
            crate::rate_calculator::LinearSolver::new(equations.clone(), constants.clone())
                .map_err(|e| format!("Solver setup failed: {}", e))?;
        let solution = solver.solve().map_err(|e| {
            format!(
                "Solver error: {}\nEquations: {:?}\nConstants: {:?}",
                e, equations, constants
            )
        })?;

        log::debug!("[SOLVER] solution={:?}", solution);

        // Apply solution to pins
        for (pid, &(vi, ratio)) in &pin_to_var {
            if vi >= solution.len() {
                continue;
            }
            let var_value = solution[vi].clone();
            let new_rate = var_value * ratio;
            if let Some((node_id, direction, idx)) = self.find_pin_location(*pid) {
                let ni = self.find_node_index(node_id).unwrap();
                if let Some(n) = self.nodes[ni].downcast_mut::<CraftNode>() {
                    match direction {
                        PinDirection::Input => n.base.ins[idx].current_rate = new_rate,
                        PinDirection::Output => n.base.outs[idx].current_rate = new_rate,
                    }
                } else if let Some(n) = self.nodes[ni].downcast_mut::<OrganizerNode>() {
                    match direction {
                        PinDirection::Input => n.base.ins[idx].current_rate = new_rate,
                        PinDirection::Output => n.base.outs[idx].current_rate = new_rate,
                    }
                } else if let Some(n) = self.nodes[ni].downcast_mut::<GroupNode>() {
                    match direction {
                        PinDirection::Input => n.base.ins[idx].current_rate = new_rate,
                        PinDirection::Output => n.base.outs[idx].current_rate = new_rate,
                    }
                } else if let Some(n) = self.nodes[ni].downcast_mut::<SinkNode>() {
                    n.base.ins[idx].current_rate = new_rate;
                }
            }
        }

        // For locked pins, keep their rates as-is (already present)

        // Update node rates from pins
        for node_any in &mut self.nodes {
            if let Some(n) = node_any.downcast_mut::<CraftNode>() {
                // Prefer outputs for craft nodes when available
                if !n.base.outs.is_empty() {
                    // Use first out that was part of propagation, else any
                    let mut chosen: Option<(FractionalNumber, FractionalNumber)> = None;
                    for p in &n.base.outs {
                        if relevant_pins.contains(&p.id) {
                            // Node rate = out.current / (out.base * (1 + num_somersloop * somersloop_mult))
                            let denom = p.base_rate
                                * (FractionalNumber::new(1, 1)
                                    + n.num_somersloop * n.somersloop_mult);
                            if denom.numerator() != 0 {
                                chosen = Some((p.current_rate / denom, p.current_rate));
                                break;
                            }
                        }
                    }
                    if chosen.is_none() {
                        for p in &n.base.outs {
                            if p.base_rate.numerator() != 0 {
                                chosen = Some((p.current_rate / p.base_rate, p.current_rate));
                                break;
                            }
                        }
                    }
                    if let Some((new_node_rate, _)) = chosen {
                        if new_node_rate != n.current_rate {
                            n.update_rate(new_node_rate);
                        }
                    }
                } else if !n.base.ins.is_empty() {
                    // Fall back to inputs
                    for p in &n.base.ins {
                        if p.base_rate.numerator() != 0 {
                            let new_node_rate = p.current_rate / p.base_rate;
                            if new_node_rate != n.current_rate {
                                n.update_rate(new_node_rate);
                            }
                            break;
                        }
                    }
                }
            } else if let Some(n) = node_any.downcast_mut::<GroupNode>() {
                // Determine node rate from any touched pin
                let mut chosen: Option<FractionalNumber> = None;
                for p in &n.base.outs {
                    if relevant_pins.contains(&p.id) && p.base_rate.numerator() != 0 {
                        chosen = Some(p.current_rate / p.base_rate);
                        break;
                    }
                }
                if chosen.is_none() {
                    for p in &n.base.ins {
                        if relevant_pins.contains(&p.id) && p.base_rate.numerator() != 0 {
                            chosen = Some(p.current_rate / p.base_rate);
                            break;
                        }
                    }
                }
                if let Some(new_node_rate) = chosen {
                    if new_node_rate != n.current_rate {
                        n.current_rate = new_node_rate;
                        // Propagate to pins
                        for p in &mut n.base.ins {
                            p.current_rate = p.base_rate * n.current_rate;
                        }
                        for p in &mut n.base.outs {
                            p.current_rate = p.base_rate * n.current_rate;
                        }
                    }
                }
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

        // Load nodes (record which nodes were saved as locked so we can restore after links exist)
        let locked_node_indices: Vec<usize> = file
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| match n {
                SerializedNode::Craft(c) if c.locked => Some(i),
                _ => None,
            })
            .collect();

        // Collect any explicit organizer pin values present in the file (C++ exports may include them)
        // We'll restore these after links/locks/propagation to preserve the author's saved values.
        let mut organizer_saved_pins: Vec<(
            usize,
            bool,
            usize,
            crate::fractional_number::FractionalNumber,
            bool,
        )> = Vec::new();
        // Also save craft node rates so we can re-apply them after propagation (preserve visuals)
        let mut craft_saved_rates: Vec<(usize, crate::fractional_number::FractionalNumber)> =
            Vec::new();
        for (idx, serialized_node) in file.nodes.iter().enumerate() {
            if let SerializedNode::Organizer(org) = serialized_node {
                if let Some(ins) = &org.ins {
                    for (i, entry) in ins.iter().enumerate() {
                        organizer_saved_pins.push((
                            idx,
                            true,
                            i,
                            FractionalNumber::new(entry.num, entry.den),
                            entry.locked,
                        ));
                    }
                }
                if let Some(outs) = &org.outs {
                    for (i, entry) in outs.iter().enumerate() {
                        organizer_saved_pins.push((
                            idx,
                            false,
                            i,
                            FractionalNumber::new(entry.num, entry.den),
                            entry.locked,
                        ));
                    }
                }
            } else if let SerializedNode::Craft(c) = serialized_node {
                craft_saved_rates.push((idx, FractionalNumber::new(c.rate.num, c.rate.den)));
            }
            self.load_node(serialized_node, idx, game_data)?;
        }

        // Load links
        for serialized_link in &file.links {
            self.load_link(serialized_link)?;
        }

        // Apply node-level locks recorded in file now that links exist
        for idx in locked_node_indices {
            if let Some(node_any) = self.nodes.get(idx) {
                if let Some(craft) = node_any.downcast_ref::<CraftNode>() {
                    let node_id = craft.base.id;
                    // Best-effort: ignore errors applying lock to be permissive for slightly malformed files
                    let _ = self.set_node_locked(node_id, true);
                }
            }
        }

        // Run auto-lock logic for organizer nodes so single-pin rates are derived when
        // all multi-pins are locked (e.g., splitter outputs locked -> input computed).
        // Repeat until stable to catch cascaded locks.
        let mut counter = 0;
        loop {
            // Snapshot locked state to detect changes
            let before_locked: Vec<u64> = self
                .nodes
                .iter()
                .flat_map(|n| {
                    if let Some(org) = n.downcast_ref::<OrganizerNode>() {
                        org.base
                            .ins
                            .iter()
                            .chain(org.base.outs.iter())
                            .filter(|p| p.locked)
                            .map(|p| p.id)
                            .collect::<Vec<u64>>()
                    } else {
                        Vec::new()
                    }
                })
                .collect();

            self.auto_lock_organizer_pins();

            let after_locked: Vec<u64> = self
                .nodes
                .iter()
                .flat_map(|n| {
                    if let Some(org) = n.downcast_ref::<OrganizerNode>() {
                        org.base
                            .ins
                            .iter()
                            .chain(org.base.outs.iter())
                            .filter(|p| p.locked)
                            .map(|p| p.id)
                            .collect::<Vec<u64>>()
                    } else {
                        Vec::new()
                    }
                })
                .collect();

            if after_locked.len() == before_locked.len() {
                log::info!("[LOAD] auto-lock stable after {} iterations", counter);
                break;
            } else {
                counter += 1;
            }
        }

        // Collect all locked pins first (so we can call propagation mutably later)
        let mut locked_pins: Vec<(u64, FractionalNumber)> = Vec::new();
        for node_any in &self.nodes {
            if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                    if p.locked {
                        locked_pins.push((p.id, p.current_rate));
                    }
                }
            } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                    if p.locked {
                        locked_pins.push((p.id, p.current_rate));
                    }
                }
            } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
                for p in n.base.ins.iter().chain(n.base.outs.iter()) {
                    if p.locked {
                        locked_pins.push((p.id, p.current_rate));
                    }
                }
            } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
                for p in n.base.ins.iter() {
                    if p.locked {
                        locked_pins.push((p.id, p.current_rate));
                    }
                }
            }
        }

        // Propagate from locked pins (call propagation for each pinned constraint).
        // Previously we de-duplicated per connected component to reduce redundant solves,
        // but that can inadvertently create large mixed systems. Calling per-pin keeps
        // behavior consistent with prior logic and fixes correctness for some C++ exports.
        for (pid, rate) in locked_pins {
            if let Err(e) = self.update_nodes_rate(pid, rate) {
                log::debug!("[LOAD] propagation from pin {} failed: {}", pid, e);
            }
        }

        // Restore explicit organizer pin rates from file (best-effort). This ensures C++ exported
        // ins/outs values are preserved visually even when propagation touched them.
        // Note: organizer_saved_pins uses file node indices which map to self.nodes in same order.
        for (file_idx, is_in, pin_idx, saved_rate, saved_locked) in organizer_saved_pins {
            if file_idx >= self.nodes.len() {
                continue;
            }
            if let Some(n_any) = self.nodes[file_idx].downcast_mut::<OrganizerNode>() {
                if is_in {
                    if pin_idx < n_any.base.ins.len() {
                        n_any.base.ins[pin_idx].current_rate = saved_rate;
                        n_any.base.ins[pin_idx].locked = saved_locked;
                    }
                } else {
                    if pin_idx < n_any.base.outs.len() {
                        n_any.base.outs[pin_idx].current_rate = saved_rate;
                        n_any.base.outs[pin_idx].locked = saved_locked;
                    }
                }
            }
        }

        // Re-apply saved craft node rates to preserve visual node rates from file (best-effort)
        for (file_idx, saved_rate) in craft_saved_rates {
            if file_idx >= self.nodes.len() {
                continue;
            }
            if let Some(n_any) = self.nodes[file_idx].downcast_mut::<CraftNode>() {
                n_any.update_rate(saved_rate);
            }
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

                            // If recipe doesn't have explicit power, use building power (for power generators)
                            if node.recipe_power == 0.0 {
                                node.recipe_power = building.power;
                            }
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

                // Update pin current rates based on node rate
                node.update_rate(node.current_rate);

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
                    let mut pin = Pin::new(
                        pin_id,
                        PinDirection::Input,
                        node_id,
                        Some(input.item.clone()),
                        input.locked,
                        rate,
                    );
                    // Restore current rate to match saved rate
                    pin.current_rate = rate;
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

                // If the serialized organizer includes explicit pin descriptions (some C++ exports do),
                // restore those pins and their rates/locked flags so the graph looks correct on load.
                if let Some(ins) = &org.ins {
                    for entry in ins {
                        let pin_id = self.get_next_id();
                        let base_rate = FractionalNumber::new(entry.num, entry.den);
                        let mut pin = Pin::new(
                            pin_id,
                            PinDirection::Input,
                            node_id,
                            entry.item.clone().or_else(|| org.item.clone()),
                            entry.locked,
                            base_rate,
                        );
                        // Restore current rate to saved value
                        pin.current_rate = base_rate;
                        node.base.ins.push(pin);
                    }
                }
                if let Some(outs) = &org.outs {
                    for entry in outs {
                        let pin_id = self.get_next_id();
                        let base_rate = FractionalNumber::new(entry.num, entry.den);
                        let mut pin = Pin::new(
                            pin_id,
                            PinDirection::Output,
                            node_id,
                            entry.item.clone().or_else(|| org.item.clone()),
                            entry.locked,
                            base_rate,
                        );
                        pin.current_rate = base_rate;
                        node.base.outs.push(pin);
                    }
                }

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
        if self.nodes[node_idx]
            .downcast_ref::<OrganizerNode>()
            .is_some()
        {
            // Read current length without holding a mutable borrow
            let current_len = {
                let org_ref = self.nodes[node_idx]
                    .downcast_ref::<OrganizerNode>()
                    .unwrap();
                match direction {
                    PinDirection::Input => org_ref.base.ins.len(),
                    PinDirection::Output => org_ref.base.outs.len(),
                }
            };
            if pin_idx < current_len {
                let org_ref = self.nodes[node_idx]
                    .downcast_ref::<OrganizerNode>()
                    .unwrap();
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
            let org_mut = self.nodes[node_idx]
                .downcast_mut::<OrganizerNode>()
                .unwrap();
            for new_pin_id in new_ids {
                match direction {
                    PinDirection::Input => {
                        let locked = org_mut.base.outs.get(0).map(|p| p.locked).unwrap_or(false);
                        org_mut.base.ins.push(Pin::new(
                            new_pin_id,
                            PinDirection::Input,
                            org_mut.base.id,
                            org_mut.item_name.clone(),
                            locked,
                            FractionalNumber::default(),
                        ));
                    }
                    PinDirection::Output => {
                        let locked = org_mut.base.ins.get(0).map(|p| p.locked).unwrap_or(false);
                        org_mut.base.outs.push(Pin::new(
                            new_pin_id,
                            PinDirection::Output,
                            org_mut.base.id,
                            org_mut.item_name.clone(),
                            locked,
                            FractionalNumber::default(),
                        ));
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
                locked: craft
                    .base
                    .ins
                    .iter()
                    .chain(craft.base.outs.iter())
                    .all(|p| p.locked),
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
            // Optionally emit explicit ins/outs arrays for organizers when pins have non-default
            // values (base_rate != 0), have item names, or are locked. This improves
            // compatibility with C++ exported files and allows round-trip preservation.
            let mut ins_vec: Option<Vec<crate::serialization::SerializedPinEntry>> = None;
            let mut outs_vec: Option<Vec<crate::serialization::SerializedPinEntry>> = None;

            if org.base.ins.iter().any(|p| {
                p.base_rate != FractionalNumber::default() || p.locked || p.item_name.is_some()
            }) {
                let mut v = Vec::new();
                for p in &org.base.ins {
                    v.push(crate::serialization::SerializedPinEntry {
                        item: p.item_name.clone(),
                        num: p.base_rate.numerator(),
                        den: p.base_rate.denominator(),
                        locked: p.locked,
                    });
                }
                ins_vec = Some(v);
            }

            if org.base.outs.iter().any(|p| {
                p.base_rate != FractionalNumber::default() || p.locked || p.item_name.is_some()
            }) {
                let mut v = Vec::new();
                for p in &org.base.outs {
                    v.push(crate::serialization::SerializedPinEntry {
                        item: p.item_name.clone(),
                        num: p.base_rate.numerator(),
                        den: p.base_rate.denominator(),
                        locked: p.locked,
                    });
                }
                outs_vec = Some(v);
            }

            Some(SerializedNode::Organizer(SerializedOrganizerNode {
                kind: org.base.kind.to_kind_id(),
                pos: SerializedPosition {
                    x: org.base.position.0,
                    y: org.base.position.1,
                },
                item: org.item_name.clone(),
                ins: ins_vec,
                outs: outs_vec,
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
        // Create one input pin with base rate 1 and one output pin with base rate 1
        let in_pin = app.get_next_id();
        craft.base.ins.push(Pin::new(
            in_pin,
            PinDirection::Input,
            node_id,
            None,
            false,
            FractionalNumber::new(1, 1),
        ));
        let out_pin = app.get_next_id();
        craft.base.outs.push(Pin::new(
            out_pin,
            PinDirection::Output,
            node_id,
            None,
            false,
            FractionalNumber::new(1, 1),
        ));
        app.nodes.push(Box::new(craft));

        let node_idx = app.find_node_index(node_id).expect("find node");

        // Set input pin rate to 4 -> output and node rate should become 4
        app.set_pin_rate(node_id, PinDirection::Input, 0, FractionalNumber::new(4, 1))
            .expect("set_pin_rate");

        // Re-borrow immutably to assert
        let n = app.nodes[node_idx]
            .downcast_ref::<CraftNode>()
            .expect("expected craft node");
        assert_eq!(n.base.ins[0].current_rate, FractionalNumber::new(4, 1));
        assert_eq!(n.base.outs[0].current_rate, FractionalNumber::new(4, 1));
        assert_eq!(n.current_rate, FractionalNumber::new(4, 1));
    }

    #[test]
    fn looped_graph_has_valid_solution() {
        let mut app = ProductionApp::new();

        // Node A: consumes Water (base 2) and Bauxite (base 1), produces Alumina Solution (base 4)
        let a_id = app.get_next_id();
        let mut a = CraftNode::new(a_id, "AluminaProducer".to_string());
        let a_in_water = app.get_next_id();
        a.base.ins.push(Pin::new(
            a_in_water,
            PinDirection::Input,
            a_id,
            Some("Water".to_string()),
            false,
            FractionalNumber::new(2, 1),
        ));
        let a_in_bauxite = app.get_next_id();
        a.base.ins.push(Pin::new(
            a_in_bauxite,
            PinDirection::Input,
            a_id,
            Some("Bauxite".to_string()),
            false,
            FractionalNumber::new(1, 1),
        ));
        let a_out_alum = app.get_next_id();
        a.base.outs.push(Pin::new(
            a_out_alum,
            PinDirection::Output,
            a_id,
            Some("Alumina Solution".to_string()),
            false,
            FractionalNumber::new(4, 1),
        ));
        app.nodes.push(Box::new(a));

        // Node B: consumes Alumina Solution (base 2), produces Water (base 5/6) and Scrap
        // (water base chosen so that B produces 5 units when consuming 12 alumina)
        let b_id = app.get_next_id();
        let mut b = CraftNode::new(b_id, "AluminumScrapProducer".to_string());
        let b_in_alum = app.get_next_id();
        b.base.ins.push(Pin::new(
            b_in_alum,
            PinDirection::Input,
            b_id,
            Some("Alumina Solution".to_string()),
            false,
            FractionalNumber::new(2, 1),
        ));
        let b_out_water = app.get_next_id();
        b.base.outs.push(Pin::new(
            b_out_water,
            PinDirection::Output,
            b_id,
            Some("Water".to_string()),
            false,
            FractionalNumber::new(5, 6),
        ));
        let b_out_scrap = app.get_next_id();
        b.base.outs.push(Pin::new(
            b_out_scrap,
            PinDirection::Output,
            b_id,
            Some("Aluminum Scrap".to_string()),
            false,
            FractionalNumber::new(1, 1),
        ));
        app.nodes.push(Box::new(b));

        // Merger node to combine external water (1) with looped water from B
        let m_id = app.get_next_id();
        let mut m = OrganizerNode::new(m_id, NodeKind::Merger, None);
        // two inputs
        let m_in0 = app.get_next_id();
        m.base.ins.push(Pin::new(
            m_in0,
            PinDirection::Input,
            m_id,
            Some("Water".to_string()),
            false,
            FractionalNumber::default(),
        ));
        let m_in1 = app.get_next_id();
        m.base.ins.push(Pin::new(
            m_in1,
            PinDirection::Input,
            m_id,
            Some("Water".to_string()),
            true, // locked external supply
            FractionalNumber::default(),
        ));
        // set external locked supply to 1
        m.base.ins[1].current_rate = FractionalNumber::new(1, 1);
        // one output
        let m_out = app.get_next_id();
        m.base.outs.push(Pin::new(
            m_out,
            PinDirection::Output,
            m_id,
            Some("Water".to_string()),
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(m));

        // Connect A.out (Alumina Solution) -> B.in (Alumina Solution)
        let (_link1, _warn1) = app.create_link(a_out_alum, b_in_alum).expect("create link");
        // Connect B.out (Water) -> Merger.in0
        let (_link2, _warn2) = app.create_link(b_out_water, m_in0).expect("create link");
        // Connect Merger.out -> A.in (Water)
        let (_link3, _warn3) = app.create_link(m_out, a_in_water).expect("create link");

        // Constrain A's alumina output to 12 units
        app.update_nodes_rate(a_out_alum, FractionalNumber::new(12, 1))
            .expect("propagation should succeed");

        // Verify resulting flows
        // Re-borrow to check node/pin states
        let a_idx = app.find_node_index(a_id).unwrap();
        let b_idx = app.find_node_index(b_id).unwrap();
        let m_idx = app.find_node_index(m_id).unwrap();
        let a_node = app.nodes[a_idx].downcast_ref::<CraftNode>().unwrap();
        let b_node = app.nodes[b_idx].downcast_ref::<CraftNode>().unwrap();
        let m_node = app.nodes[m_idx].downcast_ref::<OrganizerNode>().unwrap();

        // A's output should be 12
        assert_eq!(
            a_node.base.outs[0].current_rate,
            FractionalNumber::new(12, 1)
        );
        // B's input should be 12
        assert_eq!(
            b_node.base.ins[0].current_rate,
            FractionalNumber::new(12, 1)
        );
        // B's water output should be 5 (6 * 5/6)
        assert_eq!(
            b_node.base.outs[0].current_rate,
            FractionalNumber::new(5, 1)
        );
        // Merger inputs: external locked 1 and B water 5
        assert_eq!(m_node.base.ins[1].current_rate, FractionalNumber::new(1, 1));
        assert_eq!(m_node.base.ins[0].current_rate, FractionalNumber::new(5, 1));
        // Merger output should be 6 and A's water input should be 6
        assert_eq!(
            m_node.base.outs[0].current_rate,
            FractionalNumber::new(6, 1)
        );
        assert_eq!(a_node.base.ins[0].current_rate, FractionalNumber::new(6, 1));
        // Node rates
        assert_eq!(a_node.current_rate, FractionalNumber::new(3, 1)); // 12 / 4
        assert_eq!(b_node.current_rate, FractionalNumber::new(6, 1)); // 12 / 2
    }

    #[test]
    fn get_connected_pins_chain() {
        let mut app = ProductionApp::new();
        // Node A
        let a_id = app.get_next_id();
        let mut a = CraftNode::new(a_id, "A".to_string());
        let a_in = app.get_next_id();
        a.base.ins.push(Pin::new(
            a_in,
            PinDirection::Input,
            a_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        let a_out = app.get_next_id();
        a.base.outs.push(Pin::new(
            a_out,
            PinDirection::Output,
            a_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(a));
        // Node B
        let b_id = app.get_next_id();
        let mut b = CraftNode::new(b_id, "B".to_string());
        let b_in = app.get_next_id();
        b.base.ins.push(Pin::new(
            b_in,
            PinDirection::Input,
            b_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        let b_out = app.get_next_id();
        b.base.outs.push(Pin::new(
            b_out,
            PinDirection::Output,
            b_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(b));

        // Connect A.out -> B.in
        let (_l, _w) = app.create_link(a_out, b_in).expect("create link");

        let connected = app.get_connected_pins(a_out);
        let set: std::collections::HashSet<u64> = connected.into_iter().collect();
        let expected: std::collections::HashSet<u64> =
            [a_in, a_out, b_in, b_out].iter().cloned().collect();
        assert_eq!(set, expected);
    }

    #[test]
    fn get_connected_pins_loop() {
        let mut app = ProductionApp::new();
        // Node A
        let a_id = app.get_next_id();
        let mut a = CraftNode::new(a_id, "A".to_string());
        let a_in = app.get_next_id();
        a.base.ins.push(Pin::new(
            a_in,
            PinDirection::Input,
            a_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        let a_out = app.get_next_id();
        a.base.outs.push(Pin::new(
            a_out,
            PinDirection::Output,
            a_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(a));
        // Node B
        let b_id = app.get_next_id();
        let mut b = CraftNode::new(b_id, "B".to_string());
        let b_in = app.get_next_id();
        b.base.ins.push(Pin::new(
            b_in,
            PinDirection::Input,
            b_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        let b_out = app.get_next_id();
        b.base.outs.push(Pin::new(
            b_out,
            PinDirection::Output,
            b_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(b));

        // Connect A.out -> B.in and B.out -> A.in (loop)
        let (_l1, _w1) = app.create_link(a_out, b_in).expect("create link");
        let (_l2, _w2) = app.create_link(b_out, a_in).expect("create link");

        let connected = app.get_connected_pins(a_out);
        let set: std::collections::HashSet<u64> = connected.into_iter().collect();
        let expected: std::collections::HashSet<u64> =
            [a_in, a_out, b_in, b_out].iter().cloned().collect();
        assert_eq!(set, expected);
    }

    #[test]
    fn get_connected_pins_merger() {
        let mut app = ProductionApp::new();
        // Merger node
        let m_id = app.get_next_id();
        let mut m = OrganizerNode::new(m_id, NodeKind::Merger, None);
        let m_in0 = app.get_next_id();
        m.base.ins.push(Pin::new(
            m_in0,
            PinDirection::Input,
            m_id,
            Some("X".to_string()),
            false,
            FractionalNumber::default(),
        ));
        let m_in1 = app.get_next_id();
        m.base.ins.push(Pin::new(
            m_in1,
            PinDirection::Input,
            m_id,
            Some("X".to_string()),
            false,
            FractionalNumber::default(),
        ));
        let m_out = app.get_next_id();
        m.base.outs.push(Pin::new(
            m_out,
            PinDirection::Output,
            m_id,
            Some("X".to_string()),
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(m));

        // Another node that feeds m_in0
        let n_id = app.get_next_id();
        let mut n = CraftNode::new(n_id, "N".to_string());
        let n_out = app.get_next_id();
        n.base.outs.push(Pin::new(
            n_out,
            PinDirection::Output,
            n_id,
            Some("X".to_string()),
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(n));

        // Connect n.out -> m.in0
        let (_l, _w) = app.create_link(n_out, m_in0).expect("create link");

        let connected = app.get_connected_pins(m_in0);
        let set: std::collections::HashSet<u64> = connected.into_iter().collect();
        let expected: std::collections::HashSet<u64> = [m_in0, n_out].iter().cloned().collect();
        assert_eq!(set, expected);
    }

    #[test]
    fn get_connected_pins_custom_splitter() {
        let mut app = ProductionApp::new();
        // Custom splitter node
        let s_id = app.get_next_id();
        let mut s = OrganizerNode::new(s_id, NodeKind::CustomSplitter, None);
        let s_in = app.get_next_id();
        s.base.ins.push(Pin::new(
            s_in,
            PinDirection::Input,
            s_id,
            Some("Y".to_string()),
            false,
            FractionalNumber::default(),
        ));
        let s_out0 = app.get_next_id();
        s.base.outs.push(Pin::new(
            s_out0,
            PinDirection::Output,
            s_id,
            Some("Y".to_string()),
            false,
            FractionalNumber::default(),
        ));
        let s_out1 = app.get_next_id();
        s.base.outs.push(Pin::new(
            s_out1,
            PinDirection::Output,
            s_id,
            Some("Y".to_string()),
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(s));

        // Another node that receives s_out0
        let n_id = app.get_next_id();
        let mut n = CraftNode::new(n_id, "N".to_string());
        let n_in = app.get_next_id();
        n.base.ins.push(Pin::new(
            n_in,
            PinDirection::Input,
            n_id,
            Some("Y".to_string()),
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(n));

        // Connect s.out0 -> n.in
        let (_l, _w) = app.create_link(s_out0, n_in).expect("create link");

        let connected = app.get_connected_pins(s_out0);
        let set: std::collections::HashSet<u64> = connected.into_iter().collect();
        // Custom splitter is a special case: don't include sibling pins on the splitter node
        let expected: std::collections::HashSet<u64> = [s_out0, n_in].iter().cloned().collect();
        assert_eq!(set, expected);
    }

    #[test]
    fn get_connected_pins_game_splitter() {
        let mut app = ProductionApp::new();
        // Game splitter node (treated as special as well)
        let s_id = app.get_next_id();
        let mut s = OrganizerNode::new(s_id, NodeKind::GameSplitter, None);
        let s_in = app.get_next_id();
        s.base.ins.push(Pin::new(
            s_in,
            PinDirection::Input,
            s_id,
            Some("Z".to_string()),
            false,
            FractionalNumber::default(),
        ));
        let s_out0 = app.get_next_id();
        s.base.outs.push(Pin::new(
            s_out0,
            PinDirection::Output,
            s_id,
            Some("Z".to_string()),
            false,
            FractionalNumber::default(),
        ));
        let s_out1 = app.get_next_id();
        s.base.outs.push(Pin::new(
            s_out1,
            PinDirection::Output,
            s_id,
            Some("Z".to_string()),
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(s));

        // Another node that receives s_out0
        let n_id = app.get_next_id();
        let mut n = CraftNode::new(n_id, "N".to_string());
        let n_in = app.get_next_id();
        n.base.ins.push(Pin::new(
            n_in,
            PinDirection::Input,
            n_id,
            Some("Z".to_string()),
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(n));

        // Connect s.out0 -> n.in
        let (_l, _w) = app.create_link(s_out0, n_in).expect("create link");

        let connected = app.get_connected_pins(s_out0);
        let set: std::collections::HashSet<u64> = connected.into_iter().collect();
        // Game splitter behaves like a regular node: include sibling pins on the splitter node
        let expected: std::collections::HashSet<u64> =
            [s_out0, s_out1, s_in, n_in].iter().cloned().collect();
        assert_eq!(set, expected);
    }

    #[test]
    fn get_connected_pins_lock_propagation() {
        use std::collections::HashSet;

        let mut app = ProductionApp::new();

        // Node A (craft)
        let a_id = app.get_next_id();
        let mut a = CraftNode::new(a_id, "A".to_string());
        let a_in = app.get_next_id();
        a.base.ins.push(Pin::new(
            a_in,
            PinDirection::Input,
            a_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        let a_out = app.get_next_id();
        a.base.outs.push(Pin::new(
            a_out,
            PinDirection::Output,
            a_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(a));

        // Game splitter node
        let s_id = app.get_next_id();
        let mut s = OrganizerNode::new(s_id, NodeKind::GameSplitter, None);
        let s_in = app.get_next_id();
        s.base.ins.push(Pin::new(
            s_in,
            PinDirection::Input,
            s_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        let s_out0 = app.get_next_id();
        s.base.outs.push(Pin::new(
            s_out0,
            PinDirection::Output,
            s_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        let s_out1 = app.get_next_id();
        s.base.outs.push(Pin::new(
            s_out1,
            PinDirection::Output,
            s_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(s));

        // Node B (craft)
        let b_id = app.get_next_id();
        let mut b = CraftNode::new(b_id, "B".to_string());
        let b_in = app.get_next_id();
        b.base.ins.push(Pin::new(
            b_in,
            PinDirection::Input,
            b_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        let b_out = app.get_next_id();
        b.base.outs.push(Pin::new(
            b_out,
            PinDirection::Output,
            b_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(b));

        // Node C (craft)
        let c_id = app.get_next_id();
        let mut c = CraftNode::new(c_id, "C".to_string());
        let c_in = app.get_next_id();
        c.base.ins.push(Pin::new(
            c_in,
            PinDirection::Input,
            c_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        let c_out = app.get_next_id();
        c.base.outs.push(Pin::new(
            c_out,
            PinDirection::Output,
            c_id,
            None,
            false,
            FractionalNumber::default(),
        ));
        app.nodes.push(Box::new(c));

        // Connect A.out -> S.in, S.out0 -> B.in, S.out1 -> C.in
        app.create_link(a_out, s_in).expect("create link");
        app.create_link(s_out0, b_in).expect("create link");
        app.create_link(s_out1, c_in).expect("create link");

        // Get connected pins starting from a_out
        let connected = app.get_connected_pins(a_out);
        let connected_set: HashSet<u64> = connected.into_iter().collect();

        // Ensure no pins are locked initially
        let mut initial_locked = HashSet::new();
        for node_any in &app.nodes {
            if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                for p in n.base.all_pins() {
                    if p.locked {
                        initial_locked.insert(p.id);
                    }
                }
            } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                for p in n.base.all_pins() {
                    if p.locked {
                        initial_locked.insert(p.id);
                    }
                }
            }
        }
        assert!(initial_locked.is_empty());

        // Lock the start pin
        app.set_pin_locked(a_out, true).expect("set_pin_locked");

        // Collect all locked pins after propagation
        let mut locked = HashSet::new();
        for node_any in &app.nodes {
            if let Some(n) = node_any.downcast_ref::<CraftNode>() {
                for p in n.base.all_pins() {
                    if p.locked {
                        locked.insert(p.id);
                    }
                }
            } else if let Some(n) = node_any.downcast_ref::<OrganizerNode>() {
                for p in n.base.all_pins() {
                    if p.locked {
                        locked.insert(p.id);
                    }
                }
            } else if let Some(n) = node_any.downcast_ref::<GroupNode>() {
                for p in n.base.all_pins() {
                    if p.locked {
                        locked.insert(p.id);
                    }
                }
            } else if let Some(n) = node_any.downcast_ref::<SinkNode>() {
                for p in n.base.ins.iter() {
                    if p.locked {
                        locked.insert(p.id);
                    }
                }
            }
        }

        assert_eq!(locked, connected_set);
    }
}
