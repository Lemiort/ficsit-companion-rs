use crate::production_app::ProductionApp;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings{
    pub show_spolres : bool,
    pub show_somersloop: bool,
    pub unlocked_alts: HashMap<String, bool>,
    pub power_equal_clocks: bool,
    pub show_build_progress: bool,
    pub left_panel_folded: bool,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            show_spolres: false,
            show_somersloop: false,
            unlocked_alts: HashMap::new(),
            power_equal_clocks: true,
            show_build_progress: false,
            left_panel_folded: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Craft,
    Merger,
    GameSplitter,
    CustomSplitter,
    Sink,
    Group,
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

// Allow constructing NodeType from string literals (used by tests and helpers)
impl From<&str> for NodeType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "craft" => NodeType::Craft,
            "merger" => NodeType::Merger,
            "gamesplitter" | "game_splitter" | "game-splitter" => NodeType::GameSplitter,
            "customsplitter" | "custom_splitter" | "custom-splitter" => NodeType::CustomSplitter,
            "sink" => NodeType::Sink,
            "group" => NodeType::Group,
            _ => NodeType::Craft,
        }
    }
}

impl From<String> for NodeType {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

/// Simple node representation for the node editor
#[derive(Clone, Debug)]
pub struct EditorNode {
    pub id: u64,
    pub label: String,
    pub node_type: NodeType,

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
    pub fn new(id: u64, label: impl Into<String>, node_type: impl Into<NodeType>) -> Self {
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
        node_type: impl Into<NodeType>,
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
use std::{collections::HashMap, fmt};

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

    // Pending building count edits: node_id -> string
    pending_node_building_edits: Vec<(u64, String)>,

    // Pending group built edits: node_id -> bool
    pending_node_built_edits: Vec<(u64, bool)>,

    // Pending pin add/remove ops collected during rendering
    pending_pin_adds: Vec<(u64, crate::pin::PinDirection)>,
    pending_pin_removes: Vec<(u64, crate::pin::PinDirection, usize)>,

    // Pending connection/disconnection events to be processed by TemplateApp
    pending_connections: Vec<(egui_snarl::OutPinId, egui_snarl::InPinId)>,
    pending_disconnects: Vec<(egui_snarl::OutPinId, egui_snarl::InPinId)>,

    // Pending dropped wire action recorded by show_dropped_wire_menu / show_graph_menu
    pending_dropped_wire: Option<PendingDroppedWire>,
    // UI-only locked nodes set (visual lock toggled by right-click on node header)
    ui_locked_nodes: std::collections::HashSet<u64>,

    // Pending node lock changes requested by the viewer (node_id, locked)
    pending_node_lock_changes: Vec<(u64, bool)>,

    // Pending node item type changes requested by the viewer (node_id, Option<item_name>)
    pending_node_item_changes: Vec<(u64, Option<String>)>,

    // Recent pin edit successes (node_id, direction, pin_idx) -> Instant
    pub pin_success: std::collections::HashMap<(u64, PinDirection, usize), std::time::Instant>,

    // Map of item name -> TextureId supplied by the app so the viewer can resolve icons immediately
    icon_map: std::collections::HashMap<String, egui::TextureId>,

    // Recipes & context filter for legacy-style graph add menu
    pub recipes: Vec<std::rc::Rc<crate::recipe::Recipe>>,
    pub context_menu_recipe_filter: String,
    pub recipe_checkbox_state: std::collections::HashMap<String, bool>,

    // Whether to display same-clock or last-underclock in UI
    pub power_equal_clocks: bool,

    // Last reason a connection was rejected by the viewer (displayed as error_message by TemplateApp)
    pub rejected_connection_reason: Option<String>,
}

// Pending dropped wire types (used by viewer to record actions for TemplateApp to execute)
#[derive(Clone, Debug)]
pub enum DroppedWireChoice {
    Merger,
    CustomSplitter,
    GameSplitter,
    Sink,
    Craft(Option<String>),
}

#[derive(Clone, Debug)]
pub struct PendingDroppedWire {
    pub pos: egui::Pos2,
    pub src_outs: Option<Vec<egui_snarl::OutPinId>>,
    pub src_ins: Option<Vec<egui_snarl::InPinId>>,
    pub src_item_name: Option<String>,
    pub choice: DroppedWireChoice,
}

impl SnarlViewer {
    // Fixed inset before the footer '+' (used for both input and output placements)
    const FOOTER_ADD_INSET: f32 = 48.0;

    fn drain_pending_edits(&mut self) -> Vec<(u64, PinDirection, usize, String)> {
        std::mem::take(&mut self.pending_pin_rate_edits)
    }

    fn drain_pending_connections(&mut self) -> Vec<(egui_snarl::OutPinId, egui_snarl::InPinId)> {
        std::mem::take(&mut self.pending_connections)
    }

    fn drain_pending_disconnects(&mut self) -> Vec<(egui_snarl::OutPinId, egui_snarl::InPinId)> {
        std::mem::take(&mut self.pending_disconnects)
    }

    // Mark a successful pin edit (record time)
    fn mark_pin_success(&mut self, node_id: u64, dir: PinDirection, idx: usize) {
        self.pin_success.insert((node_id, dir, idx), std::time::Instant::now());
    }

    // Return true if the pin edit was successful recently (within 1.5s)
    fn is_pin_recent_success(&self, node_id: u64, dir: PinDirection, idx: usize) -> bool {
        if let Some(t) = self.pin_success.get(&(node_id, dir, idx)) {
            t.elapsed().as_secs_f32() < 1.5
        } else {
            false
        }
    }

    fn drain_pending_somersloop_edits(&mut self) -> Vec<(u64, String)> {
        std::mem::take(&mut self.pending_node_somersloop_edits)
    }

    fn drain_pending_building_edits(&mut self) -> Vec<(u64, String)> {
        std::mem::take(&mut self.pending_node_building_edits)
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

    fn drain_pending_dropped_wire(&mut self) -> Option<PendingDroppedWire> {
        self.pending_dropped_wire.take()
    }

    fn drain_pending_node_lock_changes(&mut self) -> Vec<(u64, bool)> {
        std::mem::take(&mut self.pending_node_lock_changes)
    }

    fn drain_pending_node_item_changes(&mut self) -> Vec<(u64, Option<String>)> {
        std::mem::take(&mut self.pending_node_item_changes)
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
        // Use hover sense so inner TextEdit receives clicks; capture alloc_response to force-focus inner widget
        let (rect, alloc_response) = ui.allocate_exact_size(
            egui::Vec2::new(width, ui.spacing().interact_size.y),
            egui::Sense::hover(),
        );

        // Active input: render TextEdit inside the reserved rect
        let text_edit = egui::TextEdit::singleline(buf_ref).desired_width(width);
        let response = ui
            .allocate_ui_at_rect(rect, |ui| ui.add_enabled(!disabled, text_edit))
            .response;

        // If outer rect was clicked, ensure the inner TextEdit gets focus
        if alloc_response.clicked() {
            log::debug!("[UI][debug] outer rect clicked at rect={:?}", rect);
            response.request_focus();
            log::debug!("[UI][debug] requested focus for inner TextEdit");
        }

        // Focus highlight (blue)
        if response.has_focus() || response.gained_focus() {
            log::debug!("[UI][debug] response focus state: has_focus={} gained_focus={} clicked={}",
                      response.has_focus(), response.gained_focus(), response.clicked());
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

    /// Shared add-node menu renderer for both graph menu and dropped-wire menu
    fn draw_add_node_menu_contents(
        &mut self,
        pos: egui::Pos2,
        ui: &mut egui::Ui,
        filter_item: Option<&str>,
        filter_by_output: bool,
    ) -> Option<DroppedWireChoice> {
        // Cap menu width to avoid unlimited expansion and match legacy popup
        ui.set_max_width(350.0);
        ui.spacing_mut().item_spacing.y = 0.0;

        // Helper to render a simple menu item (hover highlight + click)
        let menu_item = |ui: &mut egui::Ui, label: &str, tooltip: &str| -> bool {
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
                egui::Sense::click(),
            );
            if response.hovered() {
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::ZERO,
                    ui.visuals().widgets.hovered.bg_fill,
                );
            }
            let mut child_ui =
                ui.child_ui(rect, egui::Layout::left_to_right(egui::Align::Center), None);
            child_ui.spacing_mut().button_padding = egui::vec2(8.0, 0.0);
            child_ui.style_mut().interaction.selectable_labels = false;
            let text_response = child_ui.label(label);
            if !tooltip.is_empty() {
                text_response.on_hover_text(tooltip);
            }
            response.clicked()
        };

        if menu_item(ui, "Merger", "") {
            return Some(DroppedWireChoice::Merger);
        }

        if menu_item(ui, "Splitter*", "Splitter with independent output rates") {
            return Some(DroppedWireChoice::CustomSplitter);
        }

        if menu_item(ui, "Splitter", "Splitter with equal output rates") {
            return Some(DroppedWireChoice::GameSplitter);
        }

        if menu_item(ui, "Sink", "") {
            return Some(DroppedWireChoice::Sink);
        }

        ui.separator();

        // Recipe filter box (disabled when filter_item is provided)
        if filter_item.is_none() {
            let filter_response = ui.add(
                egui::TextEdit::singleline(&mut self.context_menu_recipe_filter)
                    .hint_text("Filter..."),
            );
            if ui.memory(|mem| mem.focused().is_none()) {
                filter_response.request_focus();
            }
            ui.separator();
        } else {
            if filter_by_output {
                ui.label(format!("Recipes with output: {}", filter_item.unwrap()));
            } else {
                ui.label(format!("Recipes with input: {}", filter_item.unwrap()));
            }
            ui.separator();
        }

        // Show recipes (use cached list from TemplateApp)
        let all_recipes: Vec<_> = self
            .recipes
            .iter()
            .map(|r| r.clone())
            .filter(|r| {
                if let Some(item) = filter_item {
                    // Only recipes that include the specified item as an input or output depending on the drop direction
                    if filter_by_output {
                        r.outs.iter().any(|out| out.item_name == item)
                    } else {
                        r.ins.iter().any(|inp| inp.item_name == item)
                    }
                } else if self.context_menu_recipe_filter.is_empty() {
                    true
                } else {
                    r.name
                        .to_lowercase()
                        .contains(&self.context_menu_recipe_filter.to_lowercase())
                        || r.display_name
                            .to_lowercase()
                            .contains(&self.context_menu_recipe_filter.to_lowercase())
                }
            })
            .take(20)
            .collect();

        if all_recipes.is_empty() && !self.recipes.is_empty() {
            ui.label("No matching recipes");
        } else if self.recipes.is_empty() {
            ui.colored_label(egui::Color32::RED, "⚠ No game data loaded!");
            ui.label("Check assets/satisfactory.json");
        } else {
            let mut chosen: Option<DroppedWireChoice> = None;
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .max_height(300.0)
                .show(ui, |ui| {
                    // Layout sizes
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
                    let max_icons = max_inputs + max_outputs;
                    let arrow_width = if any_arrow { icon_size.x } else { 0.0 };
                    let scroll_style = &ui.style().spacing.scroll;
                    let scroll_bar_width = if scroll_style.floating {
                        scroll_style.bar_width
                    } else {
                        scroll_style.allocated_width()
                    };
                    let icon_column_width =
                        (max_icons as f32) * icon_size.x + arrow_width + scroll_bar_width;
                    let name_column_width =
                        (menu_width - checkbox_column_width - icon_column_width).max(80.0);

                    egui::Grid::new("recipe_grid")
                        .spacing([0.0, 0.0])
                        .num_columns(3)
                        .show(ui, |ui| {
                            for recipe in all_recipes {
                                // Checkbox for alternate recipes
                                if recipe.alternate {
                                    let checkbox_state = self
                                        .recipe_checkbox_state
                                        .entry(recipe.name.clone())
                                        .or_insert(true);
                                    let mut checked = *checkbox_state;
                                    if ui.checkbox(&mut checked, "").changed() {
                                        self.recipe_checkbox_state
                                            .insert(recipe.name.clone(), checked);
                                    }
                                } else {
                                    ui.allocate_space(egui::vec2(
                                        checkbox_column_width,
                                        checkbox_column_width,
                                    ));
                                }

                                // Recipe name column clickable
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(name_column_width, ui.spacing().interact_size.y),
                                    egui::Sense::click(),
                                );
                                if response.hovered() {
                                    ui.painter().rect_filled(
                                        rect,
                                        egui::CornerRadius::ZERO,
                                        ui.visuals().widgets.hovered.bg_fill,
                                    );
                                }
                                let mut child_ui = ui.child_ui(
                                    rect,
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    None,
                                );
                                child_ui.spacing_mut().button_padding = egui::vec2(8.0, 0.0);
                                child_ui.style_mut().interaction.selectable_labels = false;
                                let _text_response = child_ui.label(&recipe.display_name);

                                ui.horizontal(|ui| {
                                    ui.set_width(icon_column_width);
                                    ui.spacing_mut().item_spacing.x = 0.0;
                                    ui.style_mut().interaction.selectable_labels = false;

                                    // Inputs icons
                                    for inp in recipe.ins.iter().take(4) {
                                        if let Some(tex) = self.icon_map.get(&inp.item_name) {
                                            let _ = ui.image((*tex, icon_size));
                                        }
                                    }

                                    if !recipe.ins.is_empty() && !recipe.outs.is_empty() {
                                        ui.label("-->");
                                    }

                                    // Outputs icons
                                    for out in recipe.outs.iter().take(4) {
                                        if let Some(tex) = self.icon_map.get(&out.item_name) {
                                            let _ = ui.image((*tex, icon_size));
                                        }
                                    }
                                });

                                ui.end_row();

                                if response.clicked() {
                                    // Request TemplateApp to create the craft node for this recipe
                                    chosen =
                                        Some(DroppedWireChoice::Craft(Some(recipe.name.clone())));
                                }

                                if chosen.is_some() {
                                    return;
                                }
                            }
                        });
                });

            if let Some(ch) = chosen {
                self.context_menu_recipe_filter.clear();
                return Some(ch);
            }
        }

        // Close on Escape
        if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
            self.context_menu_recipe_filter.clear();
            ui.close();
        }

