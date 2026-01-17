use crate::production_app::ProductionApp;
use egui::text;
use serde::{Deserialize, Serialize};

/// Simple node representation for the node editor
#[derive(Clone, Debug)]
pub struct EditorNode {
    #[allow(dead_code)]
    pub id: u64,
    pub label: String,
    #[allow(dead_code)]
    pub node_type: String,

    // Pin metadata for icons, labels, rates and locked state
    pub input_names: Vec<Option<String>>,
    pub input_icons: Vec<Option<egui::TextureId>>,
    pub input_rates: Vec<Option<String>>,
    pub input_locked: Vec<bool>,
    pub output_names: Vec<Option<String>>,
    pub output_icons: Vec<Option<egui::TextureId>>,
    pub output_rates: Vec<Option<String>>,
    pub output_locked: Vec<bool>,

    // Building info for craft nodes
    pub building_count_str: String,
    pub building_name: String,
    pub same_clock_power_str: String,
    pub last_underclock_power_str: String,
    pub variable_power: bool,

    // Somersloop info
    pub num_somersloop_str: String,
    pub somersloop_mult: Option<crate::fractional_number::FractionalNumber>,
    pub somersloop_icon: Option<egui::TextureId>,

    // For group nodes: whether all contained craft nodes are built (None if not applicable)
    pub group_built: Option<bool>,

    // For sink nodes: total sink points expressed as decimal string, and fraction tooltip
    pub sink_points_str: String,
    pub sink_points_fraction_str: String,

    // Optional item type for merger/splitter nodes (shown centered in footer instead of building count / points)
    pub item_type: Option<String>,
    pub item_type_icon: Option<egui::TextureId>,
}

impl EditorNode {
    pub fn new(id: u64, label: impl Into<String>, node_type: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            node_type: node_type.into(),
            input_names: Vec::new(),
            input_icons: Vec::new(),
            input_rates: Vec::new(),
            input_locked: Vec::new(),
            output_names: Vec::new(),
            output_icons: Vec::new(),
            output_rates: Vec::new(),
            output_locked: Vec::new(),
            building_count_str: String::new(),
            building_name: String::new(),
            same_clock_power_str: String::new(),
            last_underclock_power_str: String::new(),
            variable_power: false,
            num_somersloop_str: String::new(),
            somersloop_mult: None,
            somersloop_icon: None,
            group_built: None,
            sink_points_str: String::new(),
            sink_points_fraction_str: String::new(),
            item_type: None,
            item_type_icon: None,
        }
    }

    pub fn with_pins(
        id: u64,
        label: impl Into<String>,
        node_type: impl Into<String>,
        input_names: Vec<Option<String>>,
        input_icons: Vec<Option<egui::TextureId>>,
        input_rates: Vec<Option<String>>,
        input_locked: Vec<bool>,
        output_names: Vec<Option<String>>,
        output_icons: Vec<Option<egui::TextureId>>,
        output_rates: Vec<Option<String>>,
        output_locked: Vec<bool>,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            node_type: node_type.into(),
            input_names,
            input_icons,
            input_rates,
            input_locked,
            output_names,
            output_icons,
            output_rates,
            output_locked,
            building_count_str: String::new(),
            building_name: String::new(),
            same_clock_power_str: String::new(),
            last_underclock_power_str: String::new(),
            variable_power: false,
            num_somersloop_str: String::new(),
            somersloop_mult: None,
            somersloop_icon: None,
            group_built: None,
            sink_points_str: String::new(),
            sink_points_fraction_str: String::new(),
            item_type: None,
            item_type_icon: None,
        }
    }
}

use crate::pin::PinDirection;
use std::collections::HashMap;

#[derive(Default, Debug)]
struct SnarlViewer {
    // Keep a clone of the current node being rendered to access pin metadata in show_input/show_output
    current_node: Option<EditorNode>,
    // Cursors advanced by show_input/show_output calls to get the pin index in order
    input_cursor: usize,
    output_cursor: usize,
    // Right edge anchor for output rows (per node) to avoid horizontal drift between rows
    output_anchor_right: Option<f32>,
    // Precomputed per-node output row dimensions (to align all outputs to the same right edge)
    output_row_width: Option<f32>,
    output_row_height: Option<f32>,

    // Temporary edit buffers for pin rate editing: key -> string
    edit_buffers: HashMap<String, String>,

    // Pending edits committed by the UI that TemplateApp should process after the Snarl widget is shown
    pending_pin_rate_edits: Vec<(u64, PinDirection, usize, String)>,
    // Pending somersloop edits: node_id -> string
    pending_node_somersloop_edits: Vec<(u64, String)>,

    // Pending group built edits: node_id -> bool
    pending_node_built_edits: Vec<(u64, bool)>,

    // Pending pin add/remove ops collected during rendering
    pending_pin_adds: Vec<(u64, crate::pin::PinDirection)>,
    pending_pin_removes: Vec<(u64, crate::pin::PinDirection, usize)>,

    // Map of item name -> TextureId supplied by the app so the viewer can resolve icons immediately
    icon_map: std::collections::HashMap<String, egui::TextureId>,

    // Whether to display same-clock or last-underclock in UI
    pub power_equal_clocks: bool,

    // Last reason a connection was rejected by the viewer (displayed as error_message by TemplateApp)
    pub rejected_connection_reason: Option<String>,
}

impl SnarlViewer {
    // Fixed inset before the footer '+' (used for both input and output placements)
    const FOOTER_ADD_INSET: f32 = 48.0;

    fn drain_pending_edits(&mut self) -> Vec<(u64, PinDirection, usize, String)> {
        std::mem::take(&mut self.pending_pin_rate_edits)
    }

    fn drain_pending_somersloop_edits(&mut self) -> Vec<(u64, String)> {
        std::mem::take(&mut self.pending_node_somersloop_edits)
    }

    fn drain_pending_built_edits(&mut self) -> Vec<(u64, bool)> {
        std::mem::take(&mut self.pending_node_built_edits)
    }

    fn drain_pending_pin_adds(&mut self) -> Vec<(u64, crate::pin::PinDirection)> {
        std::mem::take(&mut self.pending_pin_adds)
    }

    fn drain_pending_pin_removes(&mut self) -> Vec<(u64, crate::pin::PinDirection, usize)> {
        std::mem::take(&mut self.pending_pin_removes)
    }

    // Render a fractional number input similar to C++ RenderInputText.
    // Returns the response so caller can inspect focus/hover for tooltips.
    fn render_fractional_input(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        buf: &mut String,
        width: f32,
        disabled: bool,
    ) -> egui::Response {
        // Ensure buffer exists in edit_buffers
        self.edit_buffers
            .entry(key.to_owned())
            .or_insert_with(|| buf.clone());
        let buf_ref = self.edit_buffers.get_mut(key).unwrap();

        // Reserve a rectangle of exact size for the input
        let (rect, _alloc_response) = ui.allocate_exact_size(
            egui::Vec2::new(width, ui.spacing().interact_size.y),
            egui::Sense::click(),
        );

        // Active input: render TextEdit inside the reserved rect
        let text_edit = egui::TextEdit::singleline(buf_ref).desired_width(width);
        let response = ui
            .allocate_ui_at_rect(rect, |ui| ui.add_enabled(!disabled, text_edit))
            .response;

        // Focus highlight (blue)
        if response.has_focus() || response.gained_focus() {
            ui.painter().rect_filled(
                rect.expand(2.0),
                4.0,
                egui::Color32::from_rgba_unmultiplied(30, 70, 120, 60),
            );
        }

        // Tooltip showing parsed fraction and decimal value
        if response.hovered() {
            if let Ok(f) = crate::fractional_number::FractionalNumber::from_string(buf_ref) {
                let tip = format!("{} = {}", f.to_fraction_string(), f.to_float_string());
                return response.on_hover_text(tip);
            }
        }

        response
    }
}

impl SnarlViewer {
    /// Compute and cache per-node output row dimensions (width includes circle margin)
    fn compute_output_row_dims(&mut self, ui: &egui::Ui, node: &EditorNode, size: egui::Vec2) -> (f32, f32) {
        if self.output_row_width.is_none() {
            let mut max_label_w = 0.0f32;
            let mut max_lines = 1usize;
            for opt in node.output_names.iter() {
                let orig = opt.as_ref().map(|s| s.as_str()).unwrap_or("Out");
                let disp = orig.replace(' ', "\n");
                let mut label_w = 0.0f32;
                let line_count = disp.matches('\n').count() + 1;
                for line in disp.split('\n') {
                    let w = ui
                        .painter()
                        .layout_no_wrap(line.to_owned(), egui::FontId::default(), egui::Color32::WHITE)
                        .size()
                        .x;
                    if w > label_w { label_w = w; }
                }
                if label_w > max_label_w { max_label_w = label_w; }
                if line_count > max_lines { max_lines = line_count; }
            }
            let gap = 6.0;
            let circle_margin = size.x * 0.6;
            let computed_row_width = 88.0 + gap + size.x + gap + max_label_w + circle_margin;
            let line_height = ui.text_style_height(&egui::TextStyle::Body);
            let mut computed_row_height = (max_lines as f32) * line_height;
            if computed_row_height < size.y { computed_row_height = size.y; }
            self.output_row_width = Some(computed_row_width);
            self.output_row_height = Some(computed_row_height);
        }
        (self.output_row_width.unwrap(), self.output_row_height.unwrap())
    }

