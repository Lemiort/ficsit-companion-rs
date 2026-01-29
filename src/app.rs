use crate::graph_node::{CraftData, GroupData, ItemData, OrganizerData, PinData, SinkData};
use crate::graph_node::{GraphNode, GraphNodeType, NodeDisplayData, PendingChange};
use crate::{FractionalNumber, production_app::ProductionApp};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub show_spoilers: bool,
    pub show_somersloop: bool,
    pub unlocked_alts: HashMap<String, bool>,
    pub power_equal_clocks: bool,
    pub show_build_progress: bool,
    pub left_panel_folded: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl Settings {
    pub fn new() -> Self {
        Self {
            show_spoilers: false,
            show_somersloop: false,
            unlocked_alts: HashMap::new(),
            power_equal_clocks: true,
            show_build_progress: false,
            left_panel_folded: false,
        }
    }
}

use crate::pin::PinDirection;
use std::{collections::HashMap, i64};

#[derive(Default, Debug)]
struct SnarlViewer {
    // Per-frame display cache: node_id -> display data
    // Rebuilt from ProductionApp each frame before rendering
    node_cache: HashMap<u64, NodeDisplayData>,

    // ID of the current node being rendered (used by show_input/show_output to look up from cache)
    current_node_id: Option<u64>,
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

    // Unified pending changes queue - replaces 10 separate queues
    pending_changes: Vec<PendingChange>,

    // Pending dropped wire action recorded by show_dropped_wire_menu / show_graph_menu
    pending_dropped_wire: Option<PendingDroppedWire>,
    // UI-only locked nodes set (visual lock toggled by right-click on node header)
    ui_locked_nodes: std::collections::HashSet<u64>,

    // Buffer for temporary locks created by auto-wiring so we can restore original state later
    temporary_locks: Vec<TemporaryLock>,

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

    // Copy of app settings for UI rendering (synced from TemplateApp)
    pub settings: Settings,
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

#[derive(Clone, Debug)]
struct TemporaryLock {
    pub node_id: u64,
    pub direction: Option<PinDirection>,
    pub pin_index: Option<usize>,
    pub is_node: bool,
    /// Snapshot of pin ids that were locked before we applied the temporary lock
    pub locked_snapshot: Vec<u64>,
    /// Nodes affected/marked visually locked when we applied the temporary lock
    pub affected_nodes: Vec<u64>,
}  

impl SnarlViewer {
    // Fixed inset before the footer '+' (used for both input and output placements)
    const FOOTER_ADD_INSET: f32 = 48.0;

    /// Update settings from TemplateApp (call this once per frame)
    fn sync_settings(&mut self, new_settings: &Settings) {
        self.settings = new_settings.clone();
    }

    /// Drain all pending changes - returns the unified changes queue
    #[allow(dead_code)]
    fn drain_changes(&mut self) -> Vec<PendingChange> {
        std::mem::take(&mut self.pending_changes)
    }

    // Compatibility methods that drain specific types from pending_changes
    // These allow the existing processing loop to work while we migrate to the unified approach

    fn drain_pending_edits(&mut self) -> Vec<(u64, PinDirection, usize, FractionalNumber)> {
        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for change in std::mem::take(&mut self.pending_changes) {
            match change {
                PendingChange::PinRate {
                    node_id,
                    direction,
                    pin_index,
                    value,
                } => {
                    result.push((node_id, direction, pin_index, value));
                }
                other => remaining.push(other),
            }
        }
        self.pending_changes = remaining;
        result
    }

    fn drain_pending_disconnects(&mut self) -> Vec<(egui_snarl::OutPinId, egui_snarl::InPinId)> {
        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for change in std::mem::take(&mut self.pending_changes) {
            match change {
                PendingChange::Disconnect { out_pin, in_pin } => {
                    result.push((out_pin, in_pin));
                }
                other => remaining.push(other),
            }
        }
        self.pending_changes = remaining;
        result
    }

    fn drain_pending_connections(&mut self) -> Vec<(egui_snarl::OutPinId, egui_snarl::InPinId)> {
        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for change in std::mem::take(&mut self.pending_changes) {
            match change {
                PendingChange::Connect { out_pin, in_pin } => {
                    result.push((out_pin, in_pin));
                }
                other => remaining.push(other),
            }
        }
        self.pending_changes = remaining;
        result
    }

    fn drain_pending_somersloop_edits(&mut self) -> Vec<(u64, FractionalNumber)> {
        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for change in std::mem::take(&mut self.pending_changes) {
            match change {
                PendingChange::NodeSomersloop { node_id, value } => {
                    result.push((node_id, value));
                }
                other => remaining.push(other),
            }
        }
        self.pending_changes = remaining;
        result
    }

    fn drain_pending_building_edits(&mut self) -> Vec<(u64, FractionalNumber)> {
        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for change in std::mem::take(&mut self.pending_changes) {
            match change {
                PendingChange::NodeBuilding { node_id, count } => {
                    result.push((node_id, count));
                }
                other => remaining.push(other),
            }
        }
        self.pending_changes = remaining;
        result
    }

    fn drain_pending_built_edits(&mut self) -> Vec<(u64, bool)> {
        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for change in std::mem::take(&mut self.pending_changes) {
            match change {
                PendingChange::NodeBuilt { node_id, built } => {
                    result.push((node_id, built));
                }
                other => remaining.push(other),
            }
        }
        self.pending_changes = remaining;
        result
    }

    fn drain_pending_pin_adds(&mut self) -> Vec<(u64, PinDirection)> {
        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for change in std::mem::take(&mut self.pending_changes) {
            match change {
                PendingChange::PinAdd { node_id, direction } => {
                    result.push((node_id, direction));
                }
                other => remaining.push(other),
            }
        }
        self.pending_changes = remaining;
        result
    }

    fn drain_pending_pin_removes(&mut self) -> Vec<(u64, PinDirection, usize)> {
        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for change in std::mem::take(&mut self.pending_changes) {
            match change {
                PendingChange::PinRemove {
                    node_id,
                    direction,
                    index,
                } => {
                    result.push((node_id, direction, index));
                }
                other => remaining.push(other),
            }
        }
        self.pending_changes = remaining;
        result
    }

    fn drain_pending_node_lock_changes(&mut self) -> Vec<(u64, bool)> {
        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for change in std::mem::take(&mut self.pending_changes) {
            match change {
                PendingChange::NodeLock { node_id, locked } => {
                    result.push((node_id, locked));
                }
                other => remaining.push(other),
            }
        }
        self.pending_changes = remaining;
        result
    }

    fn drain_pending_node_item_changes(&mut self) -> Vec<(u64, Option<String>)> {
        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for change in std::mem::take(&mut self.pending_changes) {
            match change {
                PendingChange::NodeItem { node_id, item } => {
                    result.push((node_id, item));
                }
                other => remaining.push(other),
            }
        }
        self.pending_changes = remaining;
        result
    }

    fn drain_pending_sink_pin_items(&mut self) -> Vec<(u64, usize, Option<String>)> {
        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for change in std::mem::take(&mut self.pending_changes) {
            match change {
                PendingChange::SinkPinItem { node_id, pin_idx, item } => {
                    result.push((node_id, pin_idx, item));
                }
                other => remaining.push(other),
            }
        }
        self.pending_changes = remaining;
        result
    }

    fn drain_pending_pin_lock_changes(&mut self) -> Vec<(u64, PinDirection, usize, bool)> {
        let mut result = Vec::new();
        let mut remaining = Vec::new();
        for change in std::mem::take(&mut self.pending_changes) {
            match change {
                PendingChange::PinLock {
                    node_id,
                    direction,
                    pin_index,
                    locked,
                } => {
                    result.push((node_id, direction, pin_index, locked));
                }
                other => remaining.push(other),
            }
        }
        self.pending_changes = remaining;
        result
    }

    fn drain_pending_dropped_wire(&mut self) -> Option<PendingDroppedWire> {
        self.pending_dropped_wire.take()
    }

    /// Drain and return any temporary locks scheduled by the viewer (auto-wiring)
    fn drain_temporary_locks(&mut self) -> Vec<TemporaryLock> {
        std::mem::take(&mut self.temporary_locks)
    }

    /// Rebuild the node display cache from ProductionApp.
    /// Called once per frame before rendering.
    /// This replaces storing display data in EditorNode - now ProductionApp is the single source of truth.
    fn rebuild_node_cache(
        &mut self,
        production_app: &ProductionApp,
        snarl: &egui_snarl::Snarl<GraphNode>,
        item_icon_cache: &HashMap<String, egui::TextureHandle>,
        somersloop_icon: Option<egui::TextureId>,
        game_data: &crate::game_data::GameData,
    ) {
        self.node_cache.clear();

        for node_info in snarl.nodes_info() {
            let graph_node = &node_info.value;
            let node_id = graph_node.id;

            // Get label from ProductionApp
            let label = production_app.get_node_label(node_id).unwrap_or_default();

            // Build pin data (common to all node types)
            let mut pins = PinData::default();

            if let Some((input_names, output_names)) =
                production_app.get_node_pin_item_names(node_id)
            {
                pins.input_items = input_names
                    .iter()
                    .map(|opt| {
                        opt.as_ref().and_then(|n: &String| {
                            item_icon_cache.get(n).map(|h| h.id()).and_then(|icon| {
                                Some(ItemData {
                                    name: n.clone(),
                                    icon,
                                })
                            })
                        })
                    })
                    .collect();
                pins.output_items = output_names
                    .iter()
                    .map(|opt| {
                        opt.as_ref().and_then(|n: &String| {
                            item_icon_cache.get(n).map(|h| h.id()).and_then(|icon| {
                                Some(ItemData {
                                    name: n.clone(),
                                    icon,
                                })
                            })
                        })
                    })
                    .collect();
            }

            if let Some((ins, outs)) = production_app.get_node_pin_rates(node_id) {
                pins.input_rates = ins
                    .iter()
                    .map(|opt| {
                        opt.as_ref()
                            .and_then(|s| FractionalNumber::from_string(s).ok())
                    })
                    .collect();
                pins.output_rates = outs
                    .iter()
                    .map(|opt| {
                        opt.as_ref()
                            .and_then(|s| FractionalNumber::from_string(s).ok())
                    })
                    .collect();
            }

            if let Some((ins_locked, outs_locked)) =
                production_app.get_node_pin_locked_flags(node_id)
            {
                pins.input_locked = ins_locked;
                pins.output_locked = outs_locked;
            }

            // Build node-type-specific display data
            let display = match graph_node.node_type {
                GraphNodeType::Craft => {
                    let mut craft = CraftData::default();

                    if let Some((count_str, building_name)) =
                        production_app.get_node_building_info(node_id)
                    {
                        craft.building_count =
                            FractionalNumber::from_string(&count_str).unwrap_or_default();
                        craft.building_name = building_name;
                    }

                    if let Some((same, last, variable)) =
                        production_app.get_node_power_info(node_id)
                    {
                        craft.same_clock_power =
                            FractionalNumber::from_string(&same).unwrap_or_default();
                        craft.last_underclock_power =
                            FractionalNumber::from_string(&last).unwrap_or_default();
                        craft.variable_power = variable;
                    }

                    if let Some((num_str, mult)) = production_app.get_node_somersloop_info(node_id)
                    {
                        craft.num_somersloop =
                            FractionalNumber::from_string(&num_str).unwrap_or_default();
                        craft.somersloop_mult = mult.unwrap_or_default();
                        craft.somersloop_icon = somersloop_icon;
                    }

                    craft.is_power_generator = production_app.get_node_is_power_generator(node_id);

                    // Build progress tracking for Craft nodes
                    if let Some((built_count, total_count)) =
                        production_app.get_node_build_progress(node_id)
                    {
                        if total_count > 0 {
                            craft.built = built_count == total_count;
                        }
                    }

                    NodeDisplayData::Craft {
                        id: node_id,
                        label,
                        pins,
                        craft,
                    }
                }

                GraphNodeType::Merger => {
                    let mut organizer = OrganizerData::default();
                    if let Some(item_name) = production_app.get_node_item_name(node_id) {
                        organizer.item_type = item_icon_cache
                            .get(&item_name)
                            .map(|h| h.id())
                            .and_then(|icon| {
                                Some(ItemData {
                                    name: item_name,
                                    icon,
                                })
                            });
                    }
                    NodeDisplayData::Merger {
                        id: node_id,
                        label,
                        pins,
                        organizer,
                    }
                }

                GraphNodeType::GameSplitter => {
                    let mut organizer = OrganizerData::default();
                    if let Some(item_name) = production_app.get_node_item_name(node_id) {
                        organizer.item_type = item_icon_cache
                            .get(&item_name)
                            .map(|h| h.id())
                            .and_then(|icon| {
                                Some(ItemData {
                                    name: item_name,
                                    icon,
                                })
                            });
                    }
                    NodeDisplayData::GameSplitter {
                        id: node_id,
                        label,
                        pins,
                        organizer,
                    }
                }

                GraphNodeType::CustomSplitter => {
                    let mut organizer = OrganizerData::default();
                    if let Some(item_name) = production_app.get_node_item_name(node_id) {
                        organizer.item_type = item_icon_cache
                            .get(&item_name)
                            .map(|h| h.id())
                            .and_then(|icon| {
                                Some(ItemData {
                                    name: item_name,
                                    icon,
                                })
                            });
                    }
                    NodeDisplayData::CustomSplitter {
                        id: node_id,
                        label,
                        pins,
                        organizer,
                    }
                }

                GraphNodeType::Sink => {
                    let mut sink = SinkData::default();

                    // Get item type for sink
                    if let Some(item_name) = production_app.get_node_item_name(node_id) {
                        sink.item_type =
                            item_icon_cache
                                .get(&item_name)
                                .map(|h| h.id())
                                .and_then(|icon| {
                                    Some(ItemData {
                                        name: item_name,
                                        icon,
                                    })
                                });
                    }

                    // Calculate sink points
                    let mut sum = FractionalNumber::default();
                    for (opt_item, opt_rate) in pins.input_items.iter().zip(pins.input_rates.iter())
                    {
                        if let (Some(item), Some(rate_f)) = (opt_item.as_ref(), opt_rate.as_ref()) {
                            if let Some(item_rc) = game_data.items.get(item.name.as_str()) {
                                let pts = rate_f.clone()
                                    * FractionalNumber::from(item_rc.sink_value as i64);
                                sum += pts;
                            }
                        }
                    }
                    sink.sink_points_fraction_str = sum.to_fraction_string();
                    sink.sink_points = sum;

                    NodeDisplayData::Sink {
                        id: node_id,
                        label,
                        pins,
                        sink,
                    }
                }

                GraphNodeType::Group => {
                    let mut group = GroupData::default();
                    if let Some((built_count, total_count)) =
                        production_app.get_node_build_progress(node_id)
                    {
                        if total_count > 0 {
                            group.is_built = built_count == total_count;
                        }
                    }
                    NodeDisplayData::Group {
                        id: node_id,
                        label,
                        pins,
                        group,
                    }
                }
            };

            self.node_cache.insert(node_id, display);
        }
    }