        None
    }
}

impl SnarlViewer {
    /// Compute and cache per-node output row dimensions (width includes circle margin)
    fn compute_output_row_dims(
        &mut self,
        ui: &egui::Ui,
        node: &EditorNode,
        size: egui::Vec2,
    ) -> (f32, f32) {
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
                        .layout_no_wrap(
                            line.to_owned(),
                            egui::FontId::default(),
                            egui::Color32::WHITE,
                        )
                        .size()
                        .x;
                    if w > label_w {
                        label_w = w;
                    }
                }
                if label_w > max_label_w {
                    max_label_w = label_w;
                }
                if line_count > max_lines {
                    max_lines = line_count;
                }
            }
            let gap = 6.0;
            let circle_margin = size.x * 0.6;
            let computed_row_width = 88.0 + gap + size.x + gap + max_label_w + circle_margin;
            let line_height = ui.text_style_height(&egui::TextStyle::Body);
            let mut computed_row_height = (max_lines as f32) * line_height;
            if computed_row_height < size.y {
                computed_row_height = size.y;
            }
            self.output_row_width = Some(computed_row_width);
            self.output_row_height = Some(computed_row_height);
        }
        (
            self.output_row_width.unwrap(),
            self.output_row_height.unwrap(),
        )
    }

    // Synchronize merger/splitter pin types for a node: if any remote connections exist,
    // pick the first remote's item name and set all pins of that direction to it.
    // If there are no connections, clear the names.
    fn sync_merger_splitter(
        &mut self,
        snarl: &mut egui_snarl::Snarl<EditorNode>,
        node_id: egui_snarl::NodeId,
    ) {
        // Read-only pass: determine chosen item name (avoid simultaneous mutable/immutable borrows of snarl)
        if let Some(node_ref) = snarl.get_node(node_id) {
            match node_ref.node_type {
                NodeType::Merger => {
                    let mut chosen: Option<String> = None;
                    for input_idx in 0..node_ref.input_names.len() {
                        let in_id = egui_snarl::InPinId {
                            node: node_id,
                            input: input_idx,
                        };
                        let in_pin = snarl.in_pin(in_id);
                        if let Some(remote) = in_pin.remotes.first() {
                            if let Some(remote_node) = snarl.get_node(remote.node) {
                                if let Some(Some(name)) =
                                    remote_node.output_names.get(remote.output)
                                {
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
                            println!(
                                "SnarlViewer: set item_type '{}' on node {} ({})",
                                name, node_mut.id, node_mut.node_type
                            );
                            println!(
                                "SnarlViewer: propagated '{}' to outputs and set footer icon {:?}",
                                name, node_mut.item_type_icon
                            );

                            // Notify TemplateApp to update production model (set organizer node's item_name and pins)
                            self.pending_node_item_changes.push((node_mut.id, Some(name.clone())));
                            log::debug!("[UI] queued pending_node_item_change: node={} item={}", node_mut.id, name);
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

                            // Notify TemplateApp to clear production model organizer item
                            self.pending_node_item_changes.push((node_mut.id, None));
                            log::debug!("[UI] queued pending_node_item_change: node={} item=None", node_mut.id);

                            // Debug
                            println!(
                                "SnarlViewer: cleared item_type on node {} ({})",
                                node_mut.id, node_mut.node_type
                            );
                        }

                        // If this node is currently being rendered, update the cached clone so the footer shows changes immediately
                        if let Some(cur) = self.current_node.as_mut() {
                            if cur.id == node_mut.id {
                                *cur = node_mut.clone();
                            }
                        }
                    }
                }
                NodeType::Sink => {
                    // Sinks should NOT have a node-level item_type — pins carry their own types.
                    // Collect per-input chosen item names (read-only pass)
                    println!(
                        "SnarlViewer: checking sink node {} ({}) inputs (count={})",
                        node_ref.id,
                        node_ref.node_type,
                        node_ref.input_names.len()
                    );
                    let mut chosen_per_input: Vec<Option<String>> =
                        Vec::with_capacity(node_ref.input_names.len());
                    for input_idx in 0..node_ref.input_names.len() {
                        let in_id = egui_snarl::InPinId {
                            node: node_id,
                            input: input_idx,
                        };
                        let in_pin = snarl.in_pin(in_id);
                        if in_pin.remotes.is_empty() {
                            println!("  input[{}]: no remotes", input_idx);
                            chosen_per_input.push(None);
                        } else {
                            // pick first remote name (if any)
                            let mut found: Option<String> = None;
                            for r in in_pin.remotes.iter() {
                                if let Some(remote_node) = snarl.get_node(r.node) {
                                    let name_opt = remote_node
                                        .output_names
                                        .get(r.output)
                                        .and_then(|o| o.clone());
                                    println!(
                                        "  input[{}] remote -> node {:?} output {} name={:?}",
                                        input_idx, r.node, r.output, name_opt
                                    );
                                    if let Some(n) = name_opt {
                                        found = Some(n);
                                        break;
                                    }
                                } else {
                                    println!(
                                        "  input[{}] remote -> node {:?} output {} (node not found)",
                                        input_idx, r.node, r.output
                                    );
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
                                node_mut.input_icons[idx] = chosen_opt
                                    .as_ref()
                                    .and_then(|n| self.icon_map.get(n).copied());
                                if let Some(n) = chosen_opt {
                                    println!(
                                        "SnarlViewer: set input[{}] name='{}' on sink node {}",
                                        idx, n, node_mut.id
                                    );
                                } else {
                                    println!(
                                        "SnarlViewer: cleared input[{}] name on sink node {}",
                                        idx, node_mut.id
                                    );
                                }
                            }
                        }

                        // Ensure node-level item_type is cleared
                        node_mut.item_type = None;
                        node_mut.item_type_icon = None;

                        println!(
                            "SnarlViewer: sink node {} ({}) retains per-pin types; node-level item_type cleared",
                            node_mut.id, node_mut.node_type
                        );

                        // Update cached current node if needed so UI reflects cleared state immediately
                        if let Some(cur) = self.current_node.as_mut() {
                            if cur.id == node_mut.id {
                                *cur = node_mut.clone();
                            }
                        }
                    }
                }
                NodeType::Group => {
                    // Groups do not have item types
                }
                NodeType::Craft => {
                    // Craft nodes do not have item types
                }
                NodeType::CustomSplitter | NodeType::GameSplitter => {
                    let mut chosen: Option<String> = None;
                    println!(
                        "SnarlViewer: examining splitter node {:?} inputs={:?} outputs={:?}",
                        node_id, node_ref.input_names, node_ref.output_names
                    );

                    // First try: check inputs' remotes (source -> splitter input), prefer remote's output name
                    for input_idx in 0..node_ref.input_names.len() {
                        let in_id = egui_snarl::InPinId {
                            node: node_id,
                            input: input_idx,
                        };
                        let in_pin = snarl.in_pin(in_id);
                        if let Some(remote) = in_pin.remotes.first() {
                            if let Some(remote_node) = snarl.get_node(remote.node) {
                                if let Some(Some(name)) =
                                    remote_node.output_names.get(remote.output)
                                {
                                    chosen = Some(name.clone());
                                    println!(
                                        "SnarlViewer: splitter candidate from input[{}] remote node {:?} output {} = {:?}",
                                        input_idx, remote.node, remote.output, name
                                    );
                                    break;
                                } else if let Some(name) =
                                    remote_node.output_names.iter().find_map(|o| o.clone())
                                {
                                    chosen = Some(name.clone());
                                    println!(
                                        "SnarlViewer: splitter fallback from input[{}] remote node {:?} any output = {:?}",
                                        input_idx, remote.node, name
                                    );
                                    break;
                                } else {
                                    println!(
                                        "SnarlViewer: splitter input[{}] remote node {:?} had no output names",
                                        input_idx, remote.node
                                    );
                                }
                            }
                        } else {
                            println!("SnarlViewer: input[{}] has no remotes", input_idx);
                        }
                    }

                    // Fallback: inspect outputs' remotes (downstream nodes)
                    if chosen.is_none() {
                        for output_idx in 0..node_ref.output_names.len() {
                            let out_id = egui_snarl::OutPinId {
                                node: node_id,
                                output: output_idx,
                            };
                            let out_pin = snarl.out_pin(out_id);
                            if let Some(remote) = out_pin.remotes.first() {
                                if let Some(remote_node) = snarl.get_node(remote.node) {
                                    println!(
                                        "SnarlViewer: splitter remote node {:?} input_names={:?} output_names={:?}",
                                        remote.node,
                                        remote_node.input_names,
                                        remote_node.output_names
                                    );
                                    // Prefer the remote node's input pin name (the splitter feeds that input),
                                    // but fall back to any input name, then any output name on remote node.
                                    let mut found_name: Option<String> = None;
                                    if let Some(Some(name)) =
                                        remote_node.input_names.get(remote.input)
                                    {
                                        found_name = Some(name.clone());
                                        println!(
                                            "SnarlViewer: splitter candidate from remote node {:?} input {} = {:?}",
                                            remote.node, remote.input, name
                                        );
                                    } else if let Some(name) =
                                        remote_node.input_names.iter().find_map(|o| o.clone())
                                    {
                                        // fall back to any input name on remote node
                                        found_name = Some(name.clone());
                                        println!(
                                            "SnarlViewer: splitter fallback from remote node {:?} any input = {:?}",
                                            remote.node, name
                                        );
                                    } else if let Some(name) =
                                        remote_node.output_names.iter().find_map(|o| o.clone())
                                    {
                                        // last resort: pick any output name on remote node
                                        found_name = Some(name.clone());
                                        println!(
                                            "SnarlViewer: splitter fallback from remote node {:?} any output = {:?}",
                                            remote.node, name
                                        );
                                    } else {
                                        println!(
                                            "SnarlViewer: splitter remote node {:?} had no input/output names",
                                            remote.node
                                        );
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
                        println!(
                            "SnarlViewer: no chosen name for splitter node {:?} after inspection",
                            node_id
                        );
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
                            println!(
                                "SnarlViewer: set item_type '{}' on node {} ({})",
                                name, node_mut.id, node_mut.node_type
                            );
                            println!(
                                "SnarlViewer: propagated '{}' to inputs/outputs and set footer icon {:?}",
                                name, node_mut.item_type_icon
                            );
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
                            println!(
                                "SnarlViewer: cleared item_type on node {} ({})",
                                node_mut.id, node_mut.node_type
                            );
                        }

                        // If this node is currently being rendered, update the cached clone so the footer shows changes immediately
                        if let Some(cur) = self.current_node.as_mut() {
                            if cur.id == node_mut.id {
                                *cur = node_mut.clone();
                            }
                        }
                    }
                }
            }
        }
    }

    /// Provide a lightweight name -> TextureId map so the viewer can resolve icons during connect/sync
    fn set_icon_map(&mut self, map: std::collections::HashMap<String, egui::TextureId>) {
        self.icon_map = map;
    }

    // Helper to render a '+' in the footer aligned to a column depending on direction
    // Input -> column 1 (left) | Output -> column 3 (right)
    fn render_footer_add_button_middle(
        &mut self,
        ui: &mut egui::Ui,
        node: &EditorNode,
        dir: PinDirection,
    ) {
        egui::Grid::new(format!(
            "footer_add_col:{}:{}",
            node.id,
            match dir {
                PinDirection::Input => "in",
                PinDirection::Output => "out",
            }
        ))
        .num_columns(3)
        .spacing([8.0, 8.0])
        .min_col_width(ui.available_width() / 3.0)
        .show(ui, |ui| {
            match dir {
                PinDirection::Input => {
                    // Place in first column with left inset
                    ui.horizontal(|ui| {
                        ui.add_space(Self::FOOTER_ADD_INSET);
                        if ui
                            .add(
                                egui::Button::new("+")
                                    .corner_radius(egui::CornerRadius::same(0))
                                    .small(),
                            )
                            .clicked()
                        {
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
                        if ui
                            .add(
                                egui::Button::new("+")
                                    .corner_radius(egui::CornerRadius::same(0))
                                    .small(),
                            )
                            .clicked()
                        {
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
                if node.node_type == NodeType::Merger || node.node_type == NodeType::Sink {
                    let can_remove = node.input_names.len() > 1;
                    let btn = egui::Button::new("x")
                        .corner_radius(egui::CornerRadius::same(0))
                        .small();
                    let resp = ui.add_enabled(can_remove, btn);
                    if resp.clicked() {
                        self.pending_pin_removes
                            .push((node.id, PinDirection::Input, idx));
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
                    let node_locked = self.ui_locked_nodes.contains(&node.id);
                    let response =
                        self.render_fractional_input(ui, &key, &mut tmp, desired_width, disabled || node_locked);
                    // Submit when the user presses Enter while focused, or when the field loses focus and the text changed.
                    let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    // Debug info to help diagnose missed submits
                    if enter_pressed || response.lost_focus() || response.changed() || response.has_focus() {
                        log::debug!("[UI][debug] key={} enter={} has_focus={} changed={} lost_focus={}", key, enter_pressed, response.has_focus(), response.changed(), response.lost_focus());
                    }
                    // Be tolerant: allow Enter to submit if the field has focus or the mouse is hovering it
                    // Also accept Enter if the edit buffer differs from the displayed rate string (user typed without focusing)
                    let buf_opt = self.edit_buffers.get(&key).cloned();
                    let displayed_str = rate.clone();
                    let typed_differs = match &buf_opt {
                        Some(b) => b != &displayed_str,
                        None => false,
                    };
                    let submit = (enter_pressed && (response.has_focus() || response.hovered() || typed_differs)) || (response.lost_focus() && response.changed());
                    if enter_pressed && !response.has_focus() && response.hovered() {
                        log::debug!("[UI][debug] Enter pressed while not focused but hovered: key={}", key);
                    }
                    if enter_pressed && typed_differs {
                        log::debug!("[UI][debug] Enter pressed and edit buffer differs from displayed value: key={} buf={:?} displayed={}", key, buf_opt, displayed_str);
                    }
                    if submit {
                        if let Some(buf) = self.edit_buffers.get(&key) {
                            self.pending_pin_rate_edits.push((
                                node.id,
                                PinDirection::Input,
                                idx,
                                buf.clone(),
                            ));
                            log::info!("[UI] queued edit: node={} dir=Input idx={} -> {}", node.id, idx, buf);
                        } else {
                            log::debug!("[UI][debug] edit_buffers missing for key {}", key);
                        }
                    }

                    // Inline success indicator (green dot) if this pin had a recent successful edit
                    if self.is_pin_recent_success(node.id, PinDirection::Input, idx) {
                        let dot_center = egui::pos2(response.rect.right() + 8.0, response.rect.center().y);
                        ui.painter().circle_filled(dot_center, 4.0, egui::Color32::from_rgb(80, 200, 120));
                    }
                }

                // Icon + Label handling
                if node.node_type == NodeType::Sink {
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
                    if node.node_type != NodeType::Merger
                        && node.node_type != NodeType::CustomSplitter
                        && node.node_type != NodeType::GameSplitter
                    {
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
            let (slot_rect, _slot_resp) =
                ui.allocate_exact_size(egui::vec2(row_width, row_height), egui::Sense::hover());
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
                    if node.node_type == NodeType::CustomSplitter || node.node_type == NodeType::GameSplitter {
                        let can_remove = node.output_names.len() > 1;
                        let btn = egui::Button::new("x")
                            .corner_radius(egui::CornerRadius::same(0))
                            .small();
                        let resp = ui.add_enabled(can_remove, btn);
                        if resp.clicked() {
                            self.pending_pin_removes
                                .push((node.id, PinDirection::Output, idx));
                        }
                    }

                    // Rate first (near outer edge for outputs)
                    if let Some(Some(rate)) = node.output_rates.get(idx) {
                        let key = format!("pin:{}:out:{}", node.id, idx);
                        // Use a conservative fixed width similar to C++ "0000.000"
                        let desired_width = 88.0;
                        let mut tmp = rate.clone();
                        let disabled = node.output_locked.get(idx).copied().unwrap_or(false);
                        let node_locked = self.ui_locked_nodes.contains(&node.id);
                        let response = self.render_fractional_input(
                            ui,
                            &key,
                            &mut tmp,
                            desired_width,
                            disabled || node_locked,
                        );
                        rate_rect = Some(response.rect);
                        // Submit when the user presses Enter while focused, or when the field loses focus and the text changed.
                        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        // Debug info to help diagnose missed submits
                        if enter_pressed || response.lost_focus() || response.changed() || response.has_focus() {
                            eprintln!("[UI][debug] key={} enter={} has_focus={} changed={} lost_focus={}",
                                      key, enter_pressed, response.has_focus(), response.changed(), response.lost_focus());
                        }
                        // Be tolerant: allow Enter to submit if the field has focus or the mouse is hovering it
                        // Also accept Enter if the edit buffer differs from the displayed rate string (user typed without focusing)
                        let buf_opt = self.edit_buffers.get(&key).cloned();
                        let displayed_str = rate.clone();
                        let typed_differs = match &buf_opt {
                            Some(b) => b != &displayed_str,
                            None => false,
                        };
                        let submit = (enter_pressed && (response.has_focus() || response.hovered() || typed_differs)) || (response.lost_focus() && response.changed());
                        if enter_pressed && !response.has_focus() && response.hovered() {
                            log::debug!("[UI][debug] Enter pressed while not focused but hovered: key={}", key);
                        }
                        if enter_pressed && typed_differs {
                            log::debug!("[UI][debug] Enter pressed and edit buffer differs from displayed value: key={} buf={:?} displayed={}", key, buf_opt, displayed_str);
                        }
                        if submit {
                            if let Some(buf) = self.edit_buffers.get(&key) {
                                self.pending_pin_rate_edits.push((
                                    node.id,
                                    PinDirection::Output,
                                    idx,
                                    buf.clone(),
                                ));
                                log::info!("[UI] queued edit: node={} dir=Output idx={} -> {}", node.id, idx, buf);
                            } else {
                                log::debug!("[UI][debug] edit_buffers missing for key {}", key);
                            }
                        }

                        // Inline success indicator (green dot) if this pin had a recent successful edit
                        if self.is_pin_recent_success(node.id, PinDirection::Output, idx) {
                            let dot_center = egui::pos2(response.rect.right() + 8.0, response.rect.center().y);
                            ui.painter().circle_filled(dot_center, 4.0, egui::Color32::from_rgb(80, 200, 120));
                        }
                    }

                    // For merger/splitter nodes we intentionally hide per-pin icons and labels
                    if node.node_type != NodeType::Merger
                        && node.node_type != NodeType::CustomSplitter
                        && node.node_type != NodeType::GameSplitter
                    {
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
            || node.node_type == NodeType::CustomSplitter
            || node.node_type == NodeType::GameSplitter
            || node.node_type == NodeType::Merger
            || node.node_type == NodeType::Sink
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
                if !power_value.is_empty() || !node.building_name.is_empty() {
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
                                            let node_locked = self.ui_locked_nodes.contains(&node.id);
                                            let r = self.render_fractional_input(
                                                ui,
                                                &key,
                                                &mut tmp,
                                                center_field_width,
                                                false || node_locked,
                                            );
                                            // Commit building count edits on Enter (focused/hovered/typed-diff) or lost-focus+changed
                                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                                            if enter_pressed || r.lost_focus() || r.changed() || r.has_focus() {
                                                log::debug!("[UI][debug] key={} enter={} has_focus={} changed={} lost_focus={}", key, enter_pressed, r.has_focus(), r.changed(), r.lost_focus());
                                            }
                                            let buf_opt = self.edit_buffers.get(&key).cloned();
                                            let displayed_str = node.building_count_str.clone();
                                            let typed_differs = match &buf_opt {
                                                Some(b) => b != &displayed_str,
                                                None => false,
                                            };
                                            let submit = (enter_pressed && (r.has_focus() || r.hovered() || typed_differs)) || (r.lost_focus() && r.changed());
                                            if submit {
                                                if let Some(buf) = self.edit_buffers.get(&key) {
                                                    self.pending_node_building_edits.push((node.id, buf.clone()));
                                                    log::info!("[UI] queued building edit: node={} -> {}", node.id, buf);
                                                } else {
                                                    log::debug!("[UI][debug] edit_buffers missing for key {}", key);
                                                }
                                            }
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
                                                    let node_locked = self.ui_locked_nodes.contains(&node.id);
                                                    let resp = self.render_fractional_input(
                                                        ui,
                                                        &key,
                                                        &mut tmp,
                                                        somersloop_width,
                                                        is_locked || node_locked,
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
                                if node.node_type == NodeType::Merger {
                                    self.render_footer_add_button_middle(
                                        ui,
                                        &node,
                                        PinDirection::Input,
                                    );
                                } else if node.node_type == NodeType::CustomSplitter
                                    || node.node_type == NodeType::GameSplitter
                                {
                                    self.render_footer_add_button_middle(
                                        ui,
                                        &node,
                                        PinDirection::Output,
                                    );
                                }
                                ui.end_row();
                            });
                    });
                } else if !node.sink_points_str.is_empty() {
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
                                    let text_edit = egui::TextEdit::singleline(&mut points_str)
                                        .desired_width(44.0);
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
                    if node.node_type == NodeType::Merger
                        || node.node_type == NodeType::CustomSplitter
                        || node.node_type == NodeType::GameSplitter
                    {
                        // Render a three-column grid so we can center the item_type if present and still show + in the side column
                        egui::Grid::new(format!("footer_fallback_grid:{}", node.id))
                            .num_columns(3)
                            .spacing([8.0, 8.0])
                            .min_col_width(ui.available_width() / 3.0)
                            .show(ui, |ui| {
                                // Left column (input + for mergers)
                                if node.node_type == NodeType::Merger {
                                    ui.horizontal(|ui| {
                                        ui.add_space(Self::FOOTER_ADD_INSET);
                                        if ui
                                            .add(
                                                egui::Button::new("+")
                                                    .corner_radius(egui::CornerRadius::same(0))
                                                    .small(),
                                            )
                                            .clicked()
                                        {
                                            self.pending_pin_adds
                                                .push((node.id, PinDirection::Input));
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
                                if node.node_type != NodeType::Merger {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.add_space(Self::FOOTER_ADD_INSET);
                                            if ui
                                                .add(
                                                    egui::Button::new("+")
                                                        .corner_radius(egui::CornerRadius::same(0))
                                                        .small(),
                                                )
                                                .clicked()
                                            {
                                                self.pending_pin_adds
                                                    .push((node.id, PinDirection::Output));
                                            }
                                        },
                                    );
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

    fn has_node_menu(&mut self, _node: &EditorNode) -> bool {
        // Enable node menu (used on RMB). We'll intercept the menu action in `show_node_menu` to toggle visual lock and immediately close
        true
    }

    fn show_node_menu(
        &mut self,
        node: egui_snarl::NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) {
        // Toggle visual lock for this node and close menu immediately
        if let Some(node_ref) = snarl.get_node(node) {
            let nid = node_ref.id;
            let was_locked = self.ui_locked_nodes.contains(&nid);
            if was_locked {
                self.ui_locked_nodes.remove(&nid);
                log::info!("[UI] node {} unlocked (visual) via RMB menu", nid);
                // Request core unlock for connected component
                self.pending_node_lock_changes.push((nid, false));
            } else {
                self.ui_locked_nodes.insert(nid);
                log::info!("[UI] node {} locked (visual) via RMB menu", nid);
                // Request core lock for connected component
                self.pending_node_lock_changes.push((nid, true));
            }
        }
        // Close menu so nothing else is shown
        ui.close();
    }

    fn has_graph_menu(
        &mut self,
        _pos: egui::Pos2,
        _snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) -> bool {
        true
    }

    fn has_dropped_wire_menu(
        &mut self,
        _pins: egui_snarl::ui::AnyPins<'_>,
        _snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) -> bool {
        // Allow any dropped wire to show the menu
        true
    }

    fn show_dropped_wire_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut egui::Ui,
        src_pins: egui_snarl::ui::AnyPins<'_>,
        _snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) {
        // Determine item type from source pins (if possible) so we can pre-filter recipes
        let mut detected_item: Option<String> = None;
        // If the detected item came from a node's configured item type (organizer), prefer filtering by outputs
        let mut detected_from_node_item: bool = false;
        let (outs, ins) = match src_pins {
            egui_snarl::ui::AnyPins::Out(outs) => {
                // If there is at least one out pin, try to detect an item from that pin or the node's item_type
                if !outs.is_empty() {
                    let out = outs[0];
                    if let Some(node) = _snarl.get_node(out.node) {
                        if let Some(Some(name)) = node.output_names.get(out.output) {
                            detected_item = Some(name.clone());
                        } else if let Some(name) = node.item_type.as_ref() {
                            detected_item = Some(name.clone());
                            detected_from_node_item = true;
                        }
                    }
                }
                (Some(outs.to_vec()), None)
            }
            egui_snarl::ui::AnyPins::In(ins) => {
                // If single in pin and it has a named input, use it
                if ins.len() == 1 {
                    if let Some(node) = _snarl.get_node(ins[0].node) {
                        if let Some(Some(name)) = node.input_names.get(ins[0].input) {
                            detected_item = Some(name.clone());
                        }
                    }
                }
                (None, Some(ins.to_vec()))
            }
        };

        // Decide whether to filter candidate recipes by outputs or inputs.
        // By default, starting from an input pin means we filter by outputs (producer)
        // But if the detected item came from a node's configured item_type (organizer output),
        // treat it as a request to find recipes that *produce* that item (filter by outputs).
        let filter_by_output = ins.is_some() || detected_from_node_item;
        if let Some(choice) =
            self.draw_add_node_menu_contents(pos, ui, detected_item.as_deref(), filter_by_output)
        {
            self.pending_dropped_wire = Some(PendingDroppedWire {
                pos,
                src_outs: outs,
                src_ins: ins,
                src_item_name: detected_item,
                choice,
            });
            ui.close();
        }
    }

    fn show_graph_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut egui::Ui,
        _snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) {
        if let Some(choice) = self.draw_add_node_menu_contents(pos, ui, None, false) {
            self.pending_dropped_wire = Some(PendingDroppedWire {
                pos,
                src_outs: None,
                src_ins: None,
                src_item_name: None,
                choice,
            });
            ui.close();
        }
    }

    fn connect(
        &mut self,
        from: &egui_snarl::OutPin,
        to: &egui_snarl::InPin,
        snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) {
        // Lookup the output and input names (if any) from the corresponding nodes
        let out_name = snarl
            .get_node(from.id.node)
            .and_then(|n| n.output_names.get(from.id.output))
            .and_then(|opt| opt.clone());
        let in_name = snarl
            .get_node(to.id.node)
            .and_then(|n| n.input_names.get(to.id.input))
            .and_then(|opt| opt.clone());

        // Debug: log the attempted connection and the current item types on both pins
        println!(
            "SnarlViewer: connect attempt from {:?} (out_name={:?}) -> {:?} (in_name={:?})",
            from.id, out_name, to.id, in_name
        );

        // If both pins have an associated item name and they differ, consider rejection.
        // However, if the target input already has existing remotes we allow replacing them (and thus changing the type).
        if let (Some(outn), Some(inn)) = (out_name.clone(), in_name.clone()) {
            if outn != inn {
                let in_has_remotes = !snarl.in_pin(to.id).remotes.is_empty();
                if in_has_remotes {
                    println!(
                        "SnarlViewer: types differ ('{}' != '{}') but input has existing remotes — will replace them",
                        outn, inn
                    );
                } else {
                    let msg = format!(
                        "Cannot connect different item types: '{}' -> '{}'",
                        outn, inn
                    );
                    println!("{}", msg);
                    self.rejected_connection_reason = Some(msg);
                    return;
                }
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
                // record pending disconnect so TemplateApp can delete production link
                self.pending_disconnects.push((from.id, r));
            }
        }
        let mut in_replacements = 0usize;
        for r in in_remotes.clone() {
            if r != from.id {
                in_replacements += 1;
                affected_nodes.insert(r.node);
                let _ = snarl.disconnect(r, to.id);
                // record pending disconnect so TemplateApp can delete production link
                self.pending_disconnects.push((r, to.id));
            }
        }

        if out_replacements + in_replacements > 0 {
            println!(
                "Replaced {} existing connection(s)",
                out_replacements + in_replacements
            );
        }

        // Finally perform the new connection
        let _ = snarl.connect(from.id, to.id);
        // record pending connection for TemplateApp to create production link & run propagation
        self.pending_connections.push((from.id, to.id));

        // Sync pin-type assignment/removal for the affected nodes and the endpoints
        affected_nodes.insert(from.id.node);
        affected_nodes.insert(to.id.node);
        for nid in affected_nodes {
            self.sync_merger_splitter(snarl, nid);
        }
    }

    /// Called when the user explicitly disconnects a single wire (e.g., right-click a hovered wire)
    fn disconnect(&mut self, from: &egui_snarl::OutPin, to: &egui_snarl::InPin, snarl: &mut egui_snarl::Snarl<EditorNode>) {
        // Record the disconnect for the TemplateApp to process production model changes
        self.pending_disconnects.push((from.id, to.id));
        log::debug!("[UI] queued pending_disconnect: out={:?} in={:?} (disconnect)", from.id, to.id);
        // Also perform the visual disconnect so UI stays in sync
        snarl.disconnect(from.id, to.id);
    }

    /// Called when user requests dropping all outputs (right-click on an output pin)
    fn drop_outputs(&mut self, pin: &egui_snarl::OutPin, snarl: &mut egui_snarl::Snarl<EditorNode>) {
        // enqueue each removed wire
        let remotes = pin.remotes.clone();
        for inp in remotes {
            self.pending_disconnects.push((pin.id, inp));
            log::debug!("[UI] queued pending_disconnect: out={:?} in={:?} (drop_outputs)", pin.id, inp);
        }
        // perform the actual removal
        snarl.drop_outputs(pin.id);
    }

    /// Called when user requests dropping all inputs (right-click on an input pin)
    fn drop_inputs(&mut self, pin: &egui_snarl::InPin, snarl: &mut egui_snarl::Snarl<EditorNode>) {
        // enqueue each removed wire
        let remotes = pin.remotes.clone();
        for outp in remotes {
            self.pending_disconnects.push((outp, pin.id));
            log::debug!("[UI] queued pending_disconnect: out={:?} in={:?} (drop_inputs)", outp, pin.id);
        }
        // perform the actual removal
        snarl.drop_inputs(pin.id);
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
    show_save_suggestions: bool,
    build_progress_open: bool,

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

    // Controls popup: whether it's shown and whether it was just opened (ignore input that opened it)
    #[serde(skip)]
    show_controls_popup: bool,
    #[serde(skip)]
    controls_popup_just_opened: bool,

    // Application settings
    settings: Settings,
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
                        log::error!("✗ Failed to load game data: {}", e);
                    }
                }
            } else {
                log::warn!("✗ Warning: Could not read assets/satisfactory.json");
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            // For web, we'll need to load this differently (fetch API, etc.)
            log::warn!("Web platform: game data loading not yet implemented");
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
            show_save_suggestions: false,
            show_controls_popup: false,
            controls_popup_just_opened: false,
            show_recipe_selector: false,
            selected_recipe: None,
            recipe_search: String::new(),
            error_message: String::new(),
            error_time: 0.0,
            context_menu_recipe_filter: String::new(),
            build_progress_open: false,

            item_icon_cache: std::collections::HashMap::new(),
            settings: Settings::new(),
        };

        // Don't add demo nodes if game data loaded successfully
        if app.game_data.recipes.is_empty() {
            let n = app.build_editor_node(1, "Craft Node A", NodeType::Craft);
            app.snarl.insert_node(egui::pos2(0.0, 0.0), n);
            let n = app.build_editor_node(2, "Sink Node B", NodeType::Sink);
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

        // Populate save file list
        app.list_save_files();

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
                            log::warn!("Failed to decode somersloop icon {}: {}", somersloop_path, e);
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
                            log::warn!("Failed to decode icon {}: {}", path, e);
                        }
                    },
                    Err(e) => {
                        log::warn!("Failed to open icon {}: {}", path, e);
                    }
                }
            }
            log::info!("Loaded {} item icons into cache", self.item_icon_cache.len());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Web loading requires fetching assets; skip for now
            log::warn!("Web: item texture loading not implemented");
        }
    }

    /// Build an EditorNode from production model (fill pin names and icons)
    fn build_editor_node(
        &self,
        node_id: u64,
        label: impl Into<String>,
        node_type: impl Into<NodeType>,
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
        if editor_node.node_type == NodeType::Sink {
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

    /// Populate `file_suggestions` from the local saves directory (desktop only).
    fn list_save_files(&mut self) {
        self.file_suggestions.clear();
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::fs;
            use std::path::Path;
            let save_dir = Path::new("saves");
            if save_dir.exists() && save_dir.is_dir() {
                if let Ok(entries) = fs::read_dir(save_dir) {
                    for e in entries.flatten() {
                        let path = e.path();
                        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                            if ext.eq_ignore_ascii_case("fcs") {
                                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                    self.file_suggestions.push((stem.to_owned(), false));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Rebuild the UI snarl from the current `production_app` state after loading.
    fn rebuild_snarl_from_production(&mut self) {
        // Create a fresh snarl and map production node ids -> ui node ids
        self.snarl = egui_snarl::Snarl::new();
        // Re-populate viewer icon map so nodes render icons immediately
        self.snarl_viewer.set_icon_map(self.item_icon_cache.iter().map(|(k, h)| (k.clone(), h.id())).collect());

        let mut node_map: std::collections::HashMap<u64, egui_snarl::NodeId> = std::collections::HashMap::new();

        for node_any in &self.production_app.nodes {
            // Craft
            if let Some(craft) = node_any.downcast_ref::<crate::node::CraftNode>() {
                let node_id = craft.base.id;
                let label = self
                    .game_data
                    .recipes
                    .iter()
                    .find(|r| r.name == craft.recipe_name)
                    .map(|r| r.display_name.clone())
                    .unwrap_or_else(|| craft.recipe_name.clone());
                let en = self.build_editor_node(node_id, label, NodeType::Craft);
                let pos = egui::pos2(craft.base.position.0, craft.base.position.1);
                let ui_node = self.snarl.insert_node(pos, en);
                node_map.insert(node_id, ui_node);
            }
            // Organizer nodes (splitters / merger)
            else if let Some(org) = node_any.downcast_ref::<crate::node::OrganizerNode>() {
                let node_id = org.base.id;
                let (label, node_type) = match org.base.kind {
                    crate::node::NodeKind::Merger => ("Merger".to_owned(), NodeType::Merger),
                    crate::node::NodeKind::CustomSplitter => ("Splitter*".to_owned(), NodeType::CustomSplitter),
                    crate::node::NodeKind::GameSplitter => ("Splitter".to_owned(), NodeType::GameSplitter),
                    _ => ("Organizer".to_owned(), NodeType::Group),
                };
                let en = self.build_editor_node(node_id, label, node_type);
                let pos = egui::pos2(org.base.position.0, org.base.position.1);
                let ui_node = self.snarl.insert_node(pos, en);
                node_map.insert(node_id, ui_node);
            }
            // Group
            else if let Some(group) = node_any.downcast_ref::<crate::node::GroupNode>() {
                let node_id = group.base.id;
                let en = self.build_editor_node(node_id, format!("Group {}", node_id), NodeType::Group);
                let pos = egui::pos2(group.base.position.0, group.base.position.1);
                let ui_node = self.snarl.insert_node(pos, en);
                node_map.insert(node_id, ui_node);
            }
            // Sink
            else if let Some(sink) = node_any.downcast_ref::<crate::node::SinkNode>() {
                let node_id = sink.base.id;
                let en = self.build_editor_node(node_id, "Sink", NodeType::Sink);
                let pos = egui::pos2(sink.base.position.0, sink.base.position.1);
                let ui_node = self.snarl.insert_node(pos, en);
                node_map.insert(node_id, ui_node);
            }
        }

        // Connect links (use production_app.find_pin_location to map pin ids -> node/pin idx)
        for link in &self.production_app.links {
            if let Some((start_node, start_dir, start_idx)) = self.production_app.find_pin_location(link.start_pin_id) {
                if let Some((end_node, end_dir, end_idx)) = self.production_app.find_pin_location(link.end_pin_id) {
                    // Determine out/input ends
                    let (out_node, out_idx, in_node, in_idx) = if start_dir == crate::pin::PinDirection::Output && end_dir == crate::pin::PinDirection::Input {
                        (start_node, start_idx, end_node, end_idx)
                    } else if start_dir == crate::pin::PinDirection::Input && end_dir == crate::pin::PinDirection::Output {
                        (end_node, end_idx, start_node, start_idx)
                    } else {
                        continue; // unsupported
                    };

                    if let (Some(&ui_out), Some(&ui_in)) = (node_map.get(&out_node), node_map.get(&in_node)) {
                        let out_pin = egui_snarl::OutPinId { node: ui_out, output: out_idx };
                        let in_pin = egui_snarl::InPinId { node: ui_in, input: in_idx };
                        let _ = self.snarl.connect(out_pin, in_pin);

                        // Keep types in sync for organizers (use UI node ids)
                        self.snarl_viewer.sync_merger_splitter(&mut self.snarl, ui_out);
                        self.snarl_viewer.sync_merger_splitter(&mut self.snarl, ui_in);
                    }
                }
            }
        }
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

        self.show_left_panel(ctx, frame);
        self.show_central_panel(ctx);
        self.show_dialogs(ctx);
    }
}

impl TemplateApp {

    fn separator_text_left(&self, ui: &mut egui::Ui, text: &str) {
        // Draw a separator whose text appears close to the left edge (similar to ImGui::SeparatorText)
        // Add a short visible line at the very left to match ImGui's visual style.
        let text_height = ui.text_style_height(&egui::TextStyle::Body);
        let height = text_height + 8.0;
        let available = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(available, height), egui::Sense::hover());
        let painter = ui.painter();
        let y = rect.center().y;

        let color = ui.visuals().text_color();
        let stroke = egui::Stroke::new(1.0, color.linear_multiply(0.8));

        let galley = painter.layout_no_wrap(text.to_owned(), egui::FontId::default(), color);
        let text_w = galley.size().x;

        // Fixed small left segment like ImGui's SeparatorText beginning
        let left_short_len = 20.0;
        let pad = 6.0;
        // Position text after the small left segment + padding
        let mut text_x = rect.left() + left_short_len + pad;
        // Ensure text doesn't run off the right edge
        if text_x + text_w + pad > rect.right() {
            text_x = (rect.right() - text_w - pad).max(rect.left() + pad);
        }
        let gap = 8.0;

        // Draw the fixed short left segment
        let short_end = (rect.left() + left_short_len).min(rect.right());
        if short_end > rect.left() {
            painter.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(short_end, y)],
                stroke,
            );
        }

        // Right line after text (fills the rest)
        let right_line_start = (text_x + text_w + gap).min(rect.right());
        if right_line_start < rect.right() {
            painter.line_segment(
                [egui::pos2(right_line_start, y), egui::pos2(rect.right(), y)],
                stroke,
            );
        }

        painter.text(
            egui::pos2(text_x, y),
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::default(),
            color,
        );
    }

    fn show_left_panel(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.settings.left_panel_folded {
            egui::SidePanel::left("left_panel_collapsed")
                .resizable(false)
                .width_range(40.0..=40.0)
                .show(ctx, |ui| {
                    let response = ui.button(">>");
                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if response.clicked() {
                        self.settings.left_panel_folded = false;
                    }
                });
        } else {
            egui::SidePanel::left("left_panel")
                .resizable(false)
                .default_width(250.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        // Controls popup
                                        if ui.button("Show controls list").clicked() {
                                self.show_controls_popup = true;
                                self.controls_popup_just_opened = true;
                            }

                        let is_web = cfg!(target_arch = "wasm32");

                        if is_web{ 
                            if ui.button("Export").on_hover_text("Export current production chain to disk").clicked() {
                                 // !TODO: Implement export functionality
                                 //const std::string path = "production_chain.fcs";
                                self.error_message = "Export not implemented yet".to_owned();
                                self.error_time = 3.0;
                            }
                            if ui.button("Import").on_hover_text("Import a production chain from disk").clicked() {
                                // !TODO: Implement import functionality
                                //waitForFileInput();
                                //if (std::filesystem::exists("_internal_load_file"))
                                self.error_message = "Import not implemented yet".to_owned();
                                self.error_time = 3.0;
                            }
                        }

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                // Collapse button
                                if ui.button("<<").on_hover_text("Fold left panel").clicked() {
                                    self.left_panel_collapsed = true;
                                }
                            });
                    });
            
                    // Save/Load section
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let save_resp = ui.text_edit_singleline(&mut self.save_name)
                                .on_hover_text("Name to save/load...");

                            // Update suggestions and show popup when the box is focused/changed
                            if save_resp.has_focus() || save_resp.changed() {
                                self.list_save_files();
                                self.show_save_suggestions = true;
                            }

                            // Show native popup below the text box with file suggestions
                            {
                            // Collect user interactions and apply them after the popup to avoid multiple mutable borrows
                            let mut selected: Option<String> = None;
                            let mut to_delete: Option<String> = None;

                            egui::containers::Popup::from_response(&save_resp)
                                .open_bool(&mut self.show_save_suggestions)
                                .show(|popup_ui| {
                                    use egui::ScrollArea;
                                    let max_items = 10usize;
                                    ScrollArea::vertical()
                                        .max_height((max_items as f32) * 24.0)
                                        .show(popup_ui, |ui| {
                                            for (name, _) in &self.file_suggestions {
                                                ui.horizontal(|ui| {
                                                    if ui.small_button("x").clicked() {
                                                        to_delete = Some(name.clone());
                                                    }
                                                    if ui.button(name).clicked() {
                                                        selected = Some(name.clone());
                                                    }
                                                });
                                            }
                                        });
                                });

                            if let Some(s) = selected {
                                self.save_name = s;
                                self.show_save_suggestions = false;
                            }
                            if let Some(d) = to_delete {
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    let _ = std::fs::remove_file(format!("saves/{}.fcs", d));
                                }
                                self.list_save_files();
                            }
                        }

                        
                            if ui.button("Save").on_hover_text("Save current production chain").clicked() {
                                if !self.save_name.is_empty() {
                                    // Sync UI node positions back into the production model so saves capture current layout
                                    for node_info in self.snarl.nodes_info() {
                                        let id = node_info.value.id;
                                        let center = node_info.pos;
                                        let _ = self.production_app.set_node_position(id, (center.x, center.y));
                                    }

                                    match self.production_app.save_to_json() {
                                        Ok(json) => {
                                            #[cfg(not(target_arch = "wasm32"))]
                                            {
                                                use std::fs;
                                                use std::path::Path;
                                                let save_dir = Path::new("saves");
                                                if let Err(e) = fs::create_dir_all(&save_dir) {
                                                    self.error_message = format!("Save error: {}", e);
                                                    self.error_time = 3.0;
                                                } else {
                                                    let path = save_dir.join(format!("{}.fcs", self.save_name));
                                                    match fs::write(&path, json) {
                                                        Ok(()) => {
                                                            self.error_message = format!("Saved: {}", self.save_name);
                                                            self.error_time = 3.0;
                                                            self.list_save_files();
                                                        }
                                                        Err(e) => {
                                                            self.error_message = format!("Save error: {}", e);
                                                            self.error_time = 3.0;
                                                        }
                                                    }
                                                }
                                            }
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                self.error_message = "Save not implemented for web".to_owned();
                                                self.error_time = 3.0;
                                            }
                                        }
                                        Err(e) => {
                                            self.error_message = format!("Save error: {}", e);
                                            self.error_time = 3.0;
                                        }
                                    }
                                }
                            }

                            // Enable Load only when file exists (desktop); web not implemented
                            let mut load_enabled = false;
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                if !self.save_name.is_empty() {
                                    load_enabled = std::path::Path::new(&format!("saves/{}.fcs", self.save_name)).exists();
                                }
                            }
                            #[cfg(target_arch = "wasm32")]
                            {
                                load_enabled = false;
                            }

                            let load_resp = ui.add_enabled(load_enabled, egui::Button::new("Load")).on_hover_text("Load a production chain");
                            if load_resp.clicked() {
                                if !self.save_name.is_empty() {
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        let path = format!("saves/{}.fcs", self.save_name);
                                        match std::fs::read_to_string(&path) {
                                            Ok(content) => match self.production_app.load_from_json(&content, Some(&self.game_data)) {
                                                Ok(()) => {
                                                    // Rebuild UI from production model
                                                    self.rebuild_snarl_from_production();
                                                    self.error_message = format!("Loaded: {}", self.save_name);
                                                    self.error_time = 2.0;
                                                }
                                                Err(e) => {
                                                    self.error_message = format!("Load error: {}", e);
                                                    self.error_time = 3.0;
                                                }
                                            },
                                            Err(e) => {
                                                self.error_message = format!("Load error: {}", e);
                                                self.error_time = 3.0;
                                            }
                                        }
                                    }
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        self.error_message = "Load not implemented for web".to_owned();
                                        self.error_time = 2.0;
                                    }
                                }
                            }
                        });
                    });

                    self.separator_text_left(ui, "Settings");
                    ui.horizontal(| ui | {
                        if ui.button("Unlock all alt recipes").clicked() {
                            let all_recipe_keys: Vec<String> = self
                                .game_data
                                .recipes
                                .iter()
                                .filter(|r| r.alternate)
                                .map(|r| r.name.clone())
                                .collect();
                            for k in all_recipe_keys {
                                self.settings.unlocked_alts.insert(k.clone(), true);
                            }
                        }
                        if ui.button("Reset alt recipes").clicked() {
                            for (_k, v) in self.settings.unlocked_alts.iter_mut() {
                                *v = false;
                            }
                        }
                    });
                    ui.checkbox(&mut self.settings.show_somersloop, "Show somersloop");
                    ui.checkbox(&mut self.settings.show_build_progress, "Show build progress").on_hover_text("If set, will add a checkmark on craft nodes and overall build progress bars");

                    if self.settings.show_build_progress {
                        self.separator_text_left(ui, "Build progress");

                        // Compute overall and per-building progress from production model
                        let mut total_machines: std::collections::HashMap<String, crate::fractional_number::FractionalNumber> = std::collections::HashMap::new();
                        let mut built_machines: std::collections::HashMap<String, crate::fractional_number::FractionalNumber> = std::collections::HashMap::new();
                        let mut all_machines = crate::fractional_number::FractionalNumber::default();
                        let mut all_built_machines = crate::fractional_number::FractionalNumber::default();

                        for node_any in &self.production_app.nodes {
                            if let Some(craft) = node_any.downcast_ref::<crate::node::CraftNode>() {
                                let bname = craft.building_name.clone();
                                total_machines
                                    .entry(bname.clone())
                                    .and_modify(|v| *v += craft.current_rate)
                                    .or_insert(craft.current_rate);
                                all_machines += craft.current_rate;
                                if craft.built {
                                    built_machines
                                        .entry(bname.clone())
                                        .and_modify(|v| *v += craft.current_rate)
                                        .or_insert(craft.current_rate);
                                    all_built_machines += craft.current_rate;
                                }
                            }
                        }

                        let overall = if all_machines.value() == 0.0 {
                            0.0f32
                        } else {
                            (all_built_machines / all_machines).value() as f32
                        };

                        // Header row with expandable toggle (animated triangle icon) and overall progress on the right
                        let id = ui.make_persistent_id("build_progress");
                        let header_result = egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            id,
                            self.build_progress_open,
                        )
                        .show_header(ui, |ui| {
                                ui.add(
                                egui::ProgressBar::new(overall)
                                    .text(format!("{:.0}%", overall * 100.0))
                                    .desired_width(120.0),
                            );
                        })
                        .body(|ui| {
                            // Show per-building progress bars (sorted by total desc)
                            let mut machines: Vec<(String, crate::fractional_number::FractionalNumber)> = total_machines
                                .into_iter()
                                .collect();
                            // sort alphabetically
                            machines.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

                            egui::Grid::new("build_progress_grid")
                                .num_columns(2)
                                .show(ui, |ui| {
                                    for (name, total) in machines.iter() {
                                        let built = built_machines.get(name).copied().unwrap_or_default();
                                        let pct = if total.value() == 0.0 { 0.0f32 } else { (built.value() / total.value()) as f32 };
                                        ui.label(name);
                                        ui.add(egui::ProgressBar::new(pct).text(format!("{:.0}%", pct * 100.0)).desired_width(ui.available_width() * 0.65));
                                        ui.end_row();
                                    }
                                });

                        });

                        // Persist open state
                        self.build_progress_open = header_result.2.is_some();
                    }

                    self.separator_text_left(ui, "Average Power Consumption");
                    
                    let resp = ui.checkbox(&mut self.power_equal_clocks, "Compute power with equal clocks");
                    if resp.changed() {
                            // Immediate update in viewer
                            self.snarl_viewer.power_equal_clocks = self.power_equal_clocks;
                        }
                        if resp.hovered() {
                            resp.on_hover_text("If set, the power will be calculated assuming all machines in a node are set at the same clock rate\nOtherwise, it will be calculated with machines at 100% and one last machine underclocked");
                        }
                    // !TODO: Show expandable power consumption with details per building type
                    self.separator_text_left(ui, "Sink Points");
                    // !TODO: Show expandable points breakdown by item type
                    self.separator_text_left(ui, "Machines");
                    // !TODO: Show expandable list of machines with recipes detailizations
                    self.separator_text_left(ui, "Inputs");
                    // !TODO: show list of all inputs
                    self.separator_text_left(ui, "Outputs");
                    // !TODO : show list of all outputs
                    self.separator_text_left(ui, "Intermediates");
                    // !TODO : show list of all intermediates
                });
        }
    }

    fn show_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {

            // Helper to connect pending dropped wire to newly inserted node's corresponding pins
            fn connect_pending_wire_to_node(app: &mut TemplateApp, pending: &PendingDroppedWire, new_node: egui_snarl::NodeId) {
                // If the dropped source was an Out pin (source->new node input), connect each out to the matching input (by item name) or corresponding index
                if let Some(outs) = pending.src_outs.as_ref() {
                    for out in outs.iter() {
                        // Lookup the new node to inspect its input names
                        let node_ref = match app.snarl.get_node(new_node) {
                            Some(n) => n,
                            None => continue,
                        };
                        let input_count = node_ref.input_names.len();
                        if input_count == 0 {
                            continue; // nothing to connect to
                        }

                        // Prefer to match by item name if the dropped wire had a detected item
                        let dest_idx = if let Some(ref item_name) = pending.src_item_name {
                            node_ref
                                .input_names
                                .iter()
                                .position(|opt| opt.as_ref().map(|s| s == item_name).unwrap_or(false))
                                .unwrap_or_else(|| if out.output < input_count { out.output } else { input_count - 1 })
                        } else {
                            if out.output < input_count { out.output } else { input_count - 1 }
                        };

                        let dest = egui_snarl::InPinId { node: new_node, input: dest_idx };

                        // Disconnect existing remotes on the out pin (except the same pair)
                        let out_remotes = app.snarl.out_pin(*out).remotes.clone();
                        let mut affected_nodes = std::collections::HashSet::new();
                        for r in out_remotes.iter() {
                            if *r != dest {
                                affected_nodes.insert(r.node);
                                let _ = app.snarl.disconnect(*out, *r);
                                // Ensure production link removal gets scheduled
                                app.snarl_viewer.pending_disconnects.push((*out, *r));
                                log::debug!("[UI] queued pending_disconnect: out={:?} in={:?}", out, r);
                            }
                        }

                        // Disconnect existing remotes on the dest input (except the same pair)
                        let in_remotes = app.snarl.in_pin(dest).remotes.clone();
                        for r in in_remotes.iter() {
                            if *r != *out {
                                affected_nodes.insert(r.node);
                                let _ = app.snarl.disconnect(*r, dest);
                                // Ensure production link removal gets scheduled
                                app.snarl_viewer.pending_disconnects.push((*r, dest));
                                log::debug!("[UI] queued pending_disconnect: out={:?} in={:?}", r, dest);
                            }
                        }

                        // Finally connect (visual)
                        let _ = app.snarl.connect(*out, dest);
                        // And schedule production link creation so core model is updated
                        app.snarl_viewer.pending_connections.push((*out, dest));
                        log::debug!("[UI] queued pending_connection: out={:?} in={:?}", out, dest);

                        // Sync affected nodes and endpoints
                        affected_nodes.insert(out.node);
                        affected_nodes.insert(dest.node);
                        for nid in affected_nodes {
                            app.snarl_viewer.sync_merger_splitter(&mut app.snarl, nid);
                        }
                    }
                }

                // If the dropped source was an In pin (new node output->existing inputs), connect the new node output that matches the input's item type (or corresponding index)
                if let Some(ins) = pending.src_ins.as_ref() {
                    for inp in ins.iter() {
                        // Lookup the new node to inspect its output names
                        let node_ref = match app.snarl.get_node(new_node) {
                            Some(n) => n,
                            None => continue,
                        };
                        let output_count = node_ref.output_names.len();
                        if output_count == 0 {
                            continue; // nothing to connect from
                        }

                        // Prefer to match by item name if the dropped wire had a detected item
                        let out_idx = if let Some(ref item_name) = pending.src_item_name {
                            node_ref
                                .output_names
                                .iter()
                                .position(|opt| opt.as_ref().map(|s| s == item_name).unwrap_or(false))
                                .unwrap_or_else(|| if inp.input < output_count { inp.input } else { output_count - 1 })
                        } else {
                            if inp.input < output_count { inp.input } else { output_count - 1 }
                        };

                        let src_out = egui_snarl::OutPinId { node: new_node, output: out_idx };

                        // Disconnect existing remotes on the new node output (except same pair)
                        let out_remotes = app.snarl.out_pin(src_out).remotes.clone();
                        let mut affected_nodes = std::collections::HashSet::new();
                        for r in out_remotes.iter() {
                            if *r != *inp {
                                affected_nodes.insert(r.node);
                                let _ = app.snarl.disconnect(src_out, *r);
                            }
                        }

                        // Disconnect existing remotes on the destination input
                        let in_remotes = app.snarl.in_pin(*inp).remotes.clone();
                        for r in in_remotes.iter() {
                            if *r != src_out {
                                affected_nodes.insert(r.node);
                                let _ = app.snarl.disconnect(*r, *inp);
                            }
                        }

                        // Connect
                        let _ = app.snarl.connect(src_out, *inp);

                        // Sync affected nodes and endpoints
                        affected_nodes.insert(src_out.node);
                        affected_nodes.insert(inp.node);
                        for nid in affected_nodes {
                            app.snarl_viewer.sync_merger_splitter(&mut app.snarl, nid);
                        }
                    }
                }

                // Sync the newly created node as well
                app.snarl_viewer.sync_merger_splitter(&mut app.snarl, new_node);
            }


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

            // Node editor (direct snarl widget so it receives events for selection and dragging)
            // Provide the viewer a name -> TextureId map so it can resolve icons during connect/sync
            self.snarl_viewer.set_icon_map(self.item_icon_cache.iter().map(|(k, h)| (k.clone(), h.id())).collect());

            // Sync recipe list and menu state into the viewer so the graph menu can show recipes
            self.snarl_viewer.recipes = self.game_data.recipes.clone();
            self.snarl_viewer.context_menu_recipe_filter = self.context_menu_recipe_filter.clone();
            self.snarl_viewer.recipe_checkbox_state = self.settings.unlocked_alts.clone();

            let snarl_response = egui_snarl::ui::SnarlWidget::new()
                .id(egui::Id::new("production-snarl"))
                .style(self.snarl_style)
                .show(&mut self.snarl, &mut self.snarl_viewer, ui);

            // After rendering, copy back any menu/filter state and process pending add-node actions
            self.context_menu_recipe_filter = self.snarl_viewer.context_menu_recipe_filter.clone();
            self.settings.unlocked_alts = self.snarl_viewer.recipe_checkbox_state.clone();

            if let Some(pending) = self.snarl_viewer.drain_pending_dropped_wire() {
                match pending.choice {
                    DroppedWireChoice::Merger => {
                        let node_id = self.production_app.add_merger_node();
                        let en = self.build_editor_node(node_id, "Merger", NodeType::Merger);
                        let new_ui_node = self.snarl.insert_node(pending.pos, en);
                        // Connect dropped wire to first input if present
                        connect_pending_wire_to_node(self, &pending, new_ui_node);
                        self.error_message = "Created Merger".to_string();
                        self.error_time = 2.0;
                    }
                    DroppedWireChoice::CustomSplitter => {
                        let node_id = self.production_app.add_custom_splitter_node();
                        let en = self.build_editor_node(node_id, "Splitter*", NodeType::CustomSplitter);
                        let new_ui_node = self.snarl.insert_node(pending.pos, en);
                        connect_pending_wire_to_node(self, &pending, new_ui_node);
                        self.error_message = "Created Custom Splitter".to_string();
                        self.error_time = 2.0;
                    }
                    DroppedWireChoice::GameSplitter => {
                        let node_id = self.production_app.add_game_splitter_node();
                        let en = self.build_editor_node(node_id, "Splitter", NodeType::GameSplitter);
                        let new_ui_node = self.snarl.insert_node(pending.pos, en);
                        connect_pending_wire_to_node(self, &pending, new_ui_node);
                        self.error_message = "Created Game Splitter".to_string();
                        self.error_time = 2.0;
                    }
                    DroppedWireChoice::Sink => {
                        let node_id = self.production_app.add_sink_node();
                        let en = self.build_editor_node(node_id, "Sink", NodeType::Sink);
                        let new_ui_node = self.snarl.insert_node(pending.pos, en);
                        connect_pending_wire_to_node(self, &pending, new_ui_node);
                        self.error_message = "Created Sink".to_string();
                        self.error_time = 2.0;
                    }
                    DroppedWireChoice::Craft(ref opt_name) => {
                        if let Some(recipe_name) = opt_name {
                            match self.production_app.add_craft_node(&recipe_name, &self.game_data) {
                                Ok(node_id) => {
                                    // Try to use display_name if available, otherwise raw name
                                    let display = self
                                        .game_data
                                        .recipes
                                        .iter()
                                        .find(|r| r.name == *recipe_name)
                                        .map(|r| r.display_name.clone())
                                        .unwrap_or(recipe_name.clone());
                                    let en = self.build_editor_node(node_id, display, NodeType::Craft);
                                    let new_ui_node = self.snarl.insert_node(pending.pos, en);
                                    connect_pending_wire_to_node(self, &pending, new_ui_node);
                                    self.error_message = format!("Created: {}", recipe_name);
                                    self.error_time = 2.0;
                                }
                                Err(e) => {
                                    self.error_message = format!("Error: {}", e);
                                    self.error_time = 3.0;
                                }
                            }
                        } else {
                            // Open the full recipe selector dialog if no recipe chosen
                            self.show_recipe_selector = true;
                        }
                    }
                }
            }
            // If the viewer rejected a connection, surface it as an error message for a short time
            if let Some(msg) = self.snarl_viewer.rejected_connection_reason.take() {
                self.error_message = msg;
                self.error_time = 3.0;
            }

            // Collect nodes that need a UI refresh after mutations
            let mut nodes_to_refresh: Vec<u64> = Vec::new();

            // Process pending pin rate edits collected by the SnarlViewer during rendering
            for (node_id, dir, idx, text) in self.snarl_viewer.drain_pending_edits() {
                match crate::fractional_number::FractionalNumber::from_string(&text) {
                    Ok(f) => {
                        if crate::rate_calculator::validate_rate(&f) {
                            log::info!("[UI] processing pending edit: node={} dir={:?} idx={} parsed={}", node_id, dir, idx, f.to_fraction_string());
                            match self.production_app.set_pin_rate(node_id, dir, idx, f) {
                                Ok(()) => {
                                    // Success feedback and refresh affected nodes (the node itself and direct neighbors)
                                    self.error_message = "Updated pin rate".to_string();
                                    self.error_time = 1.0;
                                    nodes_to_refresh.push(node_id);
                                    // Mark pin success (inline UI indicator)
                                    self.snarl_viewer.mark_pin_success(node_id, dir, idx);

                                    // Immediately update the snarl node display for this node so the user sees the change
                                    if let Some((ins, outs)) = self.production_app.get_node_pin_rates(node_id) {
                                        for node_info in self.snarl.nodes_info_mut() {
                                            if node_info.value.id == node_id {
                                                node_info.value.input_rates = ins.clone();
                                                node_info.value.output_rates = outs.clone();
                                                node_info.value.input_locked = self.production_app.get_node_pin_locked_flags(node_id).map(|(ins_locked,_)| ins_locked).unwrap_or(Vec::new());
                                                node_info.value.output_locked = self.production_app.get_node_pin_locked_flags(node_id).map(|(_,outs_locked)| outs_locked).unwrap_or(Vec::new());

                                                // Also update building count and power info immediately so UI reflects changes without a full rebuild
                                                if let Some((count_str, _)) = self.production_app.get_node_building_info(node_id) {
                                                    node_info.value.building_count_str = count_str.clone();
                                                    // Update edit buffer so the footer input shows the new value immediately
                                                    self.snarl_viewer.edit_buffers.insert(format!("building:{}", node_id), count_str);
                                                }
                                                if let Some((same, last, variable)) = self.production_app.get_node_power_info(node_id) {
                                                    node_info.value.same_clock_power_str = same.clone();
                                                    node_info.value.last_underclock_power_str = last.clone();
                                                    node_info.value.variable_power = variable;
                                                    // Update displayed power buffer based on current viewer mode
                                                    let power_display = if self.snarl_viewer.power_equal_clocks { same } else { last };
                                                    self.snarl_viewer.edit_buffers.insert(format!("node:{}:power", node_id), power_display);
                                                }
                                                if let Some((num_str, somersloop_mult)) = self.production_app.get_node_somersloop_info(node_id) {
                                                    node_info.value.num_somersloop_str = num_str.clone();
                                                    node_info.value.somersloop_mult = somersloop_mult;
                                                    self.snarl_viewer.edit_buffers.insert(format!("node:{}:somersloop", node_id), num_str);
                                                }

                                                // If this is a sink node, recompute sink points for display
                                                if node_info.value.node_type == NodeType::Sink {
                                                    let mut sum = crate::fractional_number::FractionalNumber::default();
                                                    for (opt_name, opt_rate) in node_info.value.input_names.iter().zip(node_info.value.input_rates.iter()) {
                                                        if let (Some(name), Some(rate_str)) = (opt_name.as_ref(), opt_rate.as_ref()) {
                                                            if let Ok(r) = crate::fractional_number::FractionalNumber::from_string(rate_str) {
                                                                if let Some(item_rc) = self.game_data.items.get(name) {
                                                                    let pts = r * crate::fractional_number::FractionalNumber::from(item_rc.sink_value as i64);
                                                                    sum += pts;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    node_info.value.sink_points_str = sum.to_float_string();
                                                    node_info.value.sink_points_fraction_str = sum.to_fraction_string();
                                                }

                                                break;
                                            }
                                        }
                                    }

                                    // Expand refresh set to all nodes in the connected component of this node
                                    use std::collections::HashSet;
                                    let mut connected: HashSet<u64> = HashSet::new();
                                    connected.insert(node_id);
                                    let mut changed = true;
                                    while changed {
                                        changed = false;
                                        for link in &self.production_app.links {
                                            let start_node = self.production_app.find_pin_location(link.start_pin_id).map(|(n,_,_)| n);
                                            let end_node = self.production_app.find_pin_location(link.end_pin_id).map(|(n,_,_)| n);
                                            if let (Some(s), Some(e)) = (start_node, end_node) {
                                                if connected.contains(&s) && !connected.contains(&e) {
                                                    connected.insert(e);
                                                    changed = true;
                                                }
                                                if connected.contains(&e) && !connected.contains(&s) {
                                                    connected.insert(s);
                                                    changed = true;
                                                }
                                            }
                                        }
                                    }
                                    for n in connected {
                                        nodes_to_refresh.push(n);
                                    }
                                }
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

            // Process pending disconnects from the viewer and remove corresponding production links
            for (out_pin, in_pin) in self.snarl_viewer.drain_pending_disconnects() {
                if let (Some(out_node), Some(in_node)) = (self.snarl.get_node(out_pin.node), self.snarl.get_node(in_pin.node)) {
                    let out_prod = out_node.id;
                    let in_prod = in_node.id;
                    if let (Some(start_pid), Some(end_pid)) = (
                        self.production_app.get_pin_id(out_prod, PinDirection::Output, out_pin.output),
                        self.production_app.get_pin_id(in_prod, PinDirection::Input, in_pin.input),
                    ) {
                        // Find link id matching these pins
                        let maybe_link = self.production_app.links.iter().find(|l| {
                            (l.start_pin_id == start_pid && l.end_pin_id == end_pid)
                                || (l.start_pin_id == end_pid && l.end_pin_id == start_pid)
                        }).map(|l| l.id);

                        if let Some(lid) = maybe_link {
                            if let Err(e) = self.production_app.delete_link(lid) {
                                self.error_message = format!("Failed to delete link: {}", e);
                                self.error_time = 3.0;
                            } else {
                                // refresh both endpoint nodes
                                nodes_to_refresh.push(out_prod);
                                nodes_to_refresh.push(in_prod);
                            }
                        }
                    }
                }
            }

            // Process pending connections and create production links (propagate rates)
            for (out_pin, in_pin) in self.snarl_viewer.drain_pending_connections() {
                if let (Some(out_node), Some(in_node)) = (self.snarl.get_node(out_pin.node), self.snarl.get_node(in_pin.node)) {
                    let out_prod = out_node.id;
                    let in_prod = in_node.id;
                    if let (Some(start_pid), Some(end_pid)) = (
                        self.production_app.get_pin_id(out_prod, PinDirection::Output, out_pin.output),
                        self.production_app.get_pin_id(in_prod, PinDirection::Input, in_pin.input),
                    ) {
                        match self.production_app.create_link(start_pid, end_pid) {
                            Ok((_link_id, Some(warn))) => {
                                self.error_message = warn;
                                self.error_time = 4.0;
                                // still refresh both endpoints to keep UI consistent
                                nodes_to_refresh.push(out_prod);
                                nodes_to_refresh.push(in_prod);

                                // Apply same lock propagation behavior as on successful connect
                                let mut lock_sources: std::collections::HashSet<u64> = std::collections::HashSet::new();
                                if self.snarl_viewer.ui_locked_nodes.contains(&out_prod) {
                                    lock_sources.insert(out_prod);
                                }
                                if self.snarl_viewer.ui_locked_nodes.contains(&in_prod) {
                                    lock_sources.insert(in_prod);
                                }
                                if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(out_prod) {
                                    if ins_locked.iter().any(|b| *b) || outs_locked.iter().any(|b| *b) {
                                        lock_sources.insert(out_prod);
                                    }
                                }
                                if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(in_prod) {
                                    if ins_locked.iter().any(|b| *b) || outs_locked.iter().any(|b| *b) {
                                        lock_sources.insert(in_prod);
                                    }
                                }

                                if !lock_sources.is_empty() {
                                    let mut all_affected: std::collections::HashSet<u64> = std::collections::HashSet::new();
                                    for src in lock_sources {
                                        match self.production_app.set_node_locked_and_get_affected(src, true) {
                                            Ok(affected_vec) => {
                                                for a in affected_vec { all_affected.insert(a); }
                                            }
                                            Err(e) => {
                                                self.error_message = format!("Error applying lock propagation: {}", e);
                                                self.error_time = 3.0;
                                                continue;
                                            }
                                        }
                                    }

                                    if all_affected.is_empty() {
                                        all_affected.insert(out_prod);
                                        all_affected.insert(in_prod);
                                    }

                                    for nid in &all_affected {
                                        self.snarl_viewer.ui_locked_nodes.insert(*nid);
                                    }
                                    for nid in &all_affected {
                                        if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(*nid) {
                                            for node_info in self.snarl.nodes_info_mut() {
                                                if node_info.value.id == *nid {
                                                    node_info.value.input_locked = ins_locked.clone();
                                                    node_info.value.output_locked = outs_locked.clone();
                                                    break;
                                                }
                                            }
                                        }
                                        nodes_to_refresh.push(*nid);
                                    }
                                }
                            }
                            Ok((_link_id, None)) => {
                                // success; refresh both endpoint nodes
                                nodes_to_refresh.push(out_prod);
                                nodes_to_refresh.push(in_prod);

                                // If either endpoint is visually locked or has locked pins in production, propagate
                                let mut lock_sources: std::collections::HashSet<u64> = std::collections::HashSet::new();
                                if self.snarl_viewer.ui_locked_nodes.contains(&out_prod) {
                                    lock_sources.insert(out_prod);
                                }
                                if self.snarl_viewer.ui_locked_nodes.contains(&in_prod) {
                                    lock_sources.insert(in_prod);
                                }
                                // Check production locked flags on endpoints
                                if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(out_prod) {
                                    if ins_locked.iter().any(|b| *b) || outs_locked.iter().any(|b| *b) {
                                        lock_sources.insert(out_prod);
                                    }
                                }
                                if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(in_prod) {
                                    if ins_locked.iter().any(|b| *b) || outs_locked.iter().any(|b| *b) {
                                        lock_sources.insert(in_prod);
                                    }
                                }

                                if !lock_sources.is_empty() {
                                    let mut all_affected: std::collections::HashSet<u64> = std::collections::HashSet::new();
                                    for src in lock_sources {
                                        match self.production_app.set_node_locked_and_get_affected(src, true) {
                                            Ok(affected_vec) => {
                                                for a in affected_vec { all_affected.insert(a); }
                                            }
                                            Err(e) => {
                                                self.error_message = format!("Error applying lock propagation: {}", e);
                                                self.error_time = 3.0;
                                                continue;
                                            }
                                        }
                                    }

                                    if all_affected.is_empty() {
                                        all_affected.insert(out_prod);
                                        all_affected.insert(in_prod);
                                    }

                                    // Update UI visual locks and per-node locked flags
                                    for nid in &all_affected {
                                        self.snarl_viewer.ui_locked_nodes.insert(*nid);
                                    }
                                    for nid in &all_affected {
                                        if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(*nid) {
                                            for node_info in self.snarl.nodes_info_mut() {
                                                if node_info.value.id == *nid {
                                                    node_info.value.input_locked = ins_locked.clone();
                                                    node_info.value.output_locked = outs_locked.clone();
                                                    break;
                                                }
                                            }
                                        }
                                        nodes_to_refresh.push(*nid);
                                    }
                                }
                            }
                            Err(e) => {
                                self.error_message = format!("Error creating link: {}", e);
                                self.error_time = 4.0;
                            }
                        }
                    }
                }
            }


            // Process somersloop edits collected by the SnarlViewer
            for (node_id, text) in self.snarl_viewer.drain_pending_somersloop_edits() {
                match crate::fractional_number::FractionalNumber::from_string(&text) {
                    Ok(f) => {
                        // Only accept non-negative integers. Use ProductionApp to apply and validate cap
                        match self.production_app.set_node_somersloop(node_id, f) {
                            Ok(()) => {
                                // refresh node so UI shows updated somersloop multiplier
                                nodes_to_refresh.push(node_id);
                            }
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

            // Process building count edits collected by the SnarlViewer
            for (node_id, text) in self.snarl_viewer.drain_pending_building_edits() {
                match crate::fractional_number::FractionalNumber::from_string(&text) {
                    Ok(f) => {
                        if crate::rate_calculator::validate_rate(&f) {
                            log::info!("[UI] processing pending building edit: node={} parsed={}", node_id, f.to_fraction_string());
                            match self.production_app.set_node_building_count(node_id, f) {
                                Ok(()) => {
                                    self.error_message = "Updated building count".to_string();
                                    self.error_time = 1.0;
                                    nodes_to_refresh.push(node_id);

                                    // Immediately update the snarl node display for this node so the user sees the change
                                    if let Some((ins, outs)) = self.production_app.get_node_pin_rates(node_id) {
                                        for node_info in self.snarl.nodes_info_mut() {
                                            if node_info.value.id == node_id {
                                                node_info.value.input_rates = ins.clone();
                                                node_info.value.output_rates = outs.clone();
                                                node_info.value.input_locked = self.production_app.get_node_pin_locked_flags(node_id).map(|(ins_locked,_)| ins_locked).unwrap_or(Vec::new());
                                                node_info.value.output_locked = self.production_app.get_node_pin_locked_flags(node_id).map(|(_,outs_locked)| outs_locked).unwrap_or(Vec::new());

                                                // Also update building count and power info immediately so UI reflects changes without a full rebuild
                                                if let Some((count_str, _)) = self.production_app.get_node_building_info(node_id) {
                                                    node_info.value.building_count_str = count_str.clone();
                                                    self.snarl_viewer.edit_buffers.insert(format!("building:{}", node_id), count_str);
                                                }
                                                if let Some((same, last, variable)) = self.production_app.get_node_power_info(node_id) {
                                                    node_info.value.same_clock_power_str = same.clone();
                                                    node_info.value.last_underclock_power_str = last.clone();
                                                    node_info.value.variable_power = variable;
                                                    let power_display = if self.snarl_viewer.power_equal_clocks { same } else { last };
                                                    self.snarl_viewer.edit_buffers.insert(format!("node:{}:power", node_id), power_display);
                                                }
                                                if let Some((num_str, somersloop_mult)) = self.production_app.get_node_somersloop_info(node_id) {
                                                    node_info.value.num_somersloop_str = num_str.clone();
                                                    node_info.value.somersloop_mult = somersloop_mult;
                                                    self.snarl_viewer.edit_buffers.insert(format!("node:{}:somersloop", node_id), num_str);
                                                }

                                                // If this is a sink node, recompute sink points for display
                                                if node_info.value.node_type == NodeType::Sink {
                                                    let mut sum = crate::fractional_number::FractionalNumber::default();
                                                    for (opt_name, opt_rate) in node_info.value.input_names.iter().zip(node_info.value.input_rates.iter()) {
                                                        if let (Some(name), Some(rate_str)) = (opt_name.as_ref(), opt_rate.as_ref()) {
                                                            if let Ok(r) = crate::fractional_number::FractionalNumber::from_string(rate_str) {
                                                                if let Some(item_rc) = self.game_data.items.get(name) {
                                                                    let pts = r * crate::fractional_number::FractionalNumber::from(item_rc.sink_value as i64);
                                                                    sum += pts;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    node_info.value.sink_points_str = sum.to_float_string();
                                                    node_info.value.sink_points_fraction_str = sum.to_fraction_string();
                                                }

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

            // Delete selected nodes with Delete key
            if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
                let snarl_widget = egui_snarl::ui::SnarlWidget::new().id(egui::Id::new("production-snarl"));
                let selected = snarl_widget.get_selected_nodes(ui);
                if !selected.is_empty() {
                    let mut deleted = 0usize;
                    // Delete each selected node from the production model and remove it from the snarl
                    for node in selected {
                        if let Some(en) = self.snarl.get_node(node) {
                            let prod_node_id = en.id;
                            match self.production_app.delete_node(prod_node_id) {
                                Ok(()) => {
                                    // Remove node from snarl if still present
                                    if self.snarl.get_node(node).is_some() {
                                        let _ = self.snarl.remove_node(node);
                                    }
                                    deleted += 1;
                                }
                                Err(e) => {
                                    self.error_message = format!("Error: {}", e);
                                    self.error_time = 3.0;
                                    break;
                                }
                            }
                        }
                    }
                    if deleted > 0 {
                        self.error_message = format!("Deleted {} node(s)", deleted);
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

            // Process pending node lock changes requested by UI (RMB toggles)
            for (node_id, locked) in self.snarl_viewer.drain_pending_node_lock_changes() {
                // Delegate locking to core and get affected nodes back for UI sync
                match self.production_app.set_node_locked_and_get_affected(node_id, locked) {
                    Ok(affected_nodes_vec) => {
                        let affected_nodes: std::collections::HashSet<u64> = affected_nodes_vec.into_iter().collect();

                        // Debug
                        log::debug!("[UI] set_node_locked_and_get_affected: node={} locked={} affected_nodes={:?}", node_id, locked, affected_nodes);

                        // Update UI visual lock set for affected nodes
                        if locked {
                            for nid in &affected_nodes {
                                self.snarl_viewer.ui_locked_nodes.insert(*nid);
                            }
                        } else {
                            for nid in &affected_nodes {
                                self.snarl_viewer.ui_locked_nodes.remove(nid);
                            }
                        }

                        // Update per-node locked flags for affected nodes and schedule refresh
                        for nid in &affected_nodes {
                            if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(*nid) {
                                for node_info in self.snarl.nodes_info_mut() {
                                    if node_info.value.id == *nid {
                                        node_info.value.input_locked = ins_locked.clone();
                                        node_info.value.output_locked = outs_locked.clone();
                                        break;
                                    }
                                }
                            }
                            nodes_to_refresh.push(*nid);
                        }
                    }
                    Err(e) => {
                        self.error_message = format!("Error: {}", e);
                        self.error_time = 3.0;
                    }
                }
            }

            // Process pending node item type changes requested by viewer (e.g., merger/splitter selection)
            for (node_id, item_opt) in self.snarl_viewer.drain_pending_node_item_changes() {
                match self.production_app.set_node_item_name(node_id, item_opt.clone()) {
                    Ok(()) => {
                        log::debug!("[UI] applied set_node_item_name: node={} item={:?}", node_id, item_opt);
                        // Update the viewer's cached node value (preserve existing item_type_icon if present)
                        for node_info in self.snarl.nodes_info_mut() {
                            if node_info.value.id == node_id {
                                node_info.value.item_type = item_opt.clone();
                                node_info.value.item_type_icon = item_opt.as_ref().and_then(|n| self.item_icon_cache.get(n).map(|h| h.id()));
                                break;
                            }
                        }
                        // Schedule refresh so the node UI updates from production
                        nodes_to_refresh.push(node_id);
                    }
                    Err(e) => {
                        self.error_message = format!("Error: {}", e);
                        self.error_time = 3.0;
                    }
                }
            }

            // Refresh modified nodes in the snarl widget (do this after mutations to avoid borrow conflicts)
            for node_id_ref in &nodes_to_refresh {
                let node_id = *node_id_ref;
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
                            if node_info.value.node_type == NodeType::Sink {
                                let mut merged = en.clone();
                                let old = node_info.value.clone();
                                let min_inputs = old.input_names.len().min(merged.input_names.len());
                                for i in 0..min_inputs {
                                    merged.input_names[i] = old.input_names[i].clone();
                                    merged.input_icons[i] = old.input_icons[i];
                                }
                                node_info.value = merged;
                            } else if node_info.value.node_type == NodeType::Merger
                                || node_info.value.node_type == NodeType::CustomSplitter
                                || node_info.value.node_type == NodeType::GameSplitter
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

                            // Sync edit buffers so UI input/input fields reflect the new rates immediately
                            let en_ref = &node_info.value;
                            for (i, opt) in en_ref.input_rates.iter().enumerate() {
                                if let Some(rate_str) = opt {
                                    let key = format!("pin:{}:in:{}", node_id, i);
                                    self.snarl_viewer.edit_buffers.insert(key, rate_str.clone());
                                }
                            }
                            for (i, opt) in en_ref.output_rates.iter().enumerate() {
                                if let Some(rate_str) = opt {
                                    let key = format!("pin:{}:out:{}", node_id, i);
                                    self.snarl_viewer.edit_buffers.insert(key, rate_str.clone());
                                }
                            }
                            break;
                        }
                    }
                }
            }

            // Debug: report refreshed nodes and some of their values so user sees immediate feedback in terminal
            if !nodes_to_refresh.is_empty() {
                let mut parts: Vec<String> = Vec::new();
                for nid in &nodes_to_refresh {
                    if let Some((cnt, _)) = self.production_app.get_node_building_info(*nid) {
                        parts.push(format!("{}->{}", nid, cnt));
                    } else {
                        parts.push(format!("{}->n/a", nid));
                    }
                }
                log::debug!("[UI] refreshed nodes: {}", parts.join(", "));
            }
        });
    }

    fn show_dialogs(&mut self, ctx: &egui::Context) {

        // Controls popup (borderless context popup that closes on any input)
        if self.show_controls_popup {
            let controls = [
                ("Right click",         "Add node/Lock Pin"),
                ("Right click + mouse", "Move view"),
                ("Left click",          "Select node/link"),
                ("Left click + mouse",  "Move node/link"),
                ("Mouse wheel",         "Zoom/Unzoom"),
                ("Del",                 "Delete selection"),
                ("F",                   "Show selection/full graph"),
                ("Alt",                 "Disable grid snapping"),
                ("Arrows",              "Nudge selection"),
                ("Ctrl + A",            "Select all nodes"),
                ("Ctrl + D",            "Duplicate nodes"),
                ("Ctrl + G",            "Group/Ungroup nodes"),
                ("Ctrl + Left click",   "Add to selection"),
            ];

            // Position popup near the cursor when possible
            let pos = ctx
                .input(|i| i.pointer.hover_pos())
                .unwrap_or(egui::pos2(100.0, 100.0));

            let inner = egui::Window::new("🎮 Keyboard Controls")
                .open(&mut self.show_controls_popup)
                .resizable(false)
                .collapsible(false)
                .title_bar(false)
                .frame(egui::Frame::popup(&ctx.style()))
                .default_pos(pos)
                .show(ctx, |ui| {
                    egui::Grid::new("controls_grid")
                        .num_columns(2)
                        .spacing([20.0, 8.0])
                        .show(ui, |ui| {
                            for (key, action) in controls.iter() {
                                ui.label(*key);
                                ui.label(*action);
                                ui.end_row();
                            }
                        });
                });

            // Ignore the input that opened it for a single frame
            if self.controls_popup_just_opened {
                self.controls_popup_just_opened = false;
            } else {
                // Close on any key press or pointer button event, or if clicked outside
                let any_input = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Key { .. } | egui::Event::PointerButton { .. })));
                let clicked_elsewhere = inner.as_ref().map(|r| r.response.clicked_elsewhere()).unwrap_or(false);
                if any_input || clicked_elsewhere {
                    self.show_controls_popup = false;
                }
            }
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