    // Synchronize merger/splitter pin types for a node: if any remote connections exist,
    // pick the first remote's item name and set all pins of that direction to it.
    // If there are no connections, clear the names.
    fn sync_merger_splitter(&mut self, snarl: &mut egui_snarl::Snarl<EditorNode>, node_id: egui_snarl::NodeId) {
        // Read-only pass: determine chosen item name (avoid simultaneous mutable/immutable borrows of snarl)
        if let Some(node_ref) = snarl.get_node(node_id) {
            match node_ref.node_type.as_str() {
                "merger" => {
                    let mut chosen: Option<String> = None;
                    for input_idx in 0..node_ref.input_names.len() {
                        let in_id = egui_snarl::InPinId { node: node_id, input: input_idx };
                        let in_pin = snarl.in_pin(in_id);
                        if let Some(remote) = in_pin.remotes.first() {
                            if let Some(remote_node) = snarl.get_node(remote.node) {
                                if let Some(Some(name)) = remote_node.output_names.get(remote.output) {
                                    chosen = Some(name.clone());
                                    break;
                                }
                            }
                        }
                    }
                    // Write pass: set node-level item_type and propagate to pins
                    if let Some(node_mut) = snarl.get_node_mut(node_id) {
                        if let Some(name) = chosen {
                            node_mut.item_type = Some(name.clone());

                            // Clear per-input names (inputs are sources feeding this merger)
                            for slot in node_mut.input_names.iter_mut() {
                                *slot = None;
                            }

                            // Propagate chosen name to the single output so downstream nodes see it
                            for slot in node_mut.output_names.iter_mut() {
                                *slot = Some(name.clone());
                            }

                            // Resolve icons for pins and footer immediately
                            for icon_slot in node_mut.output_icons.iter_mut() {
                                *icon_slot = self.icon_map.get(&name).copied();
                            }
                            node_mut.item_type_icon = self.icon_map.get(&name).copied();

                            // Debug
                            println!("SnarlViewer: set item_type '{}' on node {} ({})", name, node_mut.id, node_mut.node_type);
                            println!("SnarlViewer: propagated '{}' to outputs and set footer icon {:?}", name, node_mut.item_type_icon);
                        } else {
                            node_mut.item_type = None;
                            for slot in node_mut.input_names.iter_mut() {
                                *slot = None;
                            }
                            for slot in node_mut.output_names.iter_mut() {
                                *slot = None;
                            }
                            for icon_slot in node_mut.output_icons.iter_mut() {
                                *icon_slot = None;
                            }
                            node_mut.item_type_icon = None;

                            // Debug
                            println!("SnarlViewer: cleared item_type on node {} ({})", node_mut.id, node_mut.node_type);
                        }

                        // If this node is currently being rendered, update the cached clone so the footer shows changes immediately
                        if let Some(cur) = self.current_node.as_mut() {
                            if cur.id == node_mut.id {
                                *cur = node_mut.clone();
                            }
                        }
                    }
                }
                "sink" => {
                    // Sinks should NOT have a node-level item_type — pins carry their own types.
                    // Collect per-input chosen item names (read-only pass)
                    println!("SnarlViewer: checking sink node {} ({}) inputs (count={})", node_ref.id, node_ref.node_type, node_ref.input_names.len());
                    let mut chosen_per_input: Vec<Option<String>> = Vec::with_capacity(node_ref.input_names.len());
                    for input_idx in 0..node_ref.input_names.len() {
                        let in_id = egui_snarl::InPinId { node: node_id, input: input_idx };
                        let in_pin = snarl.in_pin(in_id);
                        if in_pin.remotes.is_empty() {
                            println!("  input[{}]: no remotes", input_idx);
                            chosen_per_input.push(None);
                        } else {
                            // pick first remote name (if any)
                            let mut found: Option<String> = None;
                            for r in in_pin.remotes.iter() {
                                if let Some(remote_node) = snarl.get_node(r.node) {
                                    let name_opt = remote_node.output_names.get(r.output).and_then(|o| o.clone());
                                    println!("  input[{}] remote -> node {:?} output {} name={:?}", input_idx, r.node, r.output, name_opt);
                                    if let Some(n) = name_opt {
                                        found = Some(n);
                                        break;
                                    }
                                } else {
                                    println!("  input[{}] remote -> node {:?} output {} (node not found)", input_idx, r.node, r.output);
                                }
                            }
                            chosen_per_input.push(found);
                        }
                    }

                    // Write pass: set per-pin names/icons on the sink node (do NOT set node-level item_type)
                    if let Some(node_mut) = snarl.get_node_mut(node_id) {
                        for (idx, chosen_opt) in chosen_per_input.into_iter().enumerate() {
                            if idx < node_mut.input_names.len() {
                                node_mut.input_names[idx] = chosen_opt.clone();
                                node_mut.input_icons[idx] = chosen_opt.as_ref().and_then(|n| self.icon_map.get(n).copied());
                                if let Some(n) = chosen_opt {
                                    println!("SnarlViewer: set input[{}] name='{}' on sink node {}", idx, n, node_mut.id);
                                } else {
                                    println!("SnarlViewer: cleared input[{}] name on sink node {}", idx, node_mut.id);
                                }
                            }
                        }

                        // Ensure node-level item_type is cleared
                        node_mut.item_type = None;
                        node_mut.item_type_icon = None;

                        println!("SnarlViewer: sink node {} ({}) retains per-pin types; node-level item_type cleared", node_mut.id, node_mut.node_type);

                        // Update cached current node if needed so UI reflects cleared state immediately
                        if let Some(cur) = self.current_node.as_mut() {
                            if cur.id == node_mut.id {
                                *cur = node_mut.clone();
                            }
                        }
                    }
                }
                "custom_splitter" | "game_splitter" => {
                    let mut chosen: Option<String> = None;
                    println!("SnarlViewer: examining splitter node {:?} inputs={:?} outputs={:?}", node_id, node_ref.input_names, node_ref.output_names);

                    // First try: check inputs' remotes (source -> splitter input), prefer remote's output name
                    for input_idx in 0..node_ref.input_names.len() {
                        let in_id = egui_snarl::InPinId { node: node_id, input: input_idx };
                        let in_pin = snarl.in_pin(in_id);
                        if let Some(remote) = in_pin.remotes.first() {
                            if let Some(remote_node) = snarl.get_node(remote.node) {
                                if let Some(Some(name)) = remote_node.output_names.get(remote.output) {
                                    chosen = Some(name.clone());
                                    println!("SnarlViewer: splitter candidate from input[{}] remote node {:?} output {} = {:?}", input_idx, remote.node, remote.output, name);
                                    break;
                                } else if let Some(name) = remote_node.output_names.iter().find_map(|o| o.clone()) {
                                    chosen = Some(name.clone());
                                    println!("SnarlViewer: splitter fallback from input[{}] remote node {:?} any output = {:?}", input_idx, remote.node, name);
                                    break;
                                } else {
                                    println!("SnarlViewer: splitter input[{}] remote node {:?} had no output names", input_idx, remote.node);
                                }
                            }
                        } else {
                            println!("SnarlViewer: input[{}] has no remotes", input_idx);
                        }
                    }

                    // Fallback: inspect outputs' remotes (downstream nodes)
                    if chosen.is_none() {
                        for output_idx in 0..node_ref.output_names.len() {
                            let out_id = egui_snarl::OutPinId { node: node_id, output: output_idx };
                            let out_pin = snarl.out_pin(out_id);
                            if let Some(remote) = out_pin.remotes.first() {
                                if let Some(remote_node) = snarl.get_node(remote.node) {
                                    println!("SnarlViewer: splitter remote node {:?} input_names={:?} output_names={:?}", remote.node, remote_node.input_names, remote_node.output_names);
                                    // Prefer the remote node's input pin name (the splitter feeds that input),
                                    // but fall back to any input name, then any output name on remote node.
                                    let mut found_name: Option<String> = None;
                                    if let Some(Some(name)) = remote_node.input_names.get(remote.input) {
                                        found_name = Some(name.clone());
                                        println!("SnarlViewer: splitter candidate from remote node {:?} input {} = {:?}", remote.node, remote.input, name);
                                    } else if let Some(name) = remote_node.input_names.iter().find_map(|o| o.clone()) {
                                        // fall back to any input name on remote node
                                        found_name = Some(name.clone());
                                        println!("SnarlViewer: splitter fallback from remote node {:?} any input = {:?}", remote.node, name);
                                    } else if let Some(name) = remote_node.output_names.iter().find_map(|o| o.clone()) {
                                        // last resort: pick any output name on remote node
                                        found_name = Some(name.clone());
                                        println!("SnarlViewer: splitter fallback from remote node {:?} any output = {:?}", remote.node, name);
                                    } else {
                                        println!("SnarlViewer: splitter remote node {:?} had no input/output names", remote.node);
                                    }
                                    if let Some(name) = found_name {
                                        chosen = Some(name);
                                        break;
                                    }
                                }
                            } else {
                                println!("SnarlViewer: output[{}] has no remotes", output_idx);
                            }
                        }
                    }

                    if chosen.is_none() {
                        println!("SnarlViewer: no chosen name for splitter node {:?} after inspection", node_id);
                    }

                    if let Some(node_mut) = snarl.get_node_mut(node_id) {
                        if let Some(name) = chosen {
                            node_mut.item_type = Some(name.clone());

                            // Propagate chosen name to both inputs and outputs so downstream nodes see it
                            for slot in node_mut.input_names.iter_mut() {
                                *slot = Some(name.clone());
                            }
                            for slot in node_mut.output_names.iter_mut() {
                                *slot = Some(name.clone());
                            }

                            // Resolve icons for all pins and footer immediately
                            for icon_slot in node_mut.input_icons.iter_mut() {
                                *icon_slot = self.icon_map.get(&name).copied();
                            }
                            for icon_slot in node_mut.output_icons.iter_mut() {
                                *icon_slot = self.icon_map.get(&name).copied();
                            }
                            node_mut.item_type_icon = self.icon_map.get(&name).copied();

                            // Debug
                            println!("SnarlViewer: set item_type '{}' on node {} ({})", name, node_mut.id, node_mut.node_type);
                            println!("SnarlViewer: propagated '{}' to inputs/outputs and set footer icon {:?}", name, node_mut.item_type_icon);
                        } else {
                            node_mut.item_type = None;

                            // Clear per-pin names and icons for both sides
                            for slot in node_mut.input_names.iter_mut() {
                                *slot = None;
                            }
                            for slot in node_mut.output_names.iter_mut() {
                                *slot = None;
                            }
                            for icon_slot in node_mut.input_icons.iter_mut() {
                                *icon_slot = None;
                            }
                            for icon_slot in node_mut.output_icons.iter_mut() {
                                *icon_slot = None;
                            }

                            node_mut.item_type_icon = None;

                            // Debug
                            println!("SnarlViewer: cleared item_type on node {} ({})", node_mut.id, node_mut.node_type);
                        }

                        // If this node is currently being rendered, update the cached clone so the footer shows changes immediately
                        if let Some(cur) = self.current_node.as_mut() {
                            if cur.id == node_mut.id {
                                *cur = node_mut.clone();
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Provide a lightweight name -> TextureId map so the viewer can resolve icons during connect/sync
    fn set_icon_map(&mut self, map: std::collections::HashMap<String, egui::TextureId>) {
        self.icon_map = map;
    }

    // Helper to render a '+' in the footer aligned to a column depending on direction
    // Input -> column 1 (left) | Output -> column 3 (right)
    fn render_footer_add_button_middle(&mut self, ui: &mut egui::Ui, node: &EditorNode, dir: PinDirection) {
        egui::Grid::new(format!("footer_add_col:{}:{}", node.id, match dir {
            PinDirection::Input => "in",
            PinDirection::Output => "out",
        }))
        .num_columns(3)
        .spacing([8.0, 8.0])
        .min_col_width(ui.available_width() / 3.0)
        .show(ui, |ui| {
            match dir {
                PinDirection::Input => {
                    // Place in first column with left inset
                    ui.horizontal(|ui| {
                        ui.add_space(Self::FOOTER_ADD_INSET);
                        if ui.add(egui::Button::new("+").corner_radius(egui::CornerRadius::same(0)).small()).clicked() {
                            self.pending_pin_adds.push((node.id, dir));
                        }
                    });
                    ui.horizontal(|ui| {});
                    ui.horizontal(|ui| {});
                }
                PinDirection::Output => {
                    ui.horizontal(|ui| {});
                    ui.horizontal(|ui| {});
                    // Place in third column with right inset using RTL layout
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(Self::FOOTER_ADD_INSET);
                        if ui.add(egui::Button::new("+").corner_radius(egui::CornerRadius::same(0)).small()).clicked() {
                            self.pending_pin_adds.push((node.id, dir));
                        }
                    });
                }
            }
            ui.end_row();
        });
    }
}

impl egui_snarl::ui::SnarlViewer<EditorNode> for SnarlViewer {
    fn title(&mut self, node: &EditorNode) -> String {
        node.label.clone()
    }

    fn show_header(
        &mut self,
        node_id: egui_snarl::NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) {
        // Default header shows title; override to add a checkbox for group nodes
        if let Some(node_info) = snarl.get_node_info(node_id) {
            // Access the EditorNode stored in the Snarl
            let node = &node_info.value;
            ui.horizontal(|ui| {
                ui.label(node.label.clone());
                // Right-aligned controls
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(is_built) = node.group_built {
                        let mut checked = is_built;
                        // Render compact checkbox without label
                        let resp = ui.add(egui::widgets::Checkbox::new(&mut checked, ""));
                        if resp.changed() {
                            // Queue the change for processing after rendering
                            self.pending_node_built_edits.push((node.id, checked));
                        }
                    }
                });
            });
        }
    }

    fn inputs(&mut self, node: &EditorNode) -> usize {
        self.current_node = Some(node.clone());
        self.input_cursor = 0;
        node.input_names.len()
    }

    fn outputs(&mut self, node: &EditorNode) -> usize {
        self.current_node = Some(node.clone());
        self.output_cursor = 0;
        self.output_anchor_right = None;
        // Reset per-node row size cache
        self.output_row_width = None;
        self.output_row_height = None;
        node.output_names.len()
    }

    fn show_input(
        &mut self,
        _pin: &egui_snarl::InPin,
        ui: &mut egui::Ui,
        _snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        let size = egui::Vec2::splat(ui.spacing().interact_size.y * 1.2);
        if let Some(node_ref) = &self.current_node {
            let node = node_ref.clone();
            let idx = self.input_cursor;
            self.input_cursor += 1;
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                // 'x' remove button for mergers/sinks (near outer edge)
                if node.node_type == "merger" || node.node_type == "sink" {
                    let can_remove = node.input_names.len() > 1;
                    let btn = egui::Button::new("x").corner_radius(egui::CornerRadius::same(0)).small();
                    let resp = ui.add_enabled(can_remove, btn);
                    if resp.clicked() {
                        self.pending_pin_removes.push((node.id, PinDirection::Input, idx));
                    }
                }

                // Rate first (near outer edge for inputs)
                if let Some(Some(rate)) = node.input_rates.get(idx) {
                    let key = format!("pin:{}:in:{}", node.id, idx);
                    // Use helper to render small input with highlight
                    // Use a conservative fixed width similar to C++ "0000.000"
                    let desired_width = 88.0;
                    let mut tmp = rate.clone();
                    let disabled = node.input_locked.get(idx).copied().unwrap_or(false);
                    let response =
                        self.render_fractional_input(ui, &key, &mut tmp, desired_width, disabled);
                    if response.lost_focus() && response.changed() {
                        if let Some(buf) = self.edit_buffers.get(&key) {
                            self.pending_pin_rate_edits.push((
                                node.id,
                                PinDirection::Input,
                                idx,
                                buf.clone(),
                            ));
                        }
                    }
                }

                // Icon + Label handling
                if node.node_type == "sink" {
                    // For sinks, show an icon+label only if the pin has an item assigned; otherwise show nothing
                    if let Some(Some(name)) = node.input_names.get(idx) {
                        if let Some(Some(tex)) = node.input_icons.get(idx) {
                            ui.image((*tex, size));
                            ui.add_space(6.0);
                        }
                        let disp = name.replace(' ', "\n");
                        ui.label(disp);
                    } else {
                        // sink: intentionally show nothing when no item set
                    }
                } else {
                    // Default behavior for non-sink nodes
                    // For merger/splitter nodes we intentionally hide per-pin icons and labels
                    if node.node_type != "merger" && node.node_type != "custom_splitter" && node.node_type != "game_splitter" {
                        if let Some(Some(tex)) = node.input_icons.get(idx) {
                            // Use the image widget to draw the texture (lets egui handle clipping/alpha)
                            ui.image((*tex, size));
                        }

                        // Label closest to center (display names with spaces -> newlines to match C++)
                        if let Some(Some(name)) = node.input_names.get(idx) {
                            let disp = name.replace(' ', "\n");
                            ui.label(disp);
                        } else {
                            ui.label("In");
                        }
                    }
                }


            });
        }
        egui_snarl::ui::PinInfo::circle()
    }

    fn show_output(
        &mut self,
        _pin: &egui_snarl::OutPin,
        ui: &mut egui::Ui,
        _snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        let size = egui::Vec2::splat(ui.spacing().interact_size.y * 1.2);
        if let Some(node_ref) = &self.current_node {
            let node = node_ref.clone();
            let idx = self.output_cursor;
            self.output_cursor += 1;
            // Capture rects for debug logging
            let mut rate_rect: Option<egui::Rect> = None;
            let mut icon_rect: Option<egui::Rect> = None;
            let mut label_rect: Option<egui::Rect> = None;

            // Use cached per-node row dimensions
            let (row_width, row_height) = self.compute_output_row_dims(ui, &node, size);

            // Advance layout but render into an anchored rect so rows don't drift
            let (slot_rect, _slot_resp) = ui.allocate_exact_size(egui::vec2(row_width, row_height), egui::Sense::hover());
            let anchor_right = *self.output_anchor_right.get_or_insert(slot_rect.right());
            // Leave a right margin for the pin circle so fields don't overlap it
            let circle_margin = size.x * 0.6;
            let anchored_rect = egui::Rect::from_min_max(
                egui::pos2(anchor_right - row_width, slot_rect.top()),
                egui::pos2(anchor_right - circle_margin, slot_rect.bottom()),
            );

            let _row = ui.scope_builder(egui::UiBuilder::new().max_rect(anchored_rect), |ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // 'x' remove button for custom/game splitters (near outer edge)
                    if node.node_type == "custom_splitter" || node.node_type == "game_splitter" {
                        let can_remove = node.output_names.len() > 1;
                        let btn = egui::Button::new("x").corner_radius(egui::CornerRadius::same(0)).small();
                        let resp = ui.add_enabled(can_remove, btn);
                        if resp.clicked() {
                            self.pending_pin_removes.push((node.id, PinDirection::Output, idx));
                        }
                    }

                    // Rate first (near outer edge for outputs)
                    if let Some(Some(rate)) = node.output_rates.get(idx) {
                        let key = format!("pin:{}:out:{}", node.id, idx);
                        // Use a conservative fixed width similar to C++ "0000.000"
                        let desired_width = 88.0;
                        let mut tmp = rate.clone();
                        let disabled = node.output_locked.get(idx).copied().unwrap_or(false);
                        let response = self.render_fractional_input(
                            ui,
                            &key,
                            &mut tmp,
                            desired_width,
                            disabled,
                        );
                        rate_rect = Some(response.rect);
                        if response.lost_focus() && response.changed() {
                            if let Some(buf) = self.edit_buffers.get(&key) {
                                self.pending_pin_rate_edits.push((
                                    node.id,
                                    PinDirection::Output,
                                    idx,
                                    buf.clone(),
                                ));
                            }
                        }
                    }

                    // For merger/splitter nodes we intentionally hide per-pin icons and labels
                    if node.node_type != "merger" && node.node_type != "custom_splitter" && node.node_type != "game_splitter" {
                        // Icon next (inward)
                        if let Some(Some(tex)) = node.output_icons.get(idx) {
                            // Use widget-based image drawing
                            let resp = ui.image((*tex, size));
                            icon_rect = Some(resp.rect);
                        }

                        // Label closest to center (display names with spaces -> newlines to match C++)
                        if let Some(Some(name)) = node.output_names.get(idx) {
                            let disp = name.replace(' ', "\n");
                            let resp = ui.label(disp);
                            label_rect = Some(resp.rect);
                        } else {
                            let resp = ui.label("Out");
                            label_rect = Some(resp.rect);
                        }
                    }


                });
            });
        } 
        egui_snarl::ui::PinInfo::circle()
    }

    fn has_footer(&mut self, node: &EditorNode) -> bool {
        // Show footer if node has building, power info (craft nodes) or sink points
        // Also show footer for organizer and sink nodes so we can render add buttons
        !node.building_name.is_empty()
            || !node.same_clock_power_str.is_empty()
            || !node.last_underclock_power_str.is_empty()
            || !node.sink_points_str.is_empty()
            || node.node_type == "custom_splitter"
            || node.node_type == "game_splitter"
            || node.node_type == "merger"
            || node.node_type == "sink"
    }

    fn show_footer(
        &mut self,
        _node_id: egui_snarl::NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        _snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) {
        if let Some(node_ref) = &self.current_node {
            let node = node_ref.clone();
            ui.vertical(|ui| {
                // Power + building row matching C++ node bottom: single power input (choice depends on power mode), MW suffix, then pushed node rate + building
                let power_value = if self.power_equal_clocks {
                    node.same_clock_power_str.clone()
                } else {
                    node.last_underclock_power_str.clone()
                };
                if !power_value.is_empty() || !node.building_name.is_empty(){
                    ui.horizontal(|ui| {
                        // Power sizes (number field + spacing + MW label)
                        let power_field_width = ui
                            .painter()
                            .layout_no_wrap(
                                "000000.00".to_owned(),
                                egui::FontId::default(),
                                egui::Color32::WHITE,
                            )
                            .size()
                            .x;
                        let power_label_text = if node.variable_power { "~MW" } else { "MW" };

                        // Building sizes (count field + spacing + label)
                        let center_field_width = ui.spacing().interact_size.y;

                        egui::Grid::new(format!("footer_grid:{}", node.id))
                            .num_columns(3)
                            .spacing([8.0, 8.0])
                            .min_col_width(ui.available_width() / 3.0)
                            .show(ui, |ui| {
                                if !power_value.is_empty() {
                                    ui.horizontal(|ui| {
                                        let key = format!("node:{}:power", node.id);
                                        let mut tmp = power_value.clone();
                                        let locked = true; // Power is always locked in this UI
                                        let input_resp = self.render_fractional_input(
                                            ui,
                                            &key,
                                            &mut tmp,
                                            power_field_width,
                                            locked,
                                        );
                                        // Render combined label ("~MW" when variable) similar to C++
                                        let label_resp = ui.label(power_label_text);
                                        if node.variable_power
                                            && (label_resp.hovered() || input_resp.hovered())
                                        {
                                            label_resp.on_hover_text("Average power");
                                        }
                                    });
                                } else {
                                    ui.horizontal(|ui| {});
                                }

                                                // Column 2: building (center column occupies the center of the footer), content left-to-right
                                if let Some(name) = node.item_type.as_ref() {
                                    ui.horizontal(|ui| {
                                        // Item icon if available
                                        if let Some(tex) = node.item_type_icon {
                                            let icon_size = egui::vec2(
                                                ui.spacing().interact_size.y,
                                                ui.spacing().interact_size.y,
                                            );
                                            ui.image((tex, icon_size));
                                        }
                                        ui.label(name);
                                    });
                                } else if !node.building_name.is_empty() {
                                    ui.horizontal(|ui| {
                                        if !node.building_count_str.is_empty() {
                                            let key = format!("building:{}", node.id);
                                            let mut tmp = node.building_count_str.clone();
                                            let _r = self.render_fractional_input(
                                                ui,
                                                &key,
                                                &mut tmp,
                                                center_field_width,
                                                false,
                                            );
                                        }
                                        if !node.building_name.is_empty() {
                                            ui.label(&node.building_name);
                                        }
                                    });
                                } else {
                                    ui.horizontal(|ui| {});
                                }

                                // Somersloop field (show only if building supports it and not a power generator)
                                if !node.num_somersloop_str.is_empty()
                                    || node.somersloop_mult.map_or(false, |m| m.numerator() != 0)
                                {
                                    if node.somersloop_mult.map_or(false, |m| m.numerator() != 0)
                                        && !node.last_underclock_power_str.starts_with("-")
                                    {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.horizontal(|ui| {
                                                    if let Some(tex) = node.somersloop_icon {
                                                        // Use the standard interact height from the UI spacing for the icon size.
                                                        let icon_size = egui::vec2(
                                                            ui.spacing().interact_size.y,
                                                            ui.spacing().interact_size.y,
                                                        );
                                                        let (rect, resp) = ui.allocate_exact_size(
                                                            icon_size,
                                                            egui::Sense::hover(),
                                                        );
                                                        ui.painter().image(
                                                            tex,
                                                            rect,
                                                            egui::Rect::from_min_max(
                                                                egui::pos2(0.0, 0.0),
                                                                egui::pos2(1.0, 1.0),
                                                            ),
                                                            egui::Color32::WHITE,
                                                        );
                                                        if resp.hovered() {
                                                            resp.on_hover_text(
                                                                "Alien Production Amplification",
                                                            );
                                                        }
                                                    }

                                                    let somersloop_width = ui
                                                        .painter()
                                                        .layout_no_wrap(
                                                            "4".to_owned(),
                                                            egui::FontId::default(),
                                                            egui::Color32::WHITE,
                                                        )
                                                        .size()
                                                        .x
                                                        + 8.0;
                                                    let key =
                                                        format!("node:{}:somersloop", node.id);
                                                    let mut tmp = node.num_somersloop_str.clone();
                                                    let is_locked = node
                                                        .input_locked
                                                        .get(0)
                                                        .copied()
                                                        .unwrap_or(false)
                                                        || node
                                                            .output_locked
                                                            .get(0)
                                                            .copied()
                                                            .unwrap_or(false);
                                                    let resp = self.render_fractional_input(
                                                        ui,
                                                        &key,
                                                        &mut tmp,
                                                        somersloop_width,
                                                        is_locked,
                                                    );

                                                    if resp.lost_focus() && resp.changed() {
                                                        // Commit somersloop edit from the internal buffer (render_fractional_input stores it)
                                                        if let Some(buf) =
                                                            self.edit_buffers.get(&key)
                                                        {
                                                            self.pending_node_somersloop_edits
                                                                .push((node.id, buf.clone()));
                                                        }
                                                    }
                                                });
                                            },
                                        );
                                    }
                                } else {
                                    // 3rd column reserved
                                    ui.horizontal(|ui| {});
                                }

                                // Add '+' row inside the footer grid so the button aligns under the number fields
                                // Column alignment: left column -> inputs (merger/sink), right column -> outputs (splitters)
                                if node.node_type == "merger" {
                                    self.render_footer_add_button_middle(ui, &node, PinDirection::Input);
                                } else if node.node_type == "custom_splitter" || node.node_type == "game_splitter" {
                                    self.render_footer_add_button_middle(ui, &node, PinDirection::Output);
                                }
                                ui.end_row();
                            });
                    });
                }
                else if !node.sink_points_str.is_empty(){
                    // Sink node: show a '+' in the left column above the points row, then show points and tooltip with fraction
                    // Render '+' above points in middle column, using shared helper for consistent alignment
                    self.render_footer_add_button_middle(ui, &node, PinDirection::Input);

                    // Points row aligned to the same columns so the '+' sits above the number field
                    egui::Grid::new(format!("footer_sink_points:{}", node.id))
                        .num_columns(3)
                        .spacing([8.0, 8.0])
                        .min_col_width(ui.available_width() / 3.0)
                        .show(ui, |ui| {
                            // Center column: show item_type if present, otherwise show points
                            if let Some(name) = node.item_type.as_ref() {
                                ui.horizontal(|ui| {
                                    if let Some(tex) = node.item_type_icon {
                                        let icon_size = egui::vec2(
                                            ui.spacing().interact_size.y,
                                            ui.spacing().interact_size.y,
                                        );
                                        ui.image((tex, icon_size));
                                        ui.add_space(6.0);
                                    }
                                    ui.label(name);
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    let mut points_str = node.sink_points_str.clone();
                                    // magic number
                                    let text_edit = egui::TextEdit::singleline(&mut points_str).desired_width(44.0);
                                    let response = ui.add_enabled(false, text_edit);
                                    if response.hovered() {
                                        response.on_hover_text(&node.sink_points_fraction_str);
                                    }
                                    ui.label("points");
                                });
                            }
                            ui.horizontal(|ui| {});
                            ui.horizontal(|ui| {});
                            ui.end_row();
                        });
                }

                // If footer didn't contain other content (power/building/sink), show a fallback centered area
                if node.building_name.is_empty()
                    && node.same_clock_power_str.is_empty()
                    && node.last_underclock_power_str.is_empty()
                    && node.sink_points_str.is_empty()
                {
                    if node.node_type == "merger" || node.node_type == "custom_splitter" || node.node_type == "game_splitter" {
                        // Render a three-column grid so we can center the item_type if present and still show + in the side column
                        egui::Grid::new(format!("footer_fallback_grid:{}", node.id))
                            .num_columns(3)
                            .spacing([8.0, 8.0])
                            .min_col_width(ui.available_width() / 3.0)
                            .show(ui, |ui| {
                                // Left column (input + for mergers)
                                if node.node_type == "merger" {
                                    ui.horizontal(|ui| {
                                        ui.add_space(Self::FOOTER_ADD_INSET);
                                        if ui.add(egui::Button::new("+").corner_radius(egui::CornerRadius::same(0)).small()).clicked() {
                                            self.pending_pin_adds.push((node.id, PinDirection::Input));
                                        }
                                    });
                                } else {
                                    ui.horizontal(|ui| {});
                                }

                                // Center column: show item_type (icon + label) if present
                                ui.horizontal(|ui| {
                                    if let Some(name) = node.item_type.as_ref() {
                                        if let Some(tex) = node.item_type_icon {
                                            let icon_size = egui::vec2(
                                                ui.spacing().interact_size.y,
                                                ui.spacing().interact_size.y,
                                            );
                                            ui.image((tex, icon_size));
                                            ui.add_space(6.0);
                                        }
                                        ui.label(name);
                                    } else {
                                        ui.horizontal(|ui| {});
                                    }
                                });

                                // Right column (output + for splitters)
                                if node.node_type != "merger" {
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.add_space(Self::FOOTER_ADD_INSET);
                                        if ui.add(egui::Button::new("+").corner_radius(egui::CornerRadius::same(0)).small()).clicked() {
                                            self.pending_pin_adds.push((node.id, PinDirection::Output));
                                        }
                                    });
                                } else {
                                    ui.horizontal(|ui| {});
                                }

                                ui.end_row();
                            });
                    }
                }

            });
        }
    }

    fn connect(&mut self, from: &egui_snarl::OutPin, to: &egui_snarl::InPin, snarl: &mut egui_snarl::Snarl<EditorNode>) {
        // Lookup the output and input names (if any) from the corresponding nodes
        let out_name = snarl.get_node(from.id.node)
            .and_then(|n| n.output_names.get(from.id.output))
            .and_then(|opt| opt.clone());
        let in_name = snarl.get_node(to.id.node)
            .and_then(|n| n.input_names.get(to.id.input))
            .and_then(|opt| opt.clone());

        // Debug: log the attempted connection and the current item types on both pins
        println!("SnarlViewer: connect attempt from {:?} (out_name={:?}) -> {:?} (in_name={:?})", from.id, out_name, to.id, in_name);

        // If both pins have an associated item name and they differ, reject the connection
        if let (Some(outn), Some(inn)) = (out_name, in_name) {
            if outn != inn {
                let msg = format!("Cannot connect different item types: '{}' -> '{}'", outn, inn);
                println!("{}", msg);
                self.rejected_connection_reason = Some(msg);
                return;
            }
        }

        // Disconnect any existing connections on either pin (except the same pair) so the new
        // connection replaces previous ones (enforce max 1 connection per pin by replacement).
        let out_remotes = snarl.out_pin(from.id).remotes.clone();
        let in_remotes = snarl.in_pin(to.id).remotes.clone();

        // Track affected node ids so we can re-sync their pin types after changes
        let mut affected_nodes = std::collections::HashSet::new();

        let mut out_replacements = 0usize;
        for r in out_remotes.clone() {
            if r != to.id {
                out_replacements += 1;
                affected_nodes.insert(r.node);
                let _ = snarl.disconnect(from.id, r);
            }
        }
        let mut in_replacements = 0usize;
        for r in in_remotes.clone() {
            if r != from.id {
                in_replacements += 1;
                affected_nodes.insert(r.node);
                let _ = snarl.disconnect(r, to.id);
            }
        }

        if out_replacements + in_replacements > 0 {
            println!("Replaced {} existing connection(s)", out_replacements + in_replacements);
        }

        // Finally perform the new connection
        let _ = snarl.connect(from.id, to.id);

        // Sync pin-type assignment/removal for the affected nodes and the endpoints
        affected_nodes.insert(from.id.node);
        affected_nodes.insert(to.id.node);
        for nid in affected_nodes {
            self.sync_merger_splitter(snarl, nid);
        }
    }
}