    // Mark a successful pin edit (record time)
    fn mark_pin_success(&mut self, node_id: u64, dir: PinDirection, idx: usize) {
        self.pin_success
            .insert((node_id, dir, idx), std::time::Instant::now());
    }

    // Return true if the pin edit was successful recently (within 1.5s)
    #[allow(dead_code)]
    fn is_pin_recent_success(&self, node_id: u64, dir: PinDirection, idx: usize) -> bool {
        if let Some(t) = self.pin_success.get(&(node_id, dir, idx)) {
            t.elapsed().as_secs_f32() < 1.5
        } else {
            false
        }
    }

    // Render a fractional number input similar to C++ RenderInputText.
    // Accepts a FractionalNumber value to display; returns (Response, Option<parsed>) where
    // the Option carries a parsed FractionalNumber when the user submitted/committed the field.
    fn render_fractional_input(
        &mut self,
        ui: &mut egui::Ui,
        key: &str,
        initial_value: &mut crate::fractional_number::FractionalNumber,
        width: f32,
        disabled: bool,
    ) -> Option<FractionalNumber> {
        // Ensure buffer exists in edit_buffers, initialize from the provided value when absent
        self.edit_buffers
            .entry(key.to_owned())
            .or_insert_with(|| initial_value.to_float_string());
        let buf_ref = self.edit_buffers.get_mut(key).unwrap();

        let response = ui
            .add_enabled(
                !disabled,
                egui::TextEdit::singleline(buf_ref).desired_width(width),
            )
            .on_hover_text(initial_value.to_fraction_string());

        let mut committed: Option<FractionalNumber> = None;

        // Commit when user presses Enter while focused, or when widget loses focus
        if (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            || response.lost_focus()
        {
            if let Ok(parsed) = FractionalNumber::from_string(buf_ref) {
                committed = Some(parsed);
            }
        }
        committed
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
        let mut all_recipes: Vec<_> = self
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
            .collect();
        // Sort recipes alphabetically (case-insensitive) by display name for predictable UI order
        // Ignore a single leading '*' and any immediate whitespace when comparing
        all_recipes.sort_by_key(|r| {
            let mut s = r.display_name.clone();
            if s.starts_with('*') {
                s = s[1..].trim_start().to_string();
            }
            s
        });

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
        node: &NodeDisplayData,
        size: egui::Vec2,
    ) -> (f32, f32) {
        if self.output_row_width.is_none() {
            let mut max_label_w = 0.0f32;
            let mut max_lines = 1usize;
            for opt in node.pins().output_items.iter() {
                let orig = opt.as_ref().map(|s| s.name.as_str()).unwrap_or("Out");
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
    // TODO: Phase 5 - This should only read from cache and queue PendingChange, not modify nodes directly
    fn sync_merger_splitter(
        &mut self,
        snarl: &mut egui_snarl::Snarl<GraphNode>,
        node_id: egui_snarl::NodeId,
    ) {
        // Get node type from the snarl
        let node_type = if let Some(graph_node) = snarl.get_node(node_id) {
            graph_node.node_type
        } else {
            return;
        };

        // Get cached display data for this node
        let node_display_id = if let Some(graph_node) = snarl.get_node(node_id) {
            graph_node.id
        } else {
            return;
        };

        match node_type {
            GraphNodeType::Merger => {
                // Find chosen item name by looking at connected remotes
                let mut chosen: Option<String> = None;
                let input_count = self
                    .node_cache
                    .get(&node_display_id)
                    .map(|c| c.pins().input_items.len())
                    .unwrap_or(0);

                for input_idx in 0..input_count {
                    let in_id = egui_snarl::InPinId {
                        node: node_id,
                        input: input_idx,
                    };
                    let in_pin = snarl.in_pin(in_id);
                    if let Some(remote) = in_pin.remotes.first() {
                        // Look up remote node's output name from cache
                        if let Some(remote_graph_node) = snarl.get_node(remote.node) {
                            if let Some(remote_cache) = self.node_cache.get(&remote_graph_node.id) {
                                if let Some(Some(item)) =
                                    remote_cache.pins().output_items.get(remote.output)
                                {
                                    chosen = Some(item.name.clone());
                                    break;
                                }
                            }
                        }
                    }
                }

                // Queue update to ProductionApp
                // workaround FIXME
                // Compare with current cached/production value. Only push changes when a
                // chosen item is detected from remotes. Do not clear production-held
                // organizer item when there are no remotes (chosen == None).
                let current_name = self
                    .node_cache
                    .get(&node_display_id)
                    .and_then(|c| c.item_data().map(|i| i.name.clone()));
                if chosen.is_some() {
                    if current_name != chosen {
                        self.pending_changes
                            .push(PendingChange::item(node_display_id, chosen.clone()));
                    }
                }
            }
            GraphNodeType::Sink => {
                // Sinks should NOT have a node-level item_type — pins carry their own types.
                let input_count = self
                    .node_cache
                    .get(&node_display_id)
                    .map(|c| c.pins().input_items.len())
                    .unwrap_or(0);

                for input_idx in 0..input_count {
                    let in_id = egui_snarl::InPinId {
                        node: node_id,
                        input: input_idx,
                    };
                    let in_pin = snarl.in_pin(in_id);

                    let chosen_for_pin = if in_pin.remotes.is_empty() {
                        None
                    } else {
                        // Pick first remote name
                        let mut found: Option<String> = None;
                        for r in in_pin.remotes.iter() {
                            if let Some(remote_graph_node) = snarl.get_node(r.node) {
                                if let Some(remote_cache) =
                                    self.node_cache.get(&remote_graph_node.id)
                                {
                                    if let Some(Some(item)) =
                                        remote_cache.pins().output_items.get(r.output)
                                    {
                                        found = Some(item.name.clone());
                                        break;
                                    }
                                }
                            }
                        }
                        found
                    };

                    // Queue update to ProductionApp for this pin's item
                    self.pending_changes.push(PendingChange::SinkPinItem {
                        node_id: node_display_id,
                        pin_idx: input_idx,
                        item: chosen_for_pin,
                    });
                }
            }
            GraphNodeType::CustomSplitter | GraphNodeType::GameSplitter => {
                // Find chosen item name by looking at connected remotes on the single input
                let mut chosen: Option<String> = None;

                let in_id = egui_snarl::InPinId {
                    node: node_id,
                    input: 0,
                };
                let in_pin = snarl.in_pin(in_id);
                if let Some(remote) = in_pin.remotes.first() {
                    if let Some(remote_graph_node) = snarl.get_node(remote.node) {
                        if let Some(remote_cache) = self.node_cache.get(&remote_graph_node.id) {
                            if let Some(Some(item)) =
                                remote_cache.pins().output_items.get(remote.output)
                            {
                                chosen = Some(item.name.clone());
                            }
                        }
                    }
                }

                // Queue update to ProductionApp
                // workaround FIXME
                // Compare with current cached/production value. Only push changes when a
                // chosen item is detected from remotes. Do not clear production-held
                // organizer item when there are no remotes (chosen == None).
                let current_name = self
                    .node_cache
                    .get(&node_display_id)
                    .and_then(|c| c.item_data().map(|i| i.name.clone()));
                if chosen.is_some() {
                    if current_name != chosen {
                        self.pending_changes.push(PendingChange::item(node_display_id, chosen.clone()));
                    }
                }
            }
            _ => {}
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
        node: &NodeDisplayData,
        dir: PinDirection,
    ) {
        egui::Grid::new(format!(
            "footer_add_col:{}:{}",
            node.id(),
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
                            self.pending_changes
                                .push(PendingChange::add_pin(node.id(), dir));
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
                            self.pending_changes
                                .push(PendingChange::add_pin(node.id(), dir));
                        }
                    });
                }
            }
            ui.end_row();
        });
    }

    // Footer helper methods (placed in regular impl block, not trait impl)
    fn render_craft_footer(
        &mut self,
        ui: &mut egui::Ui,
        node_id: u64,
        pins: &crate::graph_node::PinData,
        craft: &crate::graph_node::CraftData,
    ) {
        if craft.building_name.is_empty() {
            return;
        }

        ui.horizontal(|ui| {
            let power_field_width = ui
                .painter()
                .layout_no_wrap(
                    "000000.00".to_owned(),
                    egui::FontId::default(),
                    egui::Color32::WHITE,
                )
                .size()
                .x;
            let power_label_text = if craft.variable_power { "~MW" } else { "MW" };
            let center_field_width = ui.spacing().interact_size.y;

            egui::Grid::new(format!("footer_grid:{}", node_id))
                .num_columns(3)
                .spacing([8.0, 8.0])
                .min_col_width(ui.available_width() / 3.0)
                .show(ui, |ui| {
                    // Column 1: Power display
                    let mut display_power = if self.power_equal_clocks {
                        craft.same_clock_power.clone()
                    } else {
                        craft.last_underclock_power.clone()
                    };

                    ui.horizontal(|ui| {
                        let key = format!("node:{}:power", node_id);
                        let locked = true; // Power is always locked in this UI
                        self.render_fractional_input(
                            ui,
                            &key,
                            &mut display_power,
                            power_field_width,
                            locked,
                        );
                        let label_resp = ui.label(power_label_text);
                        if craft.variable_power && label_resp.hovered() {
                            label_resp.on_hover_text("Average power");
                        }
                    });

                    // Column 2: Building count and name
                    ui.horizontal(|ui| {
                        let mut building_count = craft.building_count.clone();
                        let key = format!("building:{}", node_id);
                        let node_locked = self.ui_locked_nodes.contains(&node_id);
                        let response = self.render_fractional_input(
                            ui,
                            &key,
                            &mut building_count,
                            center_field_width,
                            node_locked,
                        );

                        if let Some(new_value) = response {
                            self.pending_changes
                                .push(PendingChange::building(node_id, new_value));
                            log::info!(
                                "[UI] queued building edit: node={} -> {}",
                                node_id,
                                new_value.to_fraction_string()
                            );
                        }
                        ui.label(&craft.building_name);
                    });

                    // Column 3: Somersloop (if applicable)
                    if self.settings.show_somersloop
                        && craft.somersloop_mult.numerator() != 0
                        && !craft.is_power_generator
                    {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.horizontal(|ui| {
                                if let Some(tex) = craft.somersloop_icon {
                                    let icon_size = egui::vec2(
                                        ui.spacing().interact_size.y,
                                        ui.spacing().interact_size.y,
                                    );
                                    let (rect, resp) =
                                        ui.allocate_exact_size(icon_size, egui::Sense::hover());
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
                                        resp.on_hover_text("Alien Production Amplification");
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
                                let key = format!("node:{}:somersloop", node_id);
                                let mut current_somersloop = craft.num_somersloop.clone();
                                let is_locked = pins.input_locked.get(0).copied().unwrap_or(false)
                                    || pins.output_locked.get(0).copied().unwrap_or(false);
                                let node_locked = self.ui_locked_nodes.contains(&node_id);
                                let resp = self.render_fractional_input(
                                    ui,
                                    &key,
                                    &mut current_somersloop,
                                    somersloop_width,
                                    is_locked || node_locked,
                                );

                                if let Some(new_value) = resp {
                                    self.pending_changes
                                        .push(PendingChange::somersloop(node_id, new_value));
                                }
                            });
                        });
                    } else {
                        ui.horizontal(|_ui| {});
                    }
                    ui.end_row();
                });
        });
    }