/// The main Ficsit Companion application
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct TemplateApp {
    #[serde(skip)]
    production_app: ProductionApp,

    #[serde(skip)]
    game_data: crate::game_data::GameData,

    #[serde(skip)]
    snarl: egui_snarl::Snarl<EditorNode>,

    #[serde(skip)]
    snarl_viewer: SnarlViewer,

    #[serde(skip)]
    snarl_style: egui_snarl::ui::SnarlStyle,

    // UI State - Left Panel
    left_panel_collapsed: bool,
    power_equal_clocks: bool,
    save_name: String,
    file_suggestions: Vec<(String, bool)>,
    show_controls_popup: bool,

    // Icon cache for items (store TextureHandle so images stay alive)
    #[serde(skip)]
    item_icon_cache: std::collections::HashMap<String, egui::TextureHandle>,

    // UI State - Dialogs
    #[serde(skip)]
    show_recipe_selector: bool,

    #[serde(skip)]
    selected_recipe: Option<String>,

    #[serde(skip)]
    recipe_search: String,

    // Error handling
    #[serde(skip)]
    error_message: String,

    #[serde(skip)]
    error_time: f32,

    // Context menu state
    #[serde(skip)]
    context_menu_recipe_filter: String,

    #[serde(skip)]
    show_add_node_popup: bool,

    #[serde(skip)]
    add_node_popup_pos: egui::Pos2,

    // Track which recipes have their checkboxes checked (invisible)
    #[serde(skip)]
    recipe_checkbox_state: std::collections::HashMap<String, bool>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let mut game_data = crate::game_data::GameData::new();

        // Load game data from satisfactory.json
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Ok(json_data) = std::fs::read_to_string("assets/satisfactory.json") {
                match game_data.load_from_json(&json_data) {
                    Ok(_) => {
                        println!(
                            "✓ Loaded {} recipes from game data",
                            game_data.recipes.len()
                        );
                    }
                    Err(e) => {
                        eprintln!("✗ Failed to load game data: {}", e);
                    }
                }
            } else {
                eprintln!("✗ Warning: Could not read assets/satisfactory.json");
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            // For web, we'll need to load this differently (fetch API, etc.)
            eprintln!("Web platform: game data loading not yet implemented");
        }

        let mut snarl_style = egui_snarl::ui::SnarlStyle::new();
        snarl_style.collapsible = Some(false);

        let mut app = Self {
            production_app: ProductionApp::new(),
            game_data,
            snarl: egui_snarl::Snarl::new(),
            snarl_viewer: SnarlViewer::default(),
            snarl_style,
            left_panel_collapsed: false,
            power_equal_clocks: false,
            save_name: String::new(),
            file_suggestions: Vec::new(),
            show_controls_popup: false,
            show_recipe_selector: false,
            selected_recipe: None,
            recipe_search: String::new(),
            error_message: String::new(),
            error_time: 0.0,
            context_menu_recipe_filter: String::new(),
            show_add_node_popup: false,
            add_node_popup_pos: egui::pos2(0.0, 0.0),
            item_icon_cache: std::collections::HashMap::new(),
            recipe_checkbox_state: std::collections::HashMap::new(),
        };

        // Don't add demo nodes if game data loaded successfully
        if app.game_data.recipes.is_empty() {
            let n = app.build_editor_node(1, "Craft Node A", "craft");
            app.snarl.insert_node(egui::pos2(0.0, 0.0), n);
            let n = app.build_editor_node(2, "Sink Node B", "sink");
            app.snarl.insert_node(egui::pos2(300.0, 0.0), n);
        }

        app
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any).
        let mut app: TemplateApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        // Load textures for game items using the egui context
        app.load_item_textures(cc);

        // Ensure viewer respects current power mode
        app.snarl_viewer.power_equal_clocks = app.power_equal_clocks;

        app
    }

    /// Load item icon textures into `item_icon_cache` using `cc.egui_ctx`.
    fn load_item_textures(&mut self, cc: &eframe::CreationContext<'_>) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use egui::ColorImage;
            use image::ImageReader;

            // Try loading a special somersloop icon used in node footers (optional)
            let somersloop_path = "assets/icons/Wat_1_64.png";
            if !self.item_icon_cache.contains_key("Somersloop") {
                match ImageReader::open(somersloop_path) {
                    Ok(reader) => match reader.decode() {
                        Ok(img) => {
                            let img = img.to_rgba8();
                            let size = [img.width() as usize, img.height() as usize];
                            let pixels = img.into_raw();
                            let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
                            let texture = cc.egui_ctx.load_texture(
                                "Somersloop".to_owned(),
                                color_image,
                                egui::TextureOptions::default(),
                            );
                            self.item_icon_cache
                                .insert("Somersloop".to_owned(), texture);
                        }
                        Err(e) => {
                            eprintln!(
                                "Failed to decode somersloop icon {}: {}",
                                somersloop_path, e
                            );
                        }
                    },
                    Err(_) => {
                        // not fatal; icon simply not present
                    }
                }
            }

            for (name, item_rc) in self.game_data.items.iter() {
                // Avoid duplicate loads
                if self.item_icon_cache.contains_key(name) {
                    continue;
                }
                let path = if item_rc.icon_path.starts_with("icons/") {
                    format!("assets/{}", item_rc.icon_path)
                } else {
                    format!("assets/icons/{}", item_rc.icon_path)
                };
                match ImageReader::open(&path) {
                    Ok(reader) => match reader.decode() {
                        Ok(img) => {
                            let img = img.to_rgba8();
                            let size = [img.width() as usize, img.height() as usize];
                            let pixels = img.into_raw();
                            let color_image = ColorImage::from_rgba_unmultiplied(size, &pixels);
                            let texture = cc.egui_ctx.load_texture(
                                name.clone(),
                                color_image,
                                egui::TextureOptions::default(),
                            );
                            // Keep the returned TextureHandle alive in the cache
                            self.item_icon_cache.insert(name.clone(), texture);
                        }
                        Err(e) => {
                            eprintln!("Failed to decode icon {}: {}", path, e);
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to open icon {}: {}", path, e);
                    }
                }
            }
            println!(
                "Loaded {} item icons into cache",
                self.item_icon_cache.len()
            );
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Web loading requires fetching assets; skip for now
            eprintln!("Web: item texture loading not implemented");
        }
    }

    /// Build an EditorNode from production model (fill pin names and icons)
    fn build_editor_node(
        &self,
        node_id: u64,
        label: impl Into<String>,
        node_type: impl Into<String>,
    ) -> EditorNode {
        let (input_names, output_names) = self
            .production_app
            .get_node_pin_item_names(node_id)
            .unwrap_or((Vec::new(), Vec::new()));

        // Fetch locked flags from production model
        let (input_locked_flags, output_locked_flags) = self
            .production_app
            .get_node_pin_locked_flags(node_id)
            .unwrap_or((Vec::new(), Vec::new()));

        // Map icons
        // (no debug prints)

        let input_icons: Vec<Option<egui::TextureId>> = input_names
            .iter()
            .map(|opt_name| {
                opt_name
                    .as_ref()
                    .and_then(|n| self.item_icon_cache.get(n).map(|h| h.id()))
            })
            .collect();

        let output_icons: Vec<Option<egui::TextureId>> = output_names
            .iter()
            .map(|opt_name| {
                opt_name
                    .as_ref()
                    .and_then(|n| self.item_icon_cache.get(n).map(|h| h.id()))
            })
            .collect();

        // Fetch rates from production model so UI can display them
        let (input_rates, output_rates) = self
            .production_app
            .get_node_pin_rates(node_id)
            .unwrap_or((Vec::new(), Vec::new()));

        // Fetch building info from production model
        let (building_count_str, building_name) = self
            .production_app
            .get_node_building_info(node_id)
            .unwrap_or((String::new(), String::new()));

        // Fetch power info from production model
        let (same_clock_power_str, last_underclock_power_str, variable_power) = self
            .production_app
            .get_node_power_info(node_id)
            .unwrap_or((String::new(), String::new(), false));

        // Fetch somersloop info from production model (num and multiplier if available)
        let (num_somersloop_str, somersloop_mult) = self
            .production_app
            .get_node_somersloop_info(node_id)
            .unwrap_or((String::new(), None));

        let mut editor_node = EditorNode::with_pins(
            node_id,
            label,
            node_type,
            input_names,
            input_icons,
            input_rates,
            input_locked_flags,
            output_names,
            output_icons,
            output_rates,
            output_locked_flags,
        );

        editor_node.building_count_str = building_count_str;
        editor_node.building_name = building_name;
        editor_node.same_clock_power_str = same_clock_power_str;
        editor_node.last_underclock_power_str = last_underclock_power_str;
        editor_node.variable_power = variable_power;
        editor_node.num_somersloop_str = num_somersloop_str;
        editor_node.somersloop_mult = somersloop_mult;
        // Map special somersloop icon from cache (if present)
        editor_node.somersloop_icon = self.item_icon_cache.get("Somersloop").map(|h| h.id());

        // If this is a group node, fetch build progress (built / total craft nodes) so UI can render a checkbox
        if let Some((built_count, total_count)) =
            self.production_app.get_node_build_progress(node_id)
        {
            if total_count > 0 {
                editor_node.group_built = Some(built_count == total_count);
            }
        }

        // If this is a sink node, compute sink points (sum of input rates * item sink value) and store for footer display
        if editor_node.node_type == "sink" {
            let mut sum = crate::fractional_number::FractionalNumber::default();
            for (opt_name, opt_rate) in editor_node
                .input_names
                .iter()
                .zip(editor_node.input_rates.iter())
            {
                if let (Some(name), Some(rate_str)) = (opt_name.as_ref(), opt_rate.as_ref()) {
                    if let Ok(r) = crate::fractional_number::FractionalNumber::from_string(rate_str)
                    {
                        if let Some(item_rc) = self.game_data.items.get(name) {
                            let pts = r * crate::fractional_number::FractionalNumber::from(
                                item_rc.sink_value as i64,
                            );
                            sum += pts;
                        }
                    }
                }
            }
            editor_node.sink_points_str = sum.to_float_string();
            editor_node.sink_points_fraction_str = sum.to_fraction_string();
        }

        editor_node
    }
}

impl eframe::App for TemplateApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Decrease error time
        self.error_time = (self.error_time - ctx.input(|i| i.unstable_dt)).max(0.0);

        self.show_top_panel(ctx);
        self.show_left_panel(ctx, frame);
        self.show_central_panel(ctx);
        self.show_dialogs(ctx);
    }
}

impl TemplateApp {
    fn show_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let is_web = cfg!(target_arch = "wasm32");

                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("📄 New").clicked() {
                            self.production_app = ProductionApp::new();
                            self.snarl = egui_snarl::Snarl::new();
                            ui.close();
                        }

                        if ui.button("📂 Open...").clicked() {
                            // TODO: Implement file open dialog
                            ui.close();
                        }

                        if ui.button("💾 Save...").clicked() {
                            // TODO: Implement file save dialog
                            ui.close();
                        }

                        ui.separator();

                        if ui.button("❌ Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                }

                ui.menu_button("Add Node", |ui| {
                    if ui.button("⚙️ Craft Node...").clicked() {
                        self.show_recipe_selector = true;
                        ui.close();
                    }
                    if ui.button("🔀 Splitter").clicked() {
                        let node_id = self.production_app.add_custom_splitter_node();
                        let en = self.build_editor_node(node_id, "Splitter*", "custom_splitter");
                        self.snarl.insert_node(egui::pos2(300.0, 300.0), en);
                        self.error_message = "Created Custom Splitter".to_string();
                        self.error_time = 2.0;
                        ui.close();
                    }
                    if ui.button("🔁 Merger").clicked() {
                        let node_id = self.production_app.add_merger_node();
                        let en = self.build_editor_node(node_id, "Merger", "merger");
                        self.snarl.insert_node(egui::pos2(300.0, 300.0), en);
                        self.error_message = "Created Merger".to_string();
                        self.error_time = 2.0;
                        ui.close();
                    }
                    if ui.button("📦 Sink").clicked() {
                        let node_id = self.production_app.add_sink_node();
                        let en = self.build_editor_node(node_id, "Sink", "sink");
                        self.snarl.insert_node(egui::pos2(300.0, 300.0), en);
                        self.error_message = "Created Sink".to_string();
                        self.error_time = 2.0;
                        self.show_controls_popup = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("🎨 Theme").clicked() {
                        ui.close();
                    }
                });

                ui.add_space(16.0);
                egui::widgets::global_theme_preference_buttons(ui);

                // Debug: draw a few test item icons on the top bar to confirm texture painting
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for name in ["Iron Ingot", "Iron Plate", "Rocket Fuel"].iter() {
                        if let Some(handle) = self.item_icon_cache.get(*name) {
                            let size = egui::Vec2::splat(28.0);
                            let (rect, _resp) = ui.allocate_exact_size(size, egui::Sense::hover());
                            ui.painter().image(
                                handle.id(),
                                rect,
                                egui::Rect::from_min_max(
                                    egui::pos2(0.0, 0.0),
                                    egui::pos2(1.0, 1.0),
                                ),
                                egui::Color32::WHITE,
                            );
                        } else {
                            ui.label(" ");
                        }
                    }
                });
            });
        });
    }

    fn show_left_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.left_panel_collapsed {
            egui::SidePanel::left("left_panel_collapsed")
                .resizable(false)
                .width_range(30.0..=30.0)
                .show(ctx, |ui| {
                    let response = ui.button("▶");
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if response.clicked() {
                        self.left_panel_collapsed = false;
                    }
                });
        } else {
            egui::SidePanel::left("left_panel")
                .resizable(true)
                .width_range(200.0..=500.0)
                .default_width(250.0)
                .show(ctx, |ui| {
                    ui.heading("⚙️ Production Controls");

                    // Save/Load section
                    ui.group(|ui| {
                        ui.label("Save/Load Production Chain:");
                        ui.text_edit_singleline(&mut self.save_name)
                            .on_hover_text("Enter a name to save or load");

                        ui.horizontal(|ui| {
                            if ui.button("💾 Save").clicked() {
                                if !self.save_name.is_empty() {
                                    match self.production_app.save_to_json() {
                                        Ok(_json) => {
                                            // TODO: Write to file
                                            self.error_message = format!("Saved: {}", self.save_name);
                                            self.error_time = 3.0;
                                        }
                                        Err(e) => {
                                            self.error_message = format!("Save error: {}", e);
                                            self.error_time = 3.0;
                                        }
                                    }
                                }
                            }

                            if ui.button("📂 Load").clicked() {
                                if !self.save_name.is_empty() {
                                    // TODO: Read from file and load
                                    self.error_message = format!("Load not implemented yet");
                                    self.error_time = 2.0;
                                }
                            }
                        });
                    });

                    ui.separator();

                    // Graph Statistics
                    ui.group(|ui| {
                        ui.label("📊 Graph Statistics:");
                        ui.label(format!("Nodes: {}", self.production_app.node_count()));
                        ui.label(format!("Links: {}", self.production_app.links.len()));
                        ui.label(format!(
                            "Recipes: {}",
                            self.production_app.get_recipe_names().len()
                        ));

                        // Power mode selection (mirror C++)
                        let resp = ui.checkbox(&mut self.power_equal_clocks, "Compute power with equal clocks");
                        if resp.changed() {
                            // Immediate update in viewer
                            self.snarl_viewer.power_equal_clocks = self.power_equal_clocks;
                        }
                        if resp.hovered() {
                            resp.on_hover_text("If set, the power will be calculated assuming all machines in a node are set at the same clock rate\nOtherwise, it will be calculated with machines at 100% and one last machine underclocked");
                        }
                    });

                    ui.separator();

                    // Controls popup
                    if ui.button("🎮 Show Controls").clicked() {
                        self.show_controls_popup = true;
                    }

                    ui.separator();

                    // Collapse button
                    if ui.button("◀ Collapse Panel").clicked() {
                        self.left_panel_collapsed = true;
                    }
                });
        }
    }

    fn show_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Production Graph Editor");

            // Show error message if present
            if !self.error_message.is_empty() && self.error_time > 0.0 {
                ui.colored_label(
                    if self.error_message.starts_with("error")
                        || self.error_message.starts_with("Error")
                    {
                        egui::Color32::RED
                    } else {
                        egui::Color32::GREEN
                    },
                    &self.error_message,
                );
                ui.ctx().request_repaint();
            }

            ui.separator();

            // Node editor (direct snarl widget so it receives events for selection and dragging)
            // Provide the viewer a name -> TextureId map so it can resolve icons during connect/sync
            self.snarl_viewer.set_icon_map(self.item_icon_cache.iter().map(|(k, h)| (k.clone(), h.id())).collect());
            let snarl_response = egui_snarl::ui::SnarlWidget::new()
                .id(egui::Id::new("production-snarl"))
                .style(self.snarl_style)
                .show(&mut self.snarl, &mut self.snarl_viewer, ui);

            // If the viewer rejected a connection, surface it as an error message for a short time
            if let Some(msg) = self.snarl_viewer.rejected_connection_reason.take() {
                self.error_message = msg;
                self.error_time = 3.0;
            }

            // Process pending pin rate edits collected by the SnarlViewer during rendering
            for (node_id, dir, idx, text) in self.snarl_viewer.drain_pending_edits() {
                match crate::fractional_number::FractionalNumber::from_string(&text) {
                    Ok(f) => {
                        if crate::rate_calculator::validate_rate(&f) {
                            match self.production_app.set_pin_rate(node_id, dir, idx, f) {
                                Ok(()) => {}
                                Err(e) => {
                                    self.error_message = format!("Error: {}", e);
                                    self.error_time = 3.0;
                                }
                            }
                        } else {
                            self.error_message = "Invalid rate (negative)".to_string();
                            self.error_time = 2.0;
                        }
                    }
                    Err(_) => {
                        self.error_message = "Invalid rate format".to_string();
                        self.error_time = 2.0;
                    }
                }
            }



            // Process somersloop edits collected by the SnarlViewer
            for (node_id, text) in self.snarl_viewer.drain_pending_somersloop_edits() {
                match crate::fractional_number::FractionalNumber::from_string(&text) {
                    Ok(f) => {
                        // Only accept non-negative integers. Use ProductionApp to apply and validate cap
                        match self.production_app.set_node_somersloop(node_id, f) {
                            Ok(()) => {}
                            Err(e) => {
                                self.error_message = format!("Error: {}", e);
                                self.error_time = 3.0;
                            }
                        }
                    }
                    Err(_) => {
                        self.error_message = "Invalid somersloop format".to_string();
                        self.error_time = 2.0;
                    }
                }
            }

            // Process pending group built edits collected by the SnarlViewer
            for (node_id, built) in self.snarl_viewer.drain_pending_built_edits() {
                match self.production_app.set_node_built_state(node_id, built) {
                    Ok(()) => {
                        // Update the snarl node data so the UI reflects the new state immediately
                        if let Some((built_count, total_count)) =
                            self.production_app.get_node_build_progress(node_id)
                        {
                            let new_state = if total_count > 0 {
                                Some(built_count == total_count)
                            } else {
                                None
                            };
                            for node_info in self.snarl.nodes_info_mut() {
                                if node_info.value.id == node_id {
                                    node_info.value.group_built = new_state;
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        self.error_message = format!("Error: {}", e);
                        self.error_time = 3.0;
                    }
                }
            }

            // Process pending pin additions (e.g., + button) collected by the SnarlViewer
            // Collect nodes that need a UI refresh after mutating the production model
            let mut nodes_to_refresh: Vec<u64> = Vec::new();

            for (node_id, dir) in self.snarl_viewer.drain_pending_pin_adds() {
                let res = match dir {
                    PinDirection::Input => self.production_app.add_input_pin_to_node(node_id),
                    PinDirection::Output => self.production_app.add_output_pin_to_node(node_id),
                };
                if let Err(e) = res {
                    self.error_message = format!("Error: {}", e);
                    self.error_time = 3.0;
                } else {
                    nodes_to_refresh.push(node_id);
                }
            }

            // Process pending pin removals (e.g., x button) collected by the SnarlViewer
            for (node_id, dir, idx) in self.snarl_viewer.drain_pending_pin_removes() {
                let res = match dir {
                    PinDirection::Input => self.production_app.remove_input_pin_from_node(node_id, idx),
                    PinDirection::Output => self.production_app.remove_output_pin_from_node(node_id, idx),
                };
                if let Err(e) = res {
                    self.error_message = format!("Error: {}", e);
                    self.error_time = 3.0;
                } else {
                    nodes_to_refresh.push(node_id);
                }
            }

            // Refresh modified nodes in the snarl widget (do this after mutations to avoid borrow conflicts)
            for node_id in nodes_to_refresh {
                let mut new_en = None;
                // Build new editor node first (immutable borrow)
                for node_info in self.snarl.nodes_info() {
                    if node_info.value.id == node_id {
                        let label = node_info.value.label.clone();
                        let node_type = node_info.value.node_type.clone();
                        new_en = Some(self.build_editor_node(node_id, label, node_type));
                        break;
                    }
                }
                if let Some(en) = new_en {
                    for node_info in self.snarl.nodes_info_mut() {
                        if node_info.value.id == node_id {
                            // Preserve sink per-pin item names/icons added via SnarlViewer sync behavior
                            if node_info.value.node_type == "sink" {
                                let mut merged = en.clone();
                                let old = node_info.value.clone();
                                let min_inputs = old.input_names.len().min(merged.input_names.len());
                                for i in 0..min_inputs {
                                    merged.input_names[i] = old.input_names[i].clone();
                                    merged.input_icons[i] = old.input_icons[i];
                                }
                                node_info.value = merged;
                            } else if node_info.value.node_type == "merger"
                                || node_info.value.node_type == "custom_splitter"
                                || node_info.value.node_type == "game_splitter"
                            {
                                // Preserve node-level item type and icon assigned by the viewer so footer remains visible
                                let mut merged = en.clone();
                                let old = node_info.value.clone();
                                merged.item_type = old.item_type.clone();
                                merged.item_type_icon = old.item_type_icon;
                                node_info.value = merged;
                                println!("SnarlViewer: preserved item_type '{:?}' for node {} during rebuild", node_info.value.item_type, node_id);
                            } else {
                                node_info.value = en.clone();
                            }
                            break;
                        }
                    }
                }
            }

            // Right-click context menu is handled in show_dialogs
            if snarl_response.secondary_clicked() {
                self.show_add_node_popup = true;
                self.add_node_popup_pos = ui
                    .ctx()
                    .pointer_interact_pos()
                    .unwrap_or(egui::pos2(300.0, 300.0));
            }

            ui.separator();

            // Bottom status bar
            ui.horizontal(|ui| {
                ui.label("💾 Status:");
                if self.production_app.has_unsaved_changes() {
                    ui.colored_label(egui::Color32::YELLOW, "⚠ Unsaved changes");
                } else {
                    ui.colored_label(egui::Color32::GREEN, "✓ All saved");
                }
            });
        });
    }

    fn show_dialogs(&mut self, ctx: &egui::Context) {
        // Add node context menu (right-click menu) - using Area for true context menu behavior
        if self.show_add_node_popup {
            let menu_id = egui::Id::new("add_node_context_menu");

            // Use Area to position the menu at the right-click location
            let response = egui::Area::new(menu_id)
                .fixed_pos(self.add_node_popup_pos)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .inner_margin(0.0)
                        .corner_radius(egui::CornerRadius::ZERO)
                        .show(ui, |ui| {
                            ui.set_max_width(350.0);
                            ui.spacing_mut().item_spacing.y = 0.0;
                            
                            // Helper to create context menu item
                            let mut menu_item = |ui: &mut egui::Ui, label: &str, tooltip: &str| -> bool {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
                                    egui::Sense::click()
                                );
                                
                                if response.hovered() {
                                    ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, ui.visuals().widgets.hovered.bg_fill);
                                }
                                
                                let mut child_ui = ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center), None);
                                child_ui.spacing_mut().button_padding = egui::vec2(8.0, 0.0);
                                child_ui.style_mut().interaction.selectable_labels = false;
                                let text_response = child_ui.label(label);
                                if !tooltip.is_empty() {
                                    text_response.on_hover_text(tooltip);
                                }
                                
                                response.clicked()
                            };
                            
                            // Basic nodes at top
                            if menu_item(ui, "Merger", "") {
                                let node_id = self.production_app.add_merger_node();
                                let en = self.build_editor_node(node_id, "Merger", "merger");
                                self.snarl.insert_node(self.add_node_popup_pos, en);
                                self.error_message = "Created Merger".to_string();
                                self.error_time = 2.0;
                                self.show_add_node_popup = false;
                            }
                            
                            if menu_item(ui, "Splitter*", "Splitter with independent output rates") {
                                let node_id = self.production_app.add_custom_splitter_node();
                                let en = self.build_editor_node(node_id, "Splitter*", "custom_splitter");
                                self.snarl.insert_node(self.add_node_popup_pos, en);
                                self.error_message = "Created Custom Splitter".to_string();
                                self.error_time = 2.0;
                                self.show_add_node_popup = false;
                            }
                            
                            if menu_item(ui, "Splitter", "Splitter with equal output rates") {
                                let node_id = self.production_app.add_game_splitter_node();
                                let en = self.build_editor_node(node_id, "Splitter", "game_splitter");
                                self.snarl.insert_node(self.add_node_popup_pos, en);
                                self.error_message = "Created Game Splitter".to_string();
                                self.error_time = 2.0;
                                self.show_add_node_popup = false;
                            }
                            
                            if menu_item(ui, "Sink", "") {
                                let node_id = self.production_app.add_sink_node();
                                let en = self.build_editor_node(node_id, "Sink", "sink");
                                self.snarl.insert_node(self.add_node_popup_pos, en);
                                self.error_message = "Created Sink".to_string();
                                self.error_time = 2.0;
                                self.show_add_node_popup = false;
                            }
                            
                            ui.separator();
                            
                            // Recipe filter like C++ - auto-focus on first show
                            let filter_response = ui.add(egui::TextEdit::singleline(&mut self.context_menu_recipe_filter).hint_text("Filter..."));
                            if ui.memory(|mem| mem.focused().is_none()) {
                                filter_response.request_focus();
                            }
                            
                            ui.separator();
                            
                            // Show recipes
                            let all_recipes: Vec<_> = self.game_data.recipes.iter()
                                .map(|r| r.clone())
                                .filter(|r| {
                                    if self.context_menu_recipe_filter.is_empty() {
                                        true
                                    } else {
                                        r.name.to_lowercase().contains(&self.context_menu_recipe_filter.to_lowercase()) ||
                                        r.display_name.to_lowercase().contains(&self.context_menu_recipe_filter.to_lowercase())
                                    }
                                })
                                .take(20)
                                .collect();
                            
                            if all_recipes.is_empty() && !self.game_data.recipes.is_empty() {
                                ui.label("No matching recipes");
                            } else if self.game_data.recipes.is_empty() {
                                ui.colored_label(egui::Color32::RED, "⚠ No game data loaded!");
                                ui.label("Check assets/satisfactory.json");
                            } else {
                                egui::ScrollArea::vertical()
                                    .auto_shrink([false; 2])
                                    .max_height(300.0)
                                    .show(ui, |ui| {
                                        // Pin the grid to the available menu width so icon column can sit flush right
                                        let menu_width = ui.available_width();
                                        let checkbox_column_width = ui.spacing().interact_size.y;
                                        let icon_size = egui::Vec2::splat(ui.text_style_height(&egui::TextStyle::Body));
                                        let mut max_inputs = 0usize;
                                        let mut max_outputs = 0usize;
                                        let mut any_arrow = false;
                                        for recipe in all_recipes.iter() {
                                            let ins = recipe.ins.len().min(4);
                                            let outs = recipe.outs.len().min(4);
                                            max_inputs = max_inputs.max(ins);
                                            max_outputs = max_outputs.max(outs);
                                            any_arrow |= ins > 0 && outs > 0;
                                        }
                                        let max_icons = max_inputs + max_outputs; // capped at 4 per side
                                        let arrow_width = if any_arrow { icon_size.x } else { 0.0 };
                                        let scroll_style = &ui.style().spacing.scroll;
                                        let scroll_bar_width = if scroll_style.floating {
                                            scroll_style.bar_width
                                        } else {
                                            scroll_style.allocated_width()
                                        };
                                        let icon_column_width = (max_icons as f32) * icon_size.x + arrow_width + scroll_bar_width;
                                        let name_column_width = (menu_width - checkbox_column_width - icon_column_width).max(80.0);

                                        egui::Grid::new("recipe_grid")
                                            .spacing([0.0, 0.0])
                                            .num_columns(3)
                                            .show(ui, |ui| {
                                                for recipe in all_recipes {
                                                    // First column: Checkbox for alternate recipes only
                                                    if recipe.alternate {
                                                        let checkbox_state = self.recipe_checkbox_state.entry(recipe.name.clone()).or_insert(true);
                                                        let mut checked = *checkbox_state;
                                                        if ui.checkbox(&mut checked, "").changed() {
                                                            self.recipe_checkbox_state.insert(recipe.name.clone(), checked);
                                                        }
                                                    } else {
                                                        // For non-alternate recipes, reserve space but don't show checkbox
                                                        ui.allocate_space(egui::vec2(checkbox_column_width, checkbox_column_width));
                                                    }
                                                    
                                                    // Second column: Recipe name as clickable area
                                                    let (rect, response) = ui.allocate_exact_size(
                                                        egui::vec2(name_column_width, ui.spacing().interact_size.y),
                                                        egui::Sense::click()
                                                    );
                                                    
                                                    if response.hovered() {
                                                        ui.painter().rect_filled(rect, egui::CornerRadius::ZERO, ui.visuals().widgets.hovered.bg_fill);
                                                    }
                                                    
                                                    let mut child_ui = ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center), None);
                                                    child_ui.spacing_mut().button_padding = egui::vec2(8.0, 0.0);
                                                    child_ui.style_mut().interaction.selectable_labels = false;
                                                    let text_response = child_ui.label(&recipe.display_name);
                                                    
                                                    ui.horizontal(|ui| {
                                                        ui.set_width(icon_column_width);
                                                        ui.spacing_mut().item_spacing.x = 0.0;
                                                        ui.style_mut().interaction.selectable_labels = false;
                                                        
                                                        // Show input icons with tooltips (left to right from left edge)
                                                        for inp in recipe.ins.iter().take(4) {
                                                            if let Some(handle) = self.item_icon_cache.get(&inp.item_name) {
                                                                let _img_response = ui.add(egui::Image::new(egui::load::SizedTexture::new(handle.id(), icon_size)));
                                                            }
                                                        }
                                                        
                                                        // Arrow between inputs and outputs
                                                        if !recipe.ins.is_empty() && !recipe.outs.is_empty() {
                                                            ui.label("-->");
                                                        }
                                                        
                                                        // Show output icons with tooltips (left to right)
                                                        for out in recipe.outs.iter().take(4) {
                                                            if let Some(handle) = self.item_icon_cache.get(&out.item_name) {
                                                                let _img_response = ui.add(egui::Image::new(egui::load::SizedTexture::new(handle.id(), icon_size)));
                                                            }
                                                        }
                                                    });
                                                    
                                                    ui.end_row();

                                                    if response.clicked() {
                                                        match self.production_app.add_craft_node(&recipe.name, &self.game_data) {
                                                            Ok(node_id) => {
                                                                let en = self.build_editor_node(node_id, &recipe.display_name, "craft");
                                                                self.snarl.insert_node(self.add_node_popup_pos, en);
                                                                self.error_message = format!("Created: {}", recipe.display_name);
                                                                self.error_time = 2.0;
                                                            }
                                                            Err(e) => {
                                                                self.error_message = format!("Error: {}", e);
                                                                self.error_time = 3.0;
                                                            }
                                                        }
                                                        self.context_menu_recipe_filter.clear();
                                                        self.show_add_node_popup = false;
                                                    }
                                                }
                                            });
                                    });
                            }
                            
                            if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                                self.show_add_node_popup = false;
                                self.context_menu_recipe_filter.clear();
                            }
                        });
                });

            // Close menu if clicked outside of it
            if ctx.input(|i| i.pointer.primary_clicked()) {
                if let Some(pointer_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                    let rect = response.response.rect;
                    if !rect.contains(pointer_pos) {
                        self.show_add_node_popup = false;
                        self.context_menu_recipe_filter.clear();
                    }
                }
            }
        }

        // Controls popup
        if self.show_controls_popup {
            let mut open = self.show_controls_popup;
            egui::Window::new("🎮 Keyboard Controls")
                .open(&mut open)
                .resizable(true)
                .default_width(400.0)
                .default_height(300.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            ui.heading("Node Editor Controls");
                            ui.separator();

                            ui.group(|ui| {
                                ui.label("⬅️ Pan: Middle Mouse Drag");
                                ui.label("🔍 Zoom: Scroll Wheel");
                                ui.label("✂️ Delete: Select + Delete Key");
                                ui.label("🔗 Connect: Drag pin to pin");
                            });

                            ui.separator();

                            ui.heading("Graph Controls");
                            ui.separator();

                            ui.group(|ui| {
                                ui.label("💾 Save: Ctrl+S");
                                ui.label("📂 Load: Ctrl+O");
                                ui.label("🔄 Undo: Ctrl+Z");
                                ui.label("↩️ Redo: Ctrl+Shift+Z");
                            });

                            ui.separator();

                            ui.heading("Lock Controls");
                            ui.separator();

                            ui.group(|ui| {
                                ui.label("🔒 Lock Pin: Right-Click → Lock");
                                ui.label("📌 Locked pins sync rate with connections");
                                ui.label("🔓 Splitters/Mergers have special logic");
                            });
                        });
                });
            self.show_controls_popup = open;
        }

        // Recipe selection dialog
        if self.show_recipe_selector {
            let mut open = true;
            egui::Window::new("⚙️ Select Recipe for Craft Node")
                .open(&mut open)
                .resizable(true)
                .default_width(400.0)
                .default_height(400.0)
                .show(ctx, |ui| {
                    ui.label("Search recipes:");
                    ui.text_edit_singleline(&mut self.recipe_search);

                    let recipes = self.production_app.get_recipe_names();
                    let filtered_recipes: Vec<_> = recipes
                        .iter()
                        .filter(|r| {
                            self.recipe_search.is_empty()
                                || r.to_lowercase()
                                    .contains(&self.recipe_search.to_lowercase())
                        })
                        .collect();

                    ui.label(format!("Found {} recipes", filtered_recipes.len()));

                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for recipe_name in filtered_recipes.iter() {
                                if ui.button(*recipe_name).clicked() {
                                    // Create the craft node
                                    match self
                                        .production_app
                                        .add_craft_node(recipe_name, &self.game_data)
                                    {
                                        Ok(node_id) => {
                                            let en = self.build_editor_node(
                                                node_id,
                                                *recipe_name,
                                                "craft",
                                            );
                                            self.snarl.insert_node(egui::pos2(300.0, 300.0), en);
                                            self.error_message =
                                                format!("Created: {}", recipe_name);
                                            self.error_time = 2.0;
                                        }
                                        Err(e) => {
                                            self.error_message = format!("Error: {}", e);
                                            self.error_time = 3.0;
                                        }
                                    }
                                    self.selected_recipe = Some((*recipe_name).clone());
                                    self.show_recipe_selector = false;
                                    self.recipe_search.clear();
                                }
                            }
                        });

                    ui.separator();
                    if ui.button("Cancel").clicked() {
                        self.show_recipe_selector = false;
                    }
                });
            self.show_recipe_selector = open;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_editor_node_maps_icons_from_cache() {
        let mut app = TemplateApp::default();

        let recipe = app
            .game_data
            .recipes
            .get(0)
            .expect("No recipes loaded")
            .clone();
        assert!(!recipe.outs.is_empty(), "Recipe has no outputs");

        let output_item_name = recipe.outs[0].item_name.clone();

        // Insert a fake texture handle into the cache for that item using a local egui context
        let ctx = egui::Context::default();
        let color = egui::ColorImage::example();
        let handle = ctx.load_texture("test", color, egui::TextureOptions::NEAREST);
        app.item_icon_cache.insert(output_item_name.clone(), handle);

        // Add craft node using the recipe
        let node_id = app
            .production_app
            .add_craft_node(&recipe.name, &app.game_data)
            .expect("Failed to add craft node");

        // Build the editor node and ensure output icons contains at least one Some
        let en = app.build_editor_node(node_id, &recipe.display_name, "craft");
        assert!(
            en.output_icons.iter().any(|o| o.is_some()),
            "Output icons were not mapped from cache"
        );
    }
}