    fn render_organizer_footer(
        &mut self,
        ui: &mut egui::Ui,
        node_id: u64,
        organizer: &crate::graph_node::OrganizerData,
        add_pin_dir: PinDirection,
    ) {
        egui::Grid::new(format!("footer_organizer_grid:{}", node_id))
            .num_columns(3)
            .spacing([8.0, 8.0])
            .min_col_width(ui.available_width() / 3.0)
            .show(ui, |ui| {
                // Left column: '+' for mergers (input direction)
                if add_pin_dir == PinDirection::Input {
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
                            self.pending_changes
                                .push(PendingChange::add_pin(node_id, PinDirection::Input));
                        }
                    });
                } else {
                    ui.horizontal(|_ui| {});
                }

                // Center column: item type (icon + label)
                ui.horizontal(|ui| {
                    if let Some(item) = organizer.item_type.as_ref() {
                        let icon_size =
                            egui::vec2(ui.spacing().interact_size.y, ui.spacing().interact_size.y);
                        ui.image((item.icon, icon_size));
                        ui.add_space(6.0);
                        ui.label(item.name.clone());
                    }
                });

                // Right column: '+' for splitters (output direction)
                if add_pin_dir == PinDirection::Output {
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
                            self.pending_changes
                                .push(PendingChange::add_pin(node_id, PinDirection::Output));
                        }
                    });
                } else {
                    ui.horizontal(|_ui| {});
                }
                ui.end_row();
            });
    }

    fn render_sink_footer(
        &mut self,
        ui: &mut egui::Ui,
        node_id: u64,
        sink: &crate::graph_node::SinkData,
    ) {
        // Render '+' button for adding inputs
        egui::Grid::new(format!("footer_sink_add:{}", node_id))
            .num_columns(3)
            .spacing([8.0, 8.0])
            .min_col_width(ui.available_width() / 3.0)
            .show(ui, |ui| {
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
                        self.pending_changes
                            .push(PendingChange::add_pin(node_id, PinDirection::Input));
                    }
                });
                ui.horizontal(|_ui| {});
                ui.horizontal(|_ui| {});
                ui.end_row();
            });

        // Points row
        egui::Grid::new(format!("footer_sink_points:{}", node_id))
            .num_columns(3)
            .spacing([8.0, 8.0])
            .min_col_width(ui.available_width() / 3.0)
            .show(ui, |ui| {
                // show points
                ui.horizontal(|ui| {
                    let mut points_str = sink.sink_points.to_float_string();
                    let text_edit =
                        egui::TextEdit::singleline(&mut points_str).desired_width(44.0);
                    let response = ui.add_enabled(false, text_edit);
                    if response.hovered() {
                        response.on_hover_text(&sink.sink_points_fraction_str);
                    }
                    ui.label("points");
                });
                ui.horizontal(|_ui| {});
                ui.horizontal(|_ui| {});
                ui.end_row();
            });
    }
}

impl egui_snarl::ui::SnarlViewer<GraphNode> for SnarlViewer {
    fn title(&mut self, node: &GraphNode) -> String {
        // Look up the label from the cache
        if let Some(cached) = self.node_cache.get(&node.id) {
            cached.label().to_string()
        } else {
            format!("Node {}", node.id)
        }
    }

    fn show_header(
        &mut self,
        node_id: egui_snarl::NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        snarl: &mut egui_snarl::Snarl<GraphNode>,
    ) {
        // Default header shows title; override to add a checkbox for group nodes
        if let Some(node_info) = snarl.get_node_info(node_id) {
            // Access the GraphNode stored in the Snarl
            let node = &node_info.value;
            // Look up display data from cache
            let cached = self.node_cache.get(&node.id);
            egui::Grid::new(format!("header_grid_{}", node.id))
                .num_columns(3)
                .min_col_width(ui.available_width() / 3.0)
                .show(ui, |ui| {
                    // skip first column
                    ui.horizontal(|_ui| {});
                    let label = cached
                        .map(|c| c.label().to_string())
                        .unwrap_or_else(|| format!("Node {}", node.id));
                    ui.label(label);

                    if self.settings.show_build_progress {
                        // Right-aligned controls
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(built_status) = cached.and_then(|c| c.built()) {
                                let mut checked = built_status;
                                // Render compact checkbox without label
                                let resp = ui.add(egui::widgets::Checkbox::new(&mut checked, ""));
                                if resp.changed() {
                                    // Queue the change for processing after rendering
                                    self.pending_changes
                                        .push(PendingChange::built(node.id, checked));
                                }
                            }
                        });
                    } else {
                        ui.horizontal(|_ui| {});
                    }
                });
        }
    }

    fn inputs(&mut self, node: &GraphNode) -> usize {
        // Look up display data from cache (rebuilt each frame from ProductionApp)
        // This ensures we're reading from the single source of truth
        if let Some(cached) = self.node_cache.get(&node.id) {
            self.current_node_id = Some(node.id);
            self.input_cursor = 0;
            cached.pins().input_items.len()
        } else {
            // Cache miss - return 0 inputs
            self.current_node_id = None;
            self.input_cursor = 0;
            0
        }
    }

    fn outputs(&mut self, node: &GraphNode) -> usize {
        // Look up display data from cache (rebuilt each frame from ProductionApp)
        if let Some(cached) = self.node_cache.get(&node.id) {
            self.current_node_id = Some(node.id);
            self.output_cursor = 0;
            self.output_anchor_right = None;
            self.output_row_width = None;
            self.output_row_height = None;
            cached.pins().output_items.len()
        } else {
            // Cache miss - return 0 outputs
            self.current_node_id = None;
            self.output_cursor = 0;
            self.output_anchor_right = None;
            self.output_row_width = None;
            self.output_row_height = None;
            0
        }
    }

    fn show_input(
        &mut self,
        _pin: &egui_snarl::InPin,
        ui: &mut egui::Ui,
        _snarl: &mut egui_snarl::Snarl<GraphNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        let size = egui::Vec2::splat(ui.spacing().interact_size.y * 1.2);
        if let Some(node_id) = self.current_node_id {
            if let Some(node) = self.node_cache.get(&node_id).cloned() {
                let idx = self.input_cursor;
                self.input_cursor += 1;
                let pins = node.pins();
                let ntype = node.node_type();
                let nid = node.id();

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    // 'x' remove button for mergers/sinks (near outer edge)
                    if ntype == GraphNodeType::Merger || ntype == GraphNodeType::Sink {
                        let can_remove = pins.input_items.len() > 1;
                        let btn = egui::Button::new("x")
                            .corner_radius(egui::CornerRadius::same(0))
                            .small();
                        let resp = ui.add_enabled(can_remove, btn);
                        if resp.clicked() {
                            self.pending_changes.push(PendingChange::remove_pin(
                                nid,
                                PinDirection::Input,
                                idx,
                            ));
                        }
                    }

                    // Rate first (near outer edge for inputs)
                    if let Some(Some(rate_f)) = pins.input_rates.get(idx) {
                        let key = format!("pin:{}:in:{}", nid, idx);
                        // Use helper to render small input with highlight
                        // Use a conservative fixed width similar to C++ "0000.000"
                        let desired_width = 88.0;
                        let pin_locked = pins.input_locked.get(idx).copied().unwrap_or(false);
                        let node_locked = self.ui_locked_nodes.contains(&nid);
                        let mut rate_value = rate_f.clone();
                        let response = self.render_fractional_input(
                            ui,
                            &key,
                            &mut rate_value,
                            desired_width,
                            pin_locked || node_locked,
                        );
                        // If widget returned a committed value, enqueue it
                        if let Some(new_value) = response {
                            self.pending_changes.push(PendingChange::pin_rate(
                                nid,
                                PinDirection::Input,
                                idx,
                                new_value,
                            ));
                            log::info!(
                                "[UI] queued edit: node={} dir=Input idx={} -> {}",
                                nid,
                                idx,
                                new_value.to_fraction_string()
                            );
                        }

                        // Lock button for merger, custom splitter, and sink pins
                        if ntype == GraphNodeType::Merger
                            || ntype == GraphNodeType::CustomSplitter
                            || ntype == GraphNodeType::Sink
                        {
                            let lock_icon = if pin_locked { "🔒" } else { "🔓" };
                            let lock_btn = egui::Button::new(lock_icon)
                                .corner_radius(egui::CornerRadius::same(2))
                                .small();
                            let lock_resp = ui.add(lock_btn);
                            if lock_resp.clicked() {
                                self.pending_changes.push(PendingChange::pin_lock(
                                    nid,
                                    PinDirection::Input,
                                    idx,
                                    !pin_locked,
                                ));
                            }
                            lock_resp.on_hover_text(if pin_locked {
                                "Unlock this pin"
                            } else {
                                "Lock this pin"
                            });
                        }
                    }

                    // Icon + Label handling
                    if ntype == GraphNodeType::Sink {
                        // For sinks, show an icon+label only if the pin has an item assigned; otherwise show nothing
                        if let Some(Some(item)) = pins.input_items.get(idx) {
                            ui.image((item.icon, size));
                            ui.add_space(6.0);
                            let disp = item.name.clone().replace(' ', "\n");
                            ui.label(disp);
                        } else {
                            // sink: intentionally show nothing when no item set
                        }
                    } else {
                        // Default behavior for non-sink nodes
                        // For merger/splitter nodes we intentionally hide per-pin icons and labels
                        if ntype != GraphNodeType::Merger
                            && ntype != GraphNodeType::CustomSplitter
                            && ntype != GraphNodeType::GameSplitter
                        {
                            if let Some(Some(item)) = pins.input_items.get(idx) {
                                // Use the image widget to draw the texture (lets egui handle clipping/alpha)
                                ui.image((item.icon, size));
                            }

                            // Label closest to center (display names with spaces -> newlines to match C++)
                            if let Some(Some(item)) = pins.input_items.get(idx) {
                                let disp = item.name.clone().replace(' ', "\n");
                                ui.label(disp);
                            } else {
                                ui.label("In");
                            }
                        }
                    }
                });
            }
        }
        egui_snarl::ui::PinInfo::circle()
    }

    fn show_output(
        &mut self,
        _pin: &egui_snarl::OutPin,
        ui: &mut egui::Ui,
        _snarl: &mut egui_snarl::Snarl<GraphNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        let size = egui::Vec2::splat(ui.spacing().interact_size.y * 1.2);
        if let Some(node_id) = self.current_node_id {
            if let Some(node) = self.node_cache.get(&node_id).cloned() {
                let idx = self.output_cursor;
                self.output_cursor += 1;
                let pins = node.pins();
                let ntype = node.node_type();
                let nid = node.id();

                // Capture rects for debug logging
                let _rate_rect: Option<egui::Rect> = None;
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
                        if ntype == GraphNodeType::CustomSplitter
                            || ntype == GraphNodeType::GameSplitter
                        {
                            let can_remove = pins.output_items.len() > 1;
                            let btn = egui::Button::new("x")
                                .corner_radius(egui::CornerRadius::same(0))
                                .small();
                            let resp = ui.add_enabled(can_remove, btn);
                            if resp.clicked() {
                                self.pending_changes.push(PendingChange::remove_pin(
                                    nid,
                                    PinDirection::Output,
                                    idx,
                                ));
                            }
                        }

                        // Rate first (near outer edge for outputs)
                        if let Some(Some(rate_f)) = pins.output_rates.get(idx) {
                            let key = format!("pin:{}:out:{}", nid, idx);
                            // Use a conservative fixed width similar to C++ "0000.000"
                            let desired_width = 88.0;

                            let pin_locked = pins.output_locked.get(idx).copied().unwrap_or(false);
                            let node_locked = self.ui_locked_nodes.contains(&nid);
                            let mut rate_value = rate_f.clone();
                            let response = self.render_fractional_input(
                                ui,
                                &key,
                                &mut rate_value,
                                desired_width,
                                pin_locked || node_locked,
                            );
                            // If widget returned a committed value, enqueue it
                            if let Some(new_value) = response {
                                self.pending_changes.push(PendingChange::pin_rate(
                                    nid,
                                    PinDirection::Output,
                                    idx,
                                    new_value,
                                ));
                                log::info!(
                                    "[UI] queued edit: node={} dir=Output idx={} -> {}",
                                    nid,
                                    idx,
                                    new_value.to_fraction_string()
                                );
                            }

                            // Lock button for custom splitter output pins
                            if ntype == GraphNodeType::CustomSplitter {
                                let lock_icon = if pin_locked { "🔒" } else { "🔓" };
                                let lock_btn = egui::Button::new(lock_icon)
                                    .corner_radius(egui::CornerRadius::same(2))
                                    .small();
                                let lock_resp = ui.add(lock_btn);
                                if lock_resp.clicked() {
                                    self.pending_changes.push(PendingChange::pin_lock(
                                        nid,
                                        PinDirection::Output,
                                        idx,
                                        !pin_locked,
                                    ));
                                }
                                lock_resp.on_hover_text(if pin_locked {
                                    "Unlock this pin"
                                } else {
                                    "Lock this pin"
                                });
                            }
                        }

                        // For merger/splitter nodes we intentionally hide per-pin icons and labels
                        if ntype != GraphNodeType::Merger
                            && ntype != GraphNodeType::CustomSplitter
                            && ntype != GraphNodeType::GameSplitter
                        {
                            // Icon next (inward)
                            if let Some(Some(item)) = pins.output_items.get(idx) {
                                // Use widget-based image drawing
                                let resp = ui.image((item.icon, size));
                                icon_rect = Some(resp.rect);
                            }

                            // Label closest to center (display names with spaces -> newlines to match C++)
                            if let Some(Some(item)) = pins.output_items.get(idx) {
                                let disp = item.name.clone().replace(' ', "\n");
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
        }
        egui_snarl::ui::PinInfo::circle()
    }

    fn has_footer(&mut self, node: &GraphNode) -> bool {
        // Use pattern matching on the enum to determine if footer is needed
        if let Some(cached) = self.node_cache.get(&node.id) {
            match cached {
                NodeDisplayData::Craft { craft, .. } => !craft.building_name.is_empty(),
                NodeDisplayData::Merger { .. }
                | NodeDisplayData::GameSplitter { .. }
                | NodeDisplayData::CustomSplitter { .. }
                | NodeDisplayData::Sink { .. } => true,
                NodeDisplayData::Group { .. } => false,
            }
        } else {
            false
        }
    }

    fn show_footer(
        &mut self,
        _node_id: egui_snarl::NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        _snarl: &mut egui_snarl::Snarl<GraphNode>,
    ) {
        let Some(node_id) = self.current_node_id else {
            return;
        };
        let Some(node) = self.node_cache.get(&node_id).cloned() else {
            return;
        };

        ui.vertical(|ui| {
            match &node {
                NodeDisplayData::Craft {
                    id, pins, craft, ..
                } => {
                    self.render_craft_footer(ui, *id, pins, craft);
                }
                NodeDisplayData::Merger { id, organizer, .. } => {
                    self.render_organizer_footer(ui, *id, organizer, PinDirection::Input);
                }
                NodeDisplayData::GameSplitter { id, organizer, .. }
                | NodeDisplayData::CustomSplitter { id, organizer, .. } => {
                    self.render_organizer_footer(ui, *id, organizer, PinDirection::Output);
                }
                NodeDisplayData::Sink { id, sink, .. } => {
                    self.render_sink_footer(ui, *id, sink);
                }
                NodeDisplayData::Group { .. } => {
                    // Group nodes don't have a footer
                }
            }
        });
    }

    fn has_node_menu(&mut self, _node: &GraphNode) -> bool {
        // Enable node menu (used on RMB). We'll intercept the menu action in `show_node_menu` to toggle visual lock and immediately close
        true
    }

    fn show_node_menu(
        &mut self,
        node: egui_snarl::NodeId,
        _inputs: &[egui_snarl::InPin],
        _outputs: &[egui_snarl::OutPin],
        ui: &mut egui::Ui,
        snarl: &mut egui_snarl::Snarl<GraphNode>,
    ) {
        // Toggle visual lock for this node and close menu immediately
        if let Some(node_ref) = snarl.get_node(node) {
            let nid = node_ref.id;
            let was_locked = self.ui_locked_nodes.contains(&nid);
            if was_locked {
                self.ui_locked_nodes.remove(&nid);
                log::info!("[UI] node {} unlocked (visual) via RMB menu", nid);
                // Request core unlock for connected component
                self.pending_changes.push(PendingChange::lock(nid, false));
            } else {
                self.ui_locked_nodes.insert(nid);
                log::info!("[UI] node {} locked (visual) via RMB menu", nid);
                // Request core lock for connected component
                self.pending_changes.push(PendingChange::lock(nid, true));
            }
        }
        // Close menu so nothing else is shown
        ui.close();
    }

    fn has_graph_menu(
        &mut self,
        _pos: egui::Pos2,
        _snarl: &mut egui_snarl::Snarl<GraphNode>,
    ) -> bool {
        true
    }

    fn has_dropped_wire_menu(
        &mut self,
        _pins: egui_snarl::ui::AnyPins<'_>,
        _snarl: &mut egui_snarl::Snarl<GraphNode>,
    ) -> bool {
        // Allow any dropped wire to show the menu
        true
    }

    fn show_dropped_wire_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut egui::Ui,
        src_pins: egui_snarl::ui::AnyPins<'_>,
        _snarl: &mut egui_snarl::Snarl<GraphNode>,
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
                    if let Some(graph_node) = _snarl.get_node(out.node) {
                        // Look up cached display data
                        if let Some(cached) = self.node_cache.get(&graph_node.id) {
                            if let Some(Some(item)) = cached.pins().output_items.get(out.output) {
                                detected_item = Some(item.name.clone());
                            } else if let Some(item) = cached.item_data() {
                                detected_item = Some(item.name.clone());
                                detected_from_node_item = true;
                            }
                        }
                    }
                }
                (Some(outs.to_vec()), None)
            }
            egui_snarl::ui::AnyPins::In(ins) => {
                // If single in pin and it has a named input, use it
                if ins.len() == 1 {
                    if let Some(graph_node) = _snarl.get_node(ins[0].node) {
                        if let Some(cached) = self.node_cache.get(&graph_node.id) {
                            if let Some(Some(item)) = cached.pins().input_items.get(ins[0].input) {
                                detected_item = Some(item.name.clone());
                            }
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
        _snarl: &mut egui_snarl::Snarl<GraphNode>,
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
        snarl: &mut egui_snarl::Snarl<GraphNode>,
    ) {
        log::info!(
            "[CONNECT] connect called: from={:?} to={:?}",
            from.id,
            to.id
        );

        // Lookup the output and input names from the cache (via the GraphNode's id)
        let out_name = snarl
            .get_node(from.id.node)
            .and_then(|gn| self.node_cache.get(&gn.id))
            .and_then(|c| c.pins().output_items.get(from.id.output))
            .and_then(|opt| opt.clone());
        let in_name = snarl
            .get_node(to.id.node)
            .and_then(|gn| self.node_cache.get(&gn.id))
            .and_then(|c| c.pins().input_items.get(to.id.input))
            .and_then(|opt| opt.clone());

        // Debug: log the attempted connection and the current item types on both pins
        log::info!(
            "[CONNECT] connect attempt from {:?} (out_name={:?}) -> {:?} (in_name={:?})",
            from.id,
            out_name,
            to.id,
            in_name
        );

        // If both pins have an associated item name and they differ, consider rejection.
        // However, if the target input already has existing remotes we allow replacing them (and thus changing the type).
        if let (Some(outn), Some(inn)) = (out_name.clone(), in_name.clone()) {
            if outn != inn {
                let in_has_remotes = !snarl.in_pin(to.id).remotes.is_empty();
                if in_has_remotes {
                    log::info!(
                        "[CONNECT] types differ ('{}' != '{}') but input has existing remotes — will replace them",
                        outn.name,
                        inn.name
                    );
                } else {
                    let msg = format!(
                        "Cannot connect different item types: '{}' -> '{}'",
                        outn.name, inn.name
                    );
                    log::warn!("[CONNECT] {}", msg);
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
                self.pending_changes
                    .push(PendingChange::disconnect(from.id, r));
            }
        }
        let mut in_replacements = 0usize;
        for r in in_remotes.clone() {
            if r != from.id {
                in_replacements += 1;
                affected_nodes.insert(r.node);
                let _ = snarl.disconnect(r, to.id);
                // record pending disconnect so TemplateApp can delete production link
                self.pending_changes
                    .push(PendingChange::disconnect(r, to.id));
            }
        }

        if out_replacements + in_replacements > 0 {
            log::info!(
                "[CONNECT] Replaced {} existing connection(s)",
                out_replacements + in_replacements
            );
        }

        // Finally perform the new connection
        let _ = snarl.connect(from.id, to.id);
        // record pending connection for TemplateApp to create production link & run propagation
        self.pending_changes
            .push(PendingChange::connect(from.id, to.id));
        log::info!(
            "[CONNECT] queued PendingChange::connect for {:?} -> {:?}",
            from.id,
            to.id
        );

        // Sync pin-type assignment/removal for the affected nodes and the endpoints
        affected_nodes.insert(from.id.node);
        affected_nodes.insert(to.id.node);
        for nid in affected_nodes {
            self.sync_merger_splitter(snarl, nid);
        }
    }

    /// Called when the user explicitly disconnects a single wire (e.g., right-click a hovered wire)
    fn disconnect(
        &mut self,
        from: &egui_snarl::OutPin,
        to: &egui_snarl::InPin,
        snarl: &mut egui_snarl::Snarl<GraphNode>,
    ) {
        // Record the disconnect for the TemplateApp to process production model changes
        self.pending_changes
            .push(PendingChange::disconnect(from.id, to.id));
        log::debug!(
            "[UI] queued pending_disconnect: out={:?} in={:?} (disconnect)",
            from.id,
            to.id
        );
        // Also perform the visual disconnect so UI stays in sync
        snarl.disconnect(from.id, to.id);
    }

    /// Called when user requests dropping all outputs (right-click on an output pin)
    fn drop_outputs(&mut self, pin: &egui_snarl::OutPin, snarl: &mut egui_snarl::Snarl<GraphNode>) {
        // enqueue each removed wire
        let remotes = pin.remotes.clone();
        for inp in remotes {
            self.pending_changes
                .push(PendingChange::disconnect(pin.id, inp));
            log::debug!(
                "[UI] queued pending_disconnect: out={:?} in={:?} (drop_outputs)",
                pin.id,
                inp
            );
        }
        // perform the actual removal
        snarl.drop_outputs(pin.id);
    }

    /// Called when user requests dropping all inputs (right-click on an input pin)
    fn drop_inputs(&mut self, pin: &egui_snarl::InPin, snarl: &mut egui_snarl::Snarl<GraphNode>) {
        // enqueue each removed wire
        let remotes = pin.remotes.clone();
        for outp in remotes {
            self.pending_changes
                .push(PendingChange::disconnect(outp, pin.id));
            log::debug!(
                "[UI] queued pending_disconnect: out={:?} in={:?} (drop_inputs)",
                outp,
                pin.id
            );
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
    snarl: egui_snarl::Snarl<GraphNode>,

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

        let initial_settings = Settings::new();
        let mut app = Self {
            production_app: ProductionApp::new(),
            game_data,
            snarl: egui_snarl::Snarl::new(),
            snarl_viewer: SnarlViewer {
                node_cache: HashMap::new(),
                current_node_id: None,
                input_cursor: 0,
                output_cursor: 0,
                output_anchor_right: None,
                output_row_width: None,
                output_row_height: None,
                edit_buffers: HashMap::new(),
                pending_changes: Vec::new(),
                pending_dropped_wire: None,
                ui_locked_nodes: std::collections::HashSet::new(),
                // Temporary locks recorded during auto-wiring to be restored later
                temporary_locks: Vec::new(),
                pin_success: std::collections::HashMap::new(),
                icon_map: std::collections::HashMap::new(),
                recipes: Vec::new(),
                context_menu_recipe_filter: String::new(),
                recipe_checkbox_state: std::collections::HashMap::new(),
                power_equal_clocks: false,
                rejected_connection_reason: None,
                settings: initial_settings.clone(),
            },
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
            settings: initial_settings,
        };

        // Don't add demo nodes if game data loaded successfully
        if app.game_data.recipes.is_empty() {
            // NOTE: Demo nodes would need to be created in ProductionApp first
            // For now, skip demo nodes as they'd need proper production model entries
            let gn = Self::build_graph_node(1, GraphNodeType::Craft);
            app.snarl.insert_node(egui::pos2(0.0, 0.0), gn);
            let gn = Self::build_graph_node(2, GraphNodeType::Sink);
            app.snarl.insert_node(egui::pos2(300.0, 0.0), gn);
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

    // Emit a log and optionally surface the message in the UI (duration in seconds).
    fn emit_message(&mut self, msg: impl Into<String>, level: log::Level) {
        let s = msg.into();
        match level {
            log::Level::Error => log::error!("{}", s),
            log::Level::Warn => log::warn!("{}", s),
            log::Level::Info => log::info!("{}", s),
            log::Level::Debug => log::debug!("{}", s),
            log::Level::Trace => log::trace!("{}", s),
        }
        let show_in_ui = false;
        if show_in_ui {
            self.error_message = s;
            self.error_time = 3.0;
        }
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
                            log::warn!(
                                "Failed to decode somersloop icon {}: {}",
                                somersloop_path,
                                e
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
                            log::warn!("Failed to decode icon {}: {}", path, e);
                        }
                    },
                    Err(e) => {
                        log::warn!("Failed to open icon {}: {}", path, e);
                    }
                }
            }
            log::info!(
                "Loaded {} item icons into cache",
                self.item_icon_cache.len()
            );
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Web loading requires fetching assets; skip for now
            log::warn!("Web: item texture loading not implemented");
        }
    }

    /// Build a lightweight GraphNode for insertion into the Snarl.
    /// The GraphNode only contains the node ID and type - all display data
    /// comes from the cache which is rebuilt from ProductionApp each frame.
    fn build_graph_node(node_id: u64, node_type: GraphNodeType) -> GraphNode {
        GraphNode {
            id: node_id,
            node_type,
        }
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
        self.snarl_viewer.set_icon_map(
            self.item_icon_cache
                .iter()
                .map(|(k, h)| (k.clone(), h.id()))
                .collect(),
        );

        let mut node_map: std::collections::HashMap<u64, egui_snarl::NodeId> =
            std::collections::HashMap::new();

        for node_any in &self.production_app.nodes {
            // Craft
            if let Some(craft) = node_any.downcast_ref::<crate::node::CraftNode>() {
                let node_id = craft.base.id;
                let gn = Self::build_graph_node(node_id, GraphNodeType::Craft);
                let pos = egui::pos2(craft.base.position.0, craft.base.position.1);
                let ui_node = self.snarl.insert_node(pos, gn);
                node_map.insert(node_id, ui_node);
            }
            // Organizer nodes (splitters / merger)
            else if let Some(org) = node_any.downcast_ref::<crate::node::OrganizerNode>() {
                let node_id = org.base.id;
                let node_type = match org.base.kind {
                    crate::node::NodeKind::Merger => GraphNodeType::Merger,
                    crate::node::NodeKind::CustomSplitter => GraphNodeType::CustomSplitter,
                    crate::node::NodeKind::GameSplitter => GraphNodeType::GameSplitter,
                    _ => GraphNodeType::Group,
                };
                let gn = Self::build_graph_node(node_id, node_type);
                let pos = egui::pos2(org.base.position.0, org.base.position.1);
                let ui_node = self.snarl.insert_node(pos, gn);
                node_map.insert(node_id, ui_node);
            }
            // Group
            else if let Some(group) = node_any.downcast_ref::<crate::node::GroupNode>() {
                let node_id = group.base.id;
                let gn = Self::build_graph_node(node_id, GraphNodeType::Group);
                let pos = egui::pos2(group.base.position.0, group.base.position.1);
                let ui_node = self.snarl.insert_node(pos, gn);
                node_map.insert(node_id, ui_node);
            }
            // Sink
            else if let Some(sink) = node_any.downcast_ref::<crate::node::SinkNode>() {
                let node_id = sink.base.id;
                let gn = Self::build_graph_node(node_id, GraphNodeType::Sink);
                let pos = egui::pos2(sink.base.position.0, sink.base.position.1);
                let ui_node = self.snarl.insert_node(pos, gn);
                node_map.insert(node_id, ui_node);
            }
        }

        // Rebuild node display cache now that UI nodes exist so sync operations
        // (which run after connecting links) can consult per-node display data
        // such as pin item names. Without this, sync_merger_splitter will
        // not see remote pin item names and may clear organizer item types.
        self.snarl_viewer.rebuild_node_cache(
            &self.production_app,
            &self.snarl,
            &self.item_icon_cache,
            self.item_icon_cache.get("Somersloop").map(|h| h.id()),
            &self.game_data,
        );

        // Connect links (use production_app.find_pin_location to map pin ids -> node/pin idx)
        for link in &self.production_app.links {
            if let Some((start_node, start_dir, start_idx)) =
                self.production_app.find_pin_location(link.start_pin_id)
            {
                if let Some((end_node, end_dir, end_idx)) =
                    self.production_app.find_pin_location(link.end_pin_id)
                {
                    // Determine out/input ends
                    let (out_node, out_idx, in_node, in_idx) = if start_dir
                        == crate::pin::PinDirection::Output
                        && end_dir == crate::pin::PinDirection::Input
                    {
                        (start_node, start_idx, end_node, end_idx)
                    } else if start_dir == crate::pin::PinDirection::Input
                        && end_dir == crate::pin::PinDirection::Output
                    {
                        (end_node, end_idx, start_node, start_idx)
                    } else {
                        continue; // unsupported
                    };

                    if let (Some(&ui_out), Some(&ui_in)) =
                        (node_map.get(&out_node), node_map.get(&in_node))
                    {
                        let out_pin = egui_snarl::OutPinId {
                            node: ui_out,
                            output: out_idx,
                        };
                        let in_pin = egui_snarl::InPinId {
                            node: ui_in,
                            input: in_idx,
                        };
                        let _ = self.snarl.connect(out_pin, in_pin);

                        // Keep types in sync for organizers (use UI node ids)
                        self.snarl_viewer
                            .sync_merger_splitter(&mut self.snarl, ui_out);
                        self.snarl_viewer
                            .sync_merger_splitter(&mut self.snarl, ui_in);
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

        // Sync settings to snarl viewer (do this at the start of each frame)
        self.snarl_viewer.sync_settings(&self.settings);
        self.snarl_viewer.power_equal_clocks = self.power_equal_clocks;

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
                                self.emit_message("Export not implemented yet", log::Level::Warn);
                            }
                            if ui.button("Import").on_hover_text("Import a production chain from disk").clicked() {
                                // !TODO: Implement import functionality
                                //waitForFileInput();
                                //if (std::filesystem::exists("_internal_load_file"))
                                self.emit_message("Import not implemented yet", log::Level::Warn);
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
                                                    self.emit_message(format!("Save error: {}", e), log::Level::Error);
                                                } else {
                                                    let path = save_dir.join(format!("{}.fcs", self.save_name));
                                                    match fs::write(&path, json) {
                                                        Ok(()) => {
                                                            self.emit_message(format!("Saved: {}", self.save_name), log::Level::Info);
                                                            self.list_save_files();
                                                        }
                                                        Err(e) => {
                                                            self.emit_message(format!("Save error: {}", e), log::Level::Error);
                                                        }
                                                    }
                                                }
                                            }
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                self.emit_message("Save not implemented for web", log::Level::Warn);
                                            }
                                        }
                                        Err(e) => {
                                            self.emit_message(format!("Save error: {}", e), log::Level::Error);
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
                                                    self.emit_message(format!("Loaded: {}", self.save_name), log::Level::Info);
                                                }
                                                Err(e) => {
                                                    self.emit_message(format!("Load error: {}", e), log::Level::Error);
                                                }
                                            },
                                            Err(e) => {
                                                self.emit_message(format!("Load error: {}", e), log::Level::Error);
                                            }
                                        }
                                    }
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        self.emit_message("Load not implemented for web", log::Level::Warn);
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
                log::info!("[AUTO-WIRE] connect_pending_wire_to_node called: new_node={:?}, src_outs={:?}, src_ins={:?}", 
                    new_node, pending.src_outs, pending.src_ins);
                
                // If the dropped source was an Out pin (source->new node input), connect each out to the matching input (by item name) or corresponding index
                if let Some(outs) = pending.src_outs.as_ref() {
                    for out in outs.iter() {
                        // Lookup the new node to get its production ID
                        let node_prod_id = match app.snarl.get_node(new_node) {
                            Some(n) => n.id,
                            None => {
                                log::warn!("[AUTO-WIRE] could not get node from snarl for {:?}", new_node);
                                continue;
                            }
                        };
                        // Get input count and names from cache
                        let input_count = app.snarl_viewer.node_cache.get(&node_prod_id)
                            .map(|c| c.pins().input_items.len())
                            .unwrap_or(0);
                        log::info!("[AUTO-WIRE] node_prod_id={}, input_count={}", node_prod_id, input_count);
                        if input_count == 0 {
                            log::warn!("[AUTO-WIRE] input_count is 0, skipping");
                            continue; // nothing to connect to
                        }

                        // Prefer to match by item name if the dropped wire had a detected item
                        let dest_idx = if let Some(ref item_name) = pending.src_item_name {
                            app.snarl_viewer.node_cache.get(&node_prod_id)
                                .and_then(|c| c.pins().input_items.iter()
                                    .position(|opt| opt.as_ref().map(|s| s.name == *item_name).unwrap_or(false)))
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
                                app.snarl_viewer.pending_changes.push(PendingChange::disconnect(*out, *r));
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
                                app.snarl_viewer.pending_changes.push(PendingChange::disconnect(*r, dest));
                                log::debug!("[UI] queued pending_disconnect: out={:?} in={:?}", r, dest);
                            }
                        }

                        // Before we connect, temporarily lock the original source so propagation treats it as constant
                        // We'll restore the previous lock state after connection by scheduling an unlock pending change.
                        if let Some(src_node) = app.snarl.get_node(out.node) {
                            let src_prod = src_node.id;
                            // If the source is a Craft node, lock the whole node temporarily
                            if matches!(app.production_app.get_node_kind(src_prod), Some(crate::node::NodeKind::Craft)) {
                                // Check if node already has any locked pins
                                let already_locked = app
                                    .production_app
                                    .get_node_pin_locked_flags(src_prod)
                                    .map(|(ins, outs)| ins.iter().any(|b| *b) || outs.iter().any(|b| *b))
                                    .unwrap_or(false);
                                if !already_locked {
                                    match app.production_app.set_node_locked_and_get_affected(src_prod, true) {
                                        Ok(affected) => {
                                            // Update UI lock hints
                                            app.snarl_viewer.ui_locked_nodes.insert(src_prod);
                                            for nid in &affected { app.snarl_viewer.ui_locked_nodes.insert(*nid); }
                                            // Snapshot all connected pins and record which were locked before
                                            let connected_pins = app.production_app.get_all_connected_pins_for_node(src_prod);
                                            let mut locked_snapshot: Vec<u64> = Vec::new();
                                            for pid in connected_pins.iter() {
                                                if let Some((n,d,i)) = app.production_app.find_pin_location(*pid) {
                                                    if let Some((ins_locked, outs_locked)) = app.production_app.get_node_pin_locked_flags(n) {
                                                        let locked = match d {
                                                            PinDirection::Input => ins_locked.get(i).copied().unwrap_or(false),
                                                            PinDirection::Output => outs_locked.get(i).copied().unwrap_or(false),
                                                        };
                                                        if locked { locked_snapshot.push(*pid); }
                                                    }
                                                }
                                            }
                                            // Record affected nodes so we can clear visual lock hints on restore
                                            let affected_nodes_vec = affected.clone();
                                            app.snarl_viewer.temporary_locks.push(TemporaryLock { node_id: src_prod, direction: None, pin_index: None, is_node: true, locked_snapshot, affected_nodes: affected_nodes_vec });
                                            log::info!("[AUTO-WIRE] recorded temporary node lock (snapshot): node={}", src_prod);
                                        }
                                        Err(e) => {
                                            app.emit_message(format!("Error locking source node: {}", e), log::Level::Error);
                                        }
                                    }
                                }
                            } else {
                                if let Some(pinid) = app.production_app.get_pin_id(src_prod, PinDirection::Output, out.output) {
                                    // Check if this output is already locked
                                    let already_locked = app
                                        .production_app
                                        .get_node_pin_locked_flags(src_prod)
                                        .map(|(_ins, outs)| outs.get(out.output).copied().unwrap_or(false))
                                        .unwrap_or(false);
                                    if !already_locked {
                                        if let Err(e) = app.production_app.set_pin_locked(pinid, true) {
                                            app.emit_message(format!("Error locking source pin: {}", e), log::Level::Error);
                                        } else {
                                            app.snarl_viewer.ui_locked_nodes.insert(src_prod);
                                            // Snapshot connected component for this pin and record which pins were locked before
                                            if let Some(pinid) = app.production_app.get_pin_id(src_prod, PinDirection::Output, out.output) {
                                                let connected = app.production_app.get_connected_pins(pinid);
                                                let mut locked_snapshot: Vec<u64> = Vec::new();
                                                for pid in connected.iter() {
                                                    if let Some((n,d,i)) = app.production_app.find_pin_location(*pid) {
                                                        if let Some((ins_locked, outs_locked)) = app.production_app.get_node_pin_locked_flags(n) {
                                                            let locked = match d {
                                                                PinDirection::Input => ins_locked.get(i).copied().unwrap_or(false),
                                                                PinDirection::Output => outs_locked.get(i).copied().unwrap_or(false),
                                                            };
                                                            if locked { locked_snapshot.push(*pid); }
                                                        }
                                                    }
                                                }
                                                // Record affected nodes and snapshot so we can restore visual locks later
                                                let mut affected_nodes_set: std::collections::HashSet<u64> = std::collections::HashSet::new();
                                                for pid in connected.iter() {
                                                    if let Some((n, _d, _i)) = app.production_app.find_pin_location(*pid) {
                                                        affected_nodes_set.insert(n);
                                                    }
                                                }
                                                let affected_nodes_vec: Vec<u64> = affected_nodes_set.into_iter().collect();
                                                app.snarl_viewer.temporary_locks.push(TemporaryLock { node_id: src_prod, direction: Some(PinDirection::Output), pin_index: Some(out.output), is_node: false, locked_snapshot, affected_nodes: affected_nodes_vec });
                                                log::info!("[AUTO-WIRE] recorded temporary pin lock (snapshot): node={} out_idx={}", src_prod, out.output);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Finally connect (visual)
                        let _ = app.snarl.connect(*out, dest);
                        // And schedule production link creation so core model is updated
                        app.snarl_viewer.pending_changes.push(PendingChange::connect(*out, dest));
                        log::info!("[AUTO-WIRE] queued pending_connection: out={:?} in={:?}", out, dest);

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
                        // Lookup the new node to get its production ID
                        let node_prod_id = match app.snarl.get_node(new_node) {
                            Some(n) => n.id,
                            None => {
                                log::warn!("[AUTO-WIRE] could not get node from snarl for {:?}", new_node);
                                continue;
                            }
                        };
                        // Get output count and names from cache
                        let output_count = app.snarl_viewer.node_cache.get(&node_prod_id)
                            .map(|c| c.pins().output_items.len())
                            .unwrap_or(0);
                        log::info!("[AUTO-WIRE] (ins) node_prod_id={}, output_count={}", node_prod_id, output_count);
                        if output_count == 0 {
                            log::warn!("[AUTO-WIRE] output_count is 0, skipping");
                            continue; // nothing to connect from
                        }

                        // Prefer to match by item name if the dropped wire had a detected item
                        let out_idx = if let Some(ref item_name) = pending.src_item_name {
                            app.snarl_viewer.node_cache.get(&node_prod_id)
                                .and_then(|c| c.pins().output_items.iter()
                                    .position(|opt| opt.as_ref().map(|s| s.name == *item_name).unwrap_or(false)))
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
                                // Schedule production link removal
                                app.snarl_viewer.pending_changes.push(PendingChange::disconnect(src_out, *r));
                            }
                        }

                        // Disconnect existing remotes on the destination input
                        let in_remotes = app.snarl.in_pin(*inp).remotes.clone();
                        for r in in_remotes.iter() {
                            if *r != src_out {
                                affected_nodes.insert(r.node);
                                let _ = app.snarl.disconnect(*r, *inp);
                                // Schedule production link removal
                                app.snarl_viewer.pending_changes.push(PendingChange::disconnect(*r, *inp));
                            }
                        }

                        // Before connecting, temporarily lock the original source pin (the inp) so propagation treats its value as constant
                        // We'll restore the previous lock state after connection by scheduling an unlock pending change.
                        if let Some(src_node) = app.snarl.get_node(inp.node) {
                            let src_prod = src_node.id;
                            if matches!(app.production_app.get_node_kind(src_prod), Some(crate::node::NodeKind::Craft)) {
                                // Check if node already has any locked pins
                                let already_locked = app
                                    .production_app
                                    .get_node_pin_locked_flags(src_prod)
                                    .map(|(ins, outs)| ins.iter().any(|b| *b) || outs.iter().any(|b| *b))
                                    .unwrap_or(false);
                                if !already_locked {
                                    match app.production_app.set_node_locked_and_get_affected(src_prod, true) {
                                        Ok(affected) => {
                                            app.snarl_viewer.ui_locked_nodes.insert(src_prod);
                                            for nid in &affected { app.snarl_viewer.ui_locked_nodes.insert(*nid); }
                                            let connected_pins = app.production_app.get_all_connected_pins_for_node(src_prod);
                                            let mut locked_snapshot: Vec<u64> = Vec::new();
                                            for pid in connected_pins.iter() {
                                                if let Some((n,d,i)) = app.production_app.find_pin_location(*pid) {
                                                    if let Some((ins_locked, outs_locked)) = app.production_app.get_node_pin_locked_flags(n) {
                                                        let locked = match d {
                                                            PinDirection::Input => ins_locked.get(i).copied().unwrap_or(false),
                                                            PinDirection::Output => outs_locked.get(i).copied().unwrap_or(false),
                                                        };
                                                        if locked { locked_snapshot.push(*pid); }
                                                    }
                                                }
                                            }
                                            let affected_nodes_vec = affected.clone();
                                            app.snarl_viewer.temporary_locks.push(TemporaryLock { node_id: src_prod, direction: None, pin_index: None, is_node: true, locked_snapshot, affected_nodes: affected_nodes_vec });
                                            log::info!("[AUTO-WIRE] recorded temporary node lock (snapshot): node={}", src_prod);
                                        }
                                        Err(e) => {
                                            app.emit_message(format!("Error locking source node: {}", e), log::Level::Error);
                                        }
                                    }
                                }
                            } else {
                                if let Some(pinid) = app.production_app.get_pin_id(src_prod, PinDirection::Input, inp.input) {
                                    // Check if this input is already locked
                                    let already_locked = app
                                        .production_app
                                        .get_node_pin_locked_flags(src_prod)
                                        .map(|(ins, _outs)| ins.get(inp.input).copied().unwrap_or(false))
                                        .unwrap_or(false);
                                    if !already_locked {
                                        if let Err(e) = app.production_app.set_pin_locked(pinid, true) {
                                            app.emit_message(format!("Error locking source pin: {}", e), log::Level::Error);
                                        } else {
                                            app.snarl_viewer.ui_locked_nodes.insert(src_prod);
                                            if let Some(pinid) = app.production_app.get_pin_id(src_prod, PinDirection::Input, inp.input) {
                                                let connected = app.production_app.get_connected_pins(pinid);
                                                let mut locked_snapshot: Vec<u64> = Vec::new();
                                                for pid in connected.iter() {
                                                    if let Some((n,d,i)) = app.production_app.find_pin_location(*pid) {
                                                        if let Some((ins_locked, outs_locked)) = app.production_app.get_node_pin_locked_flags(n) {
                                                            let locked = match d {
                                                                PinDirection::Input => ins_locked.get(i).copied().unwrap_or(false),
                                                                PinDirection::Output => outs_locked.get(i).copied().unwrap_or(false),
                                                            };
                                                            if locked { locked_snapshot.push(*pid); }
                                                        }
                                                    }
                                                }
                                                // Record affected nodes for this pin's connected component
                                                let mut affected_nodes_set: std::collections::HashSet<u64> = std::collections::HashSet::new();
                                                for pid in connected.iter() {
                                                    if let Some((n, _d, _i)) = app.production_app.find_pin_location(*pid) {
                                                        affected_nodes_set.insert(n);
                                                    }
                                                }
                                                let affected_nodes_vec: Vec<u64> = affected_nodes_set.into_iter().collect();
                                                app.snarl_viewer.temporary_locks.push(TemporaryLock { node_id: src_prod, direction: Some(PinDirection::Input), pin_index: Some(inp.input), is_node: false, locked_snapshot, affected_nodes: affected_nodes_vec });
                                                log::info!("[AUTO-WIRE] recorded temporary pin lock (snapshot): node={} in_idx={}", src_prod, inp.input);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Connect (visual)
                        let _ = app.snarl.connect(src_out, *inp);
                        // Schedule production link creation
                        app.snarl_viewer.pending_changes.push(PendingChange::connect(src_out, *inp));
                        log::info!("[AUTO-WIRE] queued pending_connection: out={:?} in={:?}", src_out, inp);

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

            // Rebuild node display cache from ProductionApp before rendering
            // This ensures the UI reads from the single source of truth
            self.snarl_viewer.rebuild_node_cache(
                &self.production_app,
                &self.snarl,
                &self.item_icon_cache,
                self.item_icon_cache.get("Somersloop").map(|h| h.id()),
                &self.game_data,
            );

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
                        let gn = Self::build_graph_node(node_id, GraphNodeType::Merger);
                        let new_ui_node = self.snarl.insert_node(pending.pos, gn);
                        // Rebuild cache before connecting so wire connection can find pin info
                        self.snarl_viewer.rebuild_node_cache(
                            &self.production_app,
                            &self.snarl,
                            &self.item_icon_cache,
                            self.item_icon_cache.get("Somersloop").map(|h| h.id()),
                            &self.game_data,
                        );
                        // Connect dropped wire to first input if present
                        connect_pending_wire_to_node(self, &pending, new_ui_node);
                        self.emit_message("Created Merger", log::Level::Info);
                    }
                    DroppedWireChoice::CustomSplitter => {
                        let node_id = self.production_app.add_custom_splitter_node();
                        let gn = Self::build_graph_node(node_id, GraphNodeType::CustomSplitter);
                        let new_ui_node = self.snarl.insert_node(pending.pos, gn);
                        self.snarl_viewer.rebuild_node_cache(
                            &self.production_app,
                            &self.snarl,
                            &self.item_icon_cache,
                            self.item_icon_cache.get("Somersloop").map(|h| h.id()),
                            &self.game_data,
                        );
                        connect_pending_wire_to_node(self, &pending, new_ui_node);
                        self.emit_message("Created Custom Splitter", log::Level::Info);
                    }
                    DroppedWireChoice::GameSplitter => {
                        let node_id = self.production_app.add_game_splitter_node();
                        let gn = Self::build_graph_node(node_id, GraphNodeType::GameSplitter);
                        let new_ui_node = self.snarl.insert_node(pending.pos, gn);
                        self.snarl_viewer.rebuild_node_cache(
                            &self.production_app,
                            &self.snarl,
                            &self.item_icon_cache,
                            self.item_icon_cache.get("Somersloop").map(|h| h.id()),
                            &self.game_data,
                        );
                        connect_pending_wire_to_node(self, &pending, new_ui_node);
                        self.emit_message("Created Game Splitter", log::Level::Info);
                    }
                    DroppedWireChoice::Sink => {
                        let node_id = self.production_app.add_sink_node();
                        let gn = Self::build_graph_node(node_id, GraphNodeType::Sink);
                        let new_ui_node = self.snarl.insert_node(pending.pos, gn);
                        self.snarl_viewer.rebuild_node_cache(
                            &self.production_app,
                            &self.snarl,
                            &self.item_icon_cache,
                            self.item_icon_cache.get("Somersloop").map(|h| h.id()),
                            &self.game_data,
                        );
                        connect_pending_wire_to_node(self, &pending, new_ui_node);
                        self.emit_message("Created Sink", log::Level::Info);
                    }
                    DroppedWireChoice::Craft(ref opt_name) => {
                        if let Some(recipe_name) = opt_name {
                            match self.production_app.add_craft_node(&recipe_name, &self.game_data) {
                                Ok(node_id) => {
                                    let gn = Self::build_graph_node(node_id, GraphNodeType::Craft);
                                    let new_ui_node = self.snarl.insert_node(pending.pos, gn);
                                    self.snarl_viewer.rebuild_node_cache(
                                        &self.production_app,
                                        &self.snarl,
                                        &self.item_icon_cache,
                                        self.item_icon_cache.get("Somersloop").map(|h| h.id()),
                                        &self.game_data,
                                    );
                                    connect_pending_wire_to_node(self, &pending, new_ui_node);
                                    self.emit_message(format!("Created: {}", recipe_name), log::Level::Info);
                                }
                                Err(e) => {
                                    self.emit_message(format!("Error: {}", e), log::Level::Error);
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
                self.emit_message(msg, log::Level::Error);
            }

            // Collect nodes that need a UI refresh after mutations
            let mut nodes_to_refresh: Vec<u64> = Vec::new();

            // Process pending pin rate edits collected by the SnarlViewer during rendering
            for (node_id, dir, idx, f) in self.snarl_viewer.drain_pending_edits() {
                if crate::rate_calculator::validate_rate(&f) {
                    log::info!("[UI] processing pending edit: node={} dir={:?} idx={} parsed={}", node_id, dir, idx, f.to_fraction_string());
                    match self.production_app.set_pin_rate(node_id, dir, idx, f) {
                        Ok(()) => {
                            // Success feedback and refresh affected nodes (the node itself and direct neighbors)
                            self.emit_message("Updated pin rate", log::Level::Info);
                            nodes_to_refresh.push(node_id);
                            // Mark pin success (inline UI indicator)
                            self.snarl_viewer.mark_pin_success(node_id, dir, idx);

                            // Update edit buffers immediately so UI reflects the change
                            // The cache will be rebuilt at the start of the next frame
                            // Note: get_node_*_info returns fraction strings for precision, but edit buffers show decimals
                            if let Some((ins, outs)) = self.production_app.get_node_pin_rates(node_id) {
                                for (i, opt) in ins.iter().enumerate() {
                                    if let Some(s) = opt {
                                        let key = format!("pin:{}:in:{}", node_id, i);
                                        // Parse fraction string and convert to float string for display
                                        let display_str = FractionalNumber::from_string(&s).map(|f| f.to_float_string()).unwrap_or(s.clone());
                                        self.snarl_viewer.edit_buffers.insert(key, display_str);
                                    }
                                }
                                for (i, opt) in outs.iter().enumerate() {
                                    if let Some(s) = opt {
                                        let key = format!("pin:{}:out:{}", node_id, i);
                                        let display_str = FractionalNumber::from_string(&s).map(|f| f.to_float_string()).unwrap_or(s.clone());
                                        self.snarl_viewer.edit_buffers.insert(key, display_str);
                                    }
                                }
                            }
                            
                            // Also update building count and power edit buffers
                            if let Some((count_str, _)) = self.production_app.get_node_building_info(node_id) {
                                if !count_str.is_empty() {
                                    let display_str = FractionalNumber::from_string(&count_str).map(|f| f.to_float_string()).unwrap_or(count_str);
                                    self.snarl_viewer.edit_buffers.insert(format!("building:{}", node_id), display_str);
                                } else {
                                    self.snarl_viewer.edit_buffers.remove(&format!("building:{}", node_id));
                                }
                            }
                            if let Some((same, last, _variable)) = self.production_app.get_node_power_info(node_id) {
                                let power_str = if self.snarl_viewer.power_equal_clocks { same } else { last };
                                let power_display_str = FractionalNumber::from_string(&power_str).map(|f| f.to_float_string()).unwrap_or(power_str);
                                self.snarl_viewer.edit_buffers.insert(format!("node:{}:power", node_id), power_display_str);
                            }
                            if let Some((num_str, _somersloop_mult)) = self.production_app.get_node_somersloop_info(node_id) {
                                if !num_str.is_empty() {
                                    let display_str = FractionalNumber::from_string(&num_str).map(|f| f.to_float_string()).unwrap_or(num_str);
                                    self.snarl_viewer.edit_buffers.insert(format!("node:{}:somersloop", node_id), display_str);
                                } else {
                                    self.snarl_viewer.edit_buffers.remove(&format!("node:{}:somersloop", node_id));
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
                            self.emit_message(format!("Error: {}", e), log::Level::Error);
                        }
                    }
                } else {
                    self.emit_message("Invalid rate (negative)", log::Level::Warn);
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
                                self.emit_message(format!("Failed to delete link: {}", e), log::Level::Error);
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
                log::info!("[CONNECT] processing pending connection: {:?} -> {:?}", out_pin, in_pin);
                if let (Some(out_node), Some(in_node)) = (self.snarl.get_node(out_pin.node), self.snarl.get_node(in_pin.node)) {
                    let out_prod = out_node.id;
                    let in_prod = in_node.id;
                    log::info!("[CONNECT] production nodes: {} -> {}", out_prod, in_prod);
                    if let (Some(start_pid), Some(end_pid)) = (
                        self.production_app.get_pin_id(out_prod, PinDirection::Output, out_pin.output),
                        self.production_app.get_pin_id(in_prod, PinDirection::Input, in_pin.input),
                    ) {
                        log::info!("[CONNECT] calling create_link({}, {})", start_pid, end_pid);
                        match self.production_app.create_link(start_pid, end_pid) {
                            Ok((_link_id, Some(warn))) => {
                                self.emit_message(warn, log::Level::Warn);
                                // still refresh both endpoints to keep UI consistent
                                nodes_to_refresh.push(out_prod);
                                nodes_to_refresh.push(in_prod);

                                // Apply same lock propagation behavior as on successful connect
                                // Determine if endpoint nodes are Merger or CustomSplitter (pin-level locking)
                                let out_is_pin_level = matches!(
                                    self.production_app.get_node_kind(out_prod),
                                    Some(crate::node::NodeKind::Merger) | Some(crate::node::NodeKind::CustomSplitter)
                                );
                                let in_is_pin_level = matches!(
                                    self.production_app.get_node_kind(in_prod),
                                    Some(crate::node::NodeKind::Merger) | Some(crate::node::NodeKind::CustomSplitter)
                                );

                                // Check if any connected pin is locked and we need to propagate
                                let mut should_lock_start_pin = false;
                                let mut should_lock_end_pin = false;
                                
                                // Check output node (start pin): if node is UI-locked or has any locked pin
                                if self.snarl_viewer.ui_locked_nodes.contains(&out_prod) {
                                    should_lock_start_pin = true;
                                    should_lock_end_pin = true;
                                }
                                if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(out_prod) {
                                    // For pin-level nodes, check only the specific output pin being connected
                                    if out_is_pin_level {
                                        if outs_locked.get(out_pin.output).copied().unwrap_or(false) {
                                            should_lock_start_pin = true;
                                            should_lock_end_pin = true;
                                        }
                                    } else {
                                        // For other nodes, any locked pin means lock the connected pins
                                        if ins_locked.iter().any(|b| *b) || outs_locked.iter().any(|b| *b) {
                                            should_lock_start_pin = true;
                                            should_lock_end_pin = true;
                                        }
                                    }
                                }
                                
                                // Check input node (end pin): if node is UI-locked or has any locked pin
                                if self.snarl_viewer.ui_locked_nodes.contains(&in_prod) {
                                    should_lock_start_pin = true;
                                    should_lock_end_pin = true;
                                }
                                if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(in_prod) {
                                    // For pin-level nodes, check only the specific input pin being connected
                                    if in_is_pin_level {
                                        if ins_locked.get(in_pin.input).copied().unwrap_or(false) {
                                            should_lock_start_pin = true;
                                            should_lock_end_pin = true;
                                        }
                                    } else {
                                        // For other nodes, any locked pin means lock the connected pins
                                        if ins_locked.iter().any(|b| *b) || outs_locked.iter().any(|b| *b) {
                                            should_lock_start_pin = true;
                                            should_lock_end_pin = true;
                                        }
                                    }
                                }

                                // Apply locks: for pin-level nodes, use set_pin_locked on connected pins only
                                // For other nodes, use set_node_locked_and_get_affected to lock all connected components
                                if should_lock_start_pin || should_lock_end_pin {
                                    let mut all_affected: std::collections::HashSet<u64> = std::collections::HashSet::new();
                                    
                                    // Lock start pin (output) - use pin-level for Merger/CustomSplitter
                                    if should_lock_start_pin {
                                        if out_is_pin_level {
                                            // Only lock this specific pin (and its connected component)
                                            if let Err(e) = self.production_app.set_pin_locked(start_pid, true) {
                                                self.emit_message(format!("Error locking pin: {}", e), log::Level::Error);
                                            } else {
                                                all_affected.insert(out_prod);
                                                // Also add nodes in the pin's connected component
                                                for pid in self.production_app.get_connected_pins(start_pid) {
                                                    if let Some((nid, _, _)) = self.production_app.find_pin_location(pid) {
                                                        all_affected.insert(nid);
                                                    }
                                                }
                                            }
                                        } else {
                                            // Lock all connected components of this node
                                            match self.production_app.set_node_locked_and_get_affected(out_prod, true) {
                                                Ok(affected_vec) => {
                                                    for a in affected_vec { all_affected.insert(a); }
                                                }
                                                Err(e) => {
                                                    self.emit_message(format!("Error applying lock propagation: {}", e), log::Level::Error);
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Lock end pin (input) - use pin-level for Merger/CustomSplitter
                                    if should_lock_end_pin {
                                        if in_is_pin_level {
                                            // Only lock this specific pin (and its connected component)
                                            if let Err(e) = self.production_app.set_pin_locked(end_pid, true) {
                                                self.emit_message(format!("Error locking pin: {}", e), log::Level::Error);
                                            } else {
                                                all_affected.insert(in_prod);
                                                // Also add nodes in the pin's connected component
                                                for pid in self.production_app.get_connected_pins(end_pid) {
                                                    if let Some((nid, _, _)) = self.production_app.find_pin_location(pid) {
                                                        all_affected.insert(nid);
                                                    }
                                                }
                                            }
                                        } else {
                                            // Lock all connected components of this node
                                            match self.production_app.set_node_locked_and_get_affected(in_prod, true) {
                                                Ok(affected_vec) => {
                                                    for a in affected_vec { all_affected.insert(a); }
                                                }
                                                Err(e) => {
                                                    self.emit_message(format!("Error applying lock propagation: {}", e), log::Level::Error);
                                                }
                                            }
                                        }
                                    }

                                    if all_affected.is_empty() {
                                        all_affected.insert(out_prod);
                                        all_affected.insert(in_prod);
                                    }

                                    // Update UI visual locks - skip pin-level nodes (Merger/CustomSplitter)
                                    for nid in &all_affected {
                                        let is_pin_level_node = matches!(
                                            self.production_app.get_node_kind(*nid),
                                            Some(crate::node::NodeKind::Merger) | Some(crate::node::NodeKind::CustomSplitter)
                                        );
                                        if !is_pin_level_node {
                                            self.snarl_viewer.ui_locked_nodes.insert(*nid);
                                        }
                                    }
                                    for nid in &all_affected {
                                        // Locked state is read from cache which comes from ProductionApp
                                        // Just add to refresh list - cache will be rebuilt
                                        nodes_to_refresh.push(*nid);
                                    }
                                }
                            }
                            Ok((_link_id, None)) => {
                                // success; refresh both endpoint nodes
                                nodes_to_refresh.push(out_prod);
                                nodes_to_refresh.push(in_prod);

                                // Determine if endpoint nodes are Merger or CustomSplitter (pin-level locking)
                                let out_is_pin_level = matches!(
                                    self.production_app.get_node_kind(out_prod),
                                    Some(crate::node::NodeKind::Merger) | Some(crate::node::NodeKind::CustomSplitter)
                                );
                                let in_is_pin_level = matches!(
                                    self.production_app.get_node_kind(in_prod),
                                    Some(crate::node::NodeKind::Merger) | Some(crate::node::NodeKind::CustomSplitter)
                                );

                                // Check if any connected pin is locked and we need to propagate
                                let mut should_lock_start_pin = false;
                                let mut should_lock_end_pin = false;
                                
                                // Check output node (start pin): if node is UI-locked or has any locked pin
                                if self.snarl_viewer.ui_locked_nodes.contains(&out_prod) {
                                    should_lock_start_pin = true;
                                    should_lock_end_pin = true;
                                }
                                if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(out_prod) {
                                    // For pin-level nodes, check only the specific output pin being connected
                                    if out_is_pin_level {
                                        if outs_locked.get(out_pin.output).copied().unwrap_or(false) {
                                            should_lock_start_pin = true;
                                            should_lock_end_pin = true;
                                        }
                                    } else {
                                        // For other nodes, any locked pin means lock the connected pins
                                        if ins_locked.iter().any(|b| *b) || outs_locked.iter().any(|b| *b) {
                                            should_lock_start_pin = true;
                                            should_lock_end_pin = true;
                                        }
                                    }
                                }
                                
                                // Check input node (end pin): if node is UI-locked or has any locked pin
                                if self.snarl_viewer.ui_locked_nodes.contains(&in_prod) {
                                    should_lock_start_pin = true;
                                    should_lock_end_pin = true;
                                }
                                if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(in_prod) {
                                    // For pin-level nodes, check only the specific input pin being connected
                                    if in_is_pin_level {
                                        if ins_locked.get(in_pin.input).copied().unwrap_or(false) {
                                            should_lock_start_pin = true;
                                            should_lock_end_pin = true;
                                        }
                                    } else {
                                        // For other nodes, any locked pin means lock the connected pins
                                        if ins_locked.iter().any(|b| *b) || outs_locked.iter().any(|b| *b) {
                                            should_lock_start_pin = true;
                                            should_lock_end_pin = true;
                                        }
                                    }
                                }

                                // Apply locks: for pin-level nodes, use set_pin_locked on connected pins only
                                // For other nodes, use set_node_locked_and_get_affected to lock all connected components
                                if should_lock_start_pin || should_lock_end_pin {
                                    let mut all_affected: std::collections::HashSet<u64> = std::collections::HashSet::new();
                                    
                                    // Lock start pin (output) - use pin-level for Merger/CustomSplitter
                                    if should_lock_start_pin {
                                        if out_is_pin_level {
                                            // Only lock this specific pin (and its connected component)
                                            if let Err(e) = self.production_app.set_pin_locked(start_pid, true) {
                                                self.emit_message(format!("Error locking pin: {}", e), log::Level::Error);
                                            } else {
                                                all_affected.insert(out_prod);
                                                // Also add nodes in the pin's connected component
                                                for pid in self.production_app.get_connected_pins(start_pid) {
                                                    if let Some((nid, _, _)) = self.production_app.find_pin_location(pid) {
                                                        all_affected.insert(nid);
                                                    }
                                                }
                                            }
                                        } else {
                                            // Lock all connected components of this node
                                            match self.production_app.set_node_locked_and_get_affected(out_prod, true) {
                                                Ok(affected_vec) => {
                                                    for a in affected_vec { all_affected.insert(a); }
                                                }
                                                Err(e) => {
                                                    self.emit_message(format!("Error applying lock propagation: {}", e), log::Level::Error);
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Lock end pin (input) - use pin-level for Merger/CustomSplitter
                                    if should_lock_end_pin {
                                        if in_is_pin_level {
                                            // Only lock this specific pin (and its connected component)
                                            if let Err(e) = self.production_app.set_pin_locked(end_pid, true) {
                                                self.emit_message(format!("Error locking pin: {}", e), log::Level::Error);
                                            } else {
                                                all_affected.insert(in_prod);
                                                // Also add nodes in the pin's connected component
                                                for pid in self.production_app.get_connected_pins(end_pid) {
                                                    if let Some((nid, _, _)) = self.production_app.find_pin_location(pid) {
                                                        all_affected.insert(nid);
                                                    }
                                                }
                                            }
                                        } else {
                                            // Lock all connected components of this node
                                            match self.production_app.set_node_locked_and_get_affected(in_prod, true) {
                                                Ok(affected_vec) => {
                                                    for a in affected_vec { all_affected.insert(a); }
                                                }
                                                Err(e) => {
                                                    self.emit_message(format!("Error applying lock propagation: {}", e), log::Level::Error);
                                                }
                                            }
                                        }
                                    }

                                    if all_affected.is_empty() {
                                        all_affected.insert(out_prod);
                                        all_affected.insert(in_prod);
                                    }

                                    // Update UI visual locks and queue nodes for refresh
                                    // For pin-level nodes (Merger/CustomSplitter), do NOT add to ui_locked_nodes
                                    for nid in &all_affected {
                                        let is_pin_level_node = matches!(
                                            self.production_app.get_node_kind(*nid),
                                            Some(crate::node::NodeKind::Merger) | Some(crate::node::NodeKind::CustomSplitter)
                                        );
                                        if !is_pin_level_node {
                                            self.snarl_viewer.ui_locked_nodes.insert(*nid);
                                        }
                                    }
                                    for nid in &all_affected {
                                        // Locked state is read from cache which comes from ProductionApp
                                        // Just add to refresh list - cache will be rebuilt
                                        nodes_to_refresh.push(*nid);
                                    }
                                }
                            }
                            Err(e) => {
                                self.emit_message(format!("Error creating link: {}", e), log::Level::Error);
                            }
                        }
                    }
                }
            }


            // Process somersloop edits collected by the SnarlViewer
            for (node_id, f) in self.snarl_viewer.drain_pending_somersloop_edits() {
                match self.production_app.set_node_somersloop(node_id, f.clone()) {
                    Ok(()) => {
                        // refresh node so UI shows updated somersloop multiplier
                        nodes_to_refresh.push(node_id);
                    }
                    Err(e) => {
                        self.emit_message(format!("Error: {}", e), log::Level::Error);
                    }
                }
            }

            // Process building count edits collected by the SnarlViewer
            for (node_id, f) in self.snarl_viewer.drain_pending_building_edits() {
                if crate::rate_calculator::validate_rate(&f) {
                    log::info!("[UI] processing pending building edit: node={} parsed={}", node_id, f.to_fraction_string());
                    match self.production_app.set_node_building_count(node_id, f.clone()) {
                        Ok(()) => {
                            self.emit_message("Updated building count", log::Level::Info);
                            nodes_to_refresh.push(node_id);

                            // Update edit buffers immediately so UI reflects changes
                            // The cache will be rebuilt at the start of the next frame
                            if let Some((ins, outs)) = self.production_app.get_node_pin_rates(node_id) {
                                for (i, opt) in ins.iter().enumerate() {
                                    if let Some(s) = opt {
                                        let key = format!("pin:{}:in:{}", node_id, i);
                                        let display_str = FractionalNumber::from_string(&s).map(|f| f.to_float_string()).unwrap_or(s.clone());
                                        self.snarl_viewer.edit_buffers.insert(key, display_str);
                                    }
                                }
                                for (i, opt) in outs.iter().enumerate() {
                                    if let Some(s) = opt {
                                        let key = format!("pin:{}:out:{}", node_id, i);
                                        let display_str = FractionalNumber::from_string(&s).map(|f| f.to_float_string()).unwrap_or(s.clone());
                                        self.snarl_viewer.edit_buffers.insert(key, display_str);
                                    }
                                }
                            }
                            if let Some((count_str, _)) = self.production_app.get_node_building_info(node_id) {
                                if !count_str.is_empty() {
                                    let display_str = FractionalNumber::from_string(&count_str).map(|f| f.to_float_string()).unwrap_or(count_str);
                                    self.snarl_viewer.edit_buffers.insert(format!("building:{}", node_id), display_str);
                                } else {
                                    self.snarl_viewer.edit_buffers.remove(&format!("building:{}", node_id));
                                }
                            }
                            if let Some((same, last, _variable)) = self.production_app.get_node_power_info(node_id) {
                                let power_str = if self.snarl_viewer.power_equal_clocks { same } else { last };
                                let power_display = FractionalNumber::from_string(&power_str).map(|f| f.to_float_string()).unwrap_or(power_str);
                                self.snarl_viewer.edit_buffers.insert(format!("node:{}:power", node_id), power_display);
                            }
                            if let Some((num_str, _somersloop_mult)) = self.production_app.get_node_somersloop_info(node_id) {
                                if !num_str.is_empty() {
                                    let display_str = FractionalNumber::from_string(&num_str).map(|f| f.to_float_string()).unwrap_or(num_str);
                                    self.snarl_viewer.edit_buffers.insert(format!("node:{}:somersloop", node_id), display_str);
                                } else {
                                    self.snarl_viewer.edit_buffers.remove(&format!("node:{}:somersloop", node_id));
                                }
                            }

                            // Expand refresh set to all nodes in the connected component
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
                            // Update edit buffers for all connected nodes
                            for n in &connected {
                                if *n == node_id { continue; } // already updated above
                                if let Some((ins, outs)) = self.production_app.get_node_pin_rates(*n) {
                                    for (i, opt) in ins.iter().enumerate() {
                                        if let Some(s) = opt {
                                            let key = format!("pin:{}:in:{}", n, i);
                                            let display_str = FractionalNumber::from_string(&s).map(|f| f.to_float_string()).unwrap_or(s.clone());
                                            self.snarl_viewer.edit_buffers.insert(key, display_str);
                                        }
                                    }
                                    for (i, opt) in outs.iter().enumerate() {
                                        if let Some(s) = opt {
                                            let key = format!("pin:{}:out:{}", n, i);
                                            let display_str = FractionalNumber::from_string(&s).map(|f| f.to_float_string()).unwrap_or(s.clone());
                                            self.snarl_viewer.edit_buffers.insert(key, display_str);
                                        }
                                    }
                                }
                                if let Some((count_str, _)) = self.production_app.get_node_building_info(*n) {
                                    if !count_str.is_empty() {
                                        let display_str = FractionalNumber::from_string(&count_str).map(|f| f.to_float_string()).unwrap_or(count_str);
                                        self.snarl_viewer.edit_buffers.insert(format!("building:{}", n), display_str);
                                    } else {
                                        self.snarl_viewer.edit_buffers.remove(&format!("building:{}", n));
                                    }
                                }
                                if let Some((same, last, _variable)) = self.production_app.get_node_power_info(*n) {
                                    let power_str = if self.snarl_viewer.power_equal_clocks { same } else { last };
                                    let power_display = FractionalNumber::from_string(&power_str).map(|f| f.to_float_string()).unwrap_or(power_str);
                                    self.snarl_viewer.edit_buffers.insert(format!("node:{}:power", n), power_display);
                                }
                            }
                            for n in connected {
                                nodes_to_refresh.push(n);
                            }
                        }
                        Err(e) => {
                            self.emit_message(format!("Error: {}", e), log::Level::Error);
                        }
                    }
                } else {
                    self.emit_message("Invalid rate (negative)", log::Level::Warn);
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
                                    self.emit_message(format!("Error: {}", e), log::Level::Error);
                                    break;
                                }
                            }
                        }
                    }
                    if deleted > 0 {
                        self.emit_message(format!("Deleted {} node(s)", deleted), log::Level::Info);
                    }
                }
            }

            // Process pending group built edits collected by the SnarlViewer
            for (node_id, built) in self.snarl_viewer.drain_pending_built_edits() {
                match self.production_app.set_node_built_state(node_id, built) {
                    Ok(()) => {
                        // Schedule refresh - the build state will come from the rebuilt cache
                        nodes_to_refresh.push(node_id);
                    }
                    Err(e) => {
                        self.emit_message(format!("Error: {}", e), log::Level::Error);
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
                    self.emit_message(format!("Error: {}", e), log::Level::Error);
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
                    self.emit_message(format!("Error: {}", e), log::Level::Error);
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

                        // Schedule affected nodes for refresh - locked state will come from rebuilt cache
                        for nid in &affected_nodes {
                            nodes_to_refresh.push(*nid);
                        }
                    }
                    Err(e) => {
                        self.emit_message(format!("Error: {}", e), log::Level::Error);
                    }
                }
            }

            // Process pending node item type changes requested by viewer (e.g., merger/splitter selection)
            for (node_id, item_opt) in self.snarl_viewer.drain_pending_node_item_changes() {
                match self.production_app.set_node_item_name(node_id, item_opt.clone()) {
                    Ok(()) => {
                        log::debug!("[UI] applied set_node_item_name: node={} item={:?}", node_id, item_opt);
                        // Schedule refresh so the node UI updates from production
                        // The item_type will come from the rebuilt cache
                        nodes_to_refresh.push(node_id);
                    }
                    Err(e) => {
                        self.emit_message(format!("Error: {}", e), log::Level::Error);
                    }
                }
            }

            // Process pending sink pin item assignments requested by the viewer (per-input pin on Sink nodes)
            for (node_id, pin_idx, item_opt) in self.snarl_viewer.drain_pending_sink_pin_items() {
                match self.production_app.set_sink_pin_item(node_id, pin_idx, item_opt.clone()) {
                    Ok(()) => {
                        log::debug!("[UI] applied set_sink_pin_item: node={} pin={} item={:?}", node_id, pin_idx, item_opt);
                        nodes_to_refresh.push(node_id);
                    }
                    Err(e) => {
                        self.emit_message(format!("Error: {}", e), log::Level::Error);
                    }
                }
            }

            // Process pending pin lock changes (for custom splitters/mergers)
            for (node_id, direction, pin_index, locked) in self.snarl_viewer.drain_pending_pin_lock_changes() {
                if let Some(pin_id) = self.production_app.get_pin_id(node_id, direction, pin_index) {
                    match self.production_app.set_pin_locked(pin_id, locked) {
                        Ok(()) => {
                            log::info!("[UI] applied set_pin_locked: node={} dir={:?} idx={} locked={}", 
                                node_id, direction, pin_index, locked);
                            nodes_to_refresh.push(node_id);
                        }
                        Err(e) => {
                            self.emit_message(format!("Error locking pin: {}", e), log::Level::Error);
                        }
                    }
                } else {
                    self.emit_message("Could not find pin to lock".to_string(), log::Level::Error);
                }
            }

            // Restore any temporary locks recorded by auto-wiring
            // Collect pins to unlock to avoid double-unlock and duplicate refreshes
            let mut pins_to_unlock: std::collections::HashSet<u64> = std::collections::HashSet::new();
            // Nodes that we visually marked locked when applying temp locks
            let mut visual_nodes_to_clear: std::collections::HashSet<u64> = std::collections::HashSet::new();
            for t in self.snarl_viewer.drain_temporary_locks() {
                // Remember nodes we visually locked
                for nid in &t.affected_nodes { visual_nodes_to_clear.insert(*nid); }

                if t.is_node {
                    // Recompute connected pins for the node after propagation
                    let connected = self.production_app.get_all_connected_pins_for_node(t.node_id);
                    for pid in connected {
                        // If this pin was unlocked before and is locked now, schedule unlock
                        if !t.locked_snapshot.contains(&pid) {
                            if let Some((n, d, i)) = self.production_app.find_pin_location(pid) {
                                if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(n) {
                                    let currently_locked = match d {
                                        PinDirection::Input => ins_locked.get(i).copied().unwrap_or(false),
                                        PinDirection::Output => outs_locked.get(i).copied().unwrap_or(false),
                                    };
                                    if currently_locked { pins_to_unlock.insert(pid); }
                                }
                            }
                        }
                    }
                } else if let (Some(dir), Some(idx)) = (t.direction, t.pin_index) {
                    if let Some(pin_id) = self.production_app.get_pin_id(t.node_id, dir, idx) {
                        let connected = self.production_app.get_connected_pins(pin_id);
                        for pid in connected {
                            if !t.locked_snapshot.contains(&pid) {
                                if let Some((n, d, i)) = self.production_app.find_pin_location(pid) {
                                    if let Some((ins_locked, outs_locked)) = self.production_app.get_node_pin_locked_flags(n) {
                                        let currently_locked = match d {
                                            PinDirection::Input => ins_locked.get(i).copied().unwrap_or(false),
                                            PinDirection::Output => outs_locked.get(i).copied().unwrap_or(false),
                                        };
                                        if currently_locked { pins_to_unlock.insert(pid); }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Perform unlocks and update UI state / refresh lists
            let mut affected_nodes_for_unlock: std::collections::HashSet<u64> = std::collections::HashSet::new();
            for pid in pins_to_unlock.into_iter() {
                if let Some((nid, _d, _i)) = self.production_app.find_pin_location(pid) {
                    if let Err(e) = self.production_app.set_pin_locked(pid, false) {
                        self.emit_message(format!("Error restoring pin lock state: {}", e), log::Level::Error);
                    } else {
                        affected_nodes_for_unlock.insert(nid);
                    }
                }
            }
            for nid in affected_nodes_for_unlock.into_iter() {
                self.snarl_viewer.ui_locked_nodes.remove(&nid);
                nodes_to_refresh.push(nid);
            }

            // Also clear any visual locks we set when applying the temporary lock
            for nid in visual_nodes_to_clear.into_iter() {
                self.snarl_viewer.ui_locked_nodes.remove(&nid);
                nodes_to_refresh.push(nid);
            }

            // After mutations, update edit buffers so UI input fields reflect the new rates immediately
            // In the new architecture, display data comes from the cache (rebuilt each frame from ProductionApp)
            // so we only need to sync the edit buffers here
            for node_id in &nodes_to_refresh {
                // Rebuild cache to get fresh data
                self.snarl_viewer.rebuild_node_cache(
                    &self.production_app,
                    &self.snarl,
                    &self.item_icon_cache,
                    self.item_icon_cache.get("Somersloop").map(|h| h.id()),
                    &self.game_data,
                );
                
                // Sync edit buffers from the updated cache
                if let Some(cached) = self.snarl_viewer.node_cache.get(node_id) {
                    for (i, opt) in cached.pins().input_rates.iter().enumerate() {
                        if let Some(rate_f) = opt {
                            let key = format!("pin:{}:in:{}", node_id, i);
                            self.snarl_viewer.edit_buffers.insert(key, rate_f.to_float_string());
                        }
                    }
                    for (i, opt) in cached.pins().output_rates.iter().enumerate() {
                        if let Some(rate_f) = opt {
                            let key = format!("pin:{}:out:{}", node_id, i);
                            self.snarl_viewer.edit_buffers.insert(key, rate_f.to_float_string());
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
                ("Right click", "Add node/Lock Pin"),
                ("Right click + mouse", "Move view"),
                ("Left click", "Select node/link"),
                ("Left click + mouse", "Move node/link"),
                ("Mouse wheel", "Zoom/Unzoom"),
                ("Del", "Delete selection"),
                ("F", "Show selection/full graph"),
                ("Alt", "Disable grid snapping"),
                ("Arrows", "Nudge selection"),
                ("Ctrl + A", "Select all nodes"),
                ("Ctrl + D", "Duplicate nodes"),
                ("Ctrl + G", "Group/Ungroup nodes"),
                ("Ctrl + Left click", "Add to selection"),
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
                let any_input = ctx.input(|i| {
                    i.events.iter().any(|e| {
                        matches!(
                            e,
                            egui::Event::Key { .. } | egui::Event::PointerButton { .. }
                        )
                    })
                });
                let clicked_elsewhere = inner
                    .as_ref()
                    .map(|r| r.response.clicked_elsewhere())
                    .unwrap_or(false);
                if any_input || clicked_elsewhere {
                    self.show_controls_popup = false;
                }
            }
        }
    }
}
