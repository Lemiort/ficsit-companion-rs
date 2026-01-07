use crate::production_app::ProductionApp;
use serde::{Deserialize, Serialize};

/// Simple node representation for the node editor
#[derive(Clone, Debug)]
pub struct EditorNode {
    #[allow(dead_code)]
    pub id: u64,
    pub label: String,
    #[allow(dead_code)]
    pub node_type: String,

    // Pin metadata for icons and labels
    pub input_names: Vec<Option<String>>,
    pub input_icons: Vec<Option<egui::TextureId>>,
    pub output_names: Vec<Option<String>>,
    pub output_icons: Vec<Option<egui::TextureId>>,
}

impl EditorNode {
    pub fn new(id: u64, label: impl Into<String>, node_type: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            node_type: node_type.into(),
            input_names: Vec::new(),
            input_icons: Vec::new(),
            output_names: Vec::new(),
            output_icons: Vec::new(),
        }
    }

    pub fn with_pins(
        id: u64,
        label: impl Into<String>,
        node_type: impl Into<String>,
        input_names: Vec<Option<String>>,
        input_icons: Vec<Option<egui::TextureId>>,
        output_names: Vec<Option<String>>,
        output_icons: Vec<Option<egui::TextureId>>,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            node_type: node_type.into(),
            input_names,
            input_icons,
            output_names,
            output_icons,
        }
    }
}

#[derive(Default, Debug)]
struct SnarlViewer {
    // Keep a clone of the current node being rendered to access pin metadata in show_input/show_output
    current_node: Option<EditorNode>,
    // Cursors advanced by show_input/show_output calls to get the pin index in order
    input_cursor: usize,
    output_cursor: usize,
}

impl egui_snarl::ui::SnarlViewer<EditorNode> for SnarlViewer {
    fn title(&mut self, node: &EditorNode) -> String {
        node.label.clone()
    }

    fn inputs(&mut self, node: &EditorNode) -> usize {
        self.current_node = Some(node.clone());
        self.input_cursor = 0;
        node.input_names.len()
    }

    fn outputs(&mut self, node: &EditorNode) -> usize {
        self.current_node = Some(node.clone());
        self.output_cursor = 0;
        node.output_names.len()
    }

    fn show_input(
        &mut self,
        _pin: &egui_snarl::InPin,
        ui: &mut egui::Ui,
        _snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        let size = egui::Vec2::splat(ui.spacing().interact_size.y * 1.2);
        if let Some(node) = &self.current_node {
            let idx = self.input_cursor;
            self.input_cursor += 1;
            ui.horizontal(|ui| {
                if let Some(Some(tex)) = node.input_icons.get(idx) {
                    eprintln!("show_input: node {} input[{}] has texture {:?}", node.id, idx, tex);
                    // Use the image widget to draw the texture (lets egui handle clipping/alpha)
                    ui.image((*tex, size));
                } else if let Some(Some(name)) = node.input_names.get(idx) {
                    eprintln!("show_input: node {} input[{}] name={}", node.id, idx, name);
                    ui.label(name);
                } else {
                    ui.label("In");
                }
            });
        } else {
            ui.label("In");
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
        if let Some(node) = &self.current_node {
            let idx = self.output_cursor;
            self.output_cursor += 1;
            ui.horizontal(|ui| {
                if let Some(Some(name)) = node.output_names.get(idx) {
                    eprintln!("show_output: node {} output[{}] name={}", node.id, idx, name);
                    ui.label(name);
                }
                if let Some(Some(tex)) = node.output_icons.get(idx) {
                    eprintln!("show_output: node {} output[{}] has texture {:?}", node.id, idx, tex);
                    // Use widget-based image drawing
                    ui.image((*tex, size));
                }
            });
        } else {
            ui.label("Out");
        }
        egui_snarl::ui::PinInfo::circle()
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
                        println!("✓ Loaded {} recipes from game data", game_data.recipes.len());
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
        
        let mut app = Self {
            production_app: ProductionApp::new(),
            game_data,
            snarl: egui_snarl::Snarl::new(),
            snarl_viewer: SnarlViewer::default(),
            snarl_style: egui_snarl::ui::SnarlStyle::new(),
            left_panel_collapsed: false,
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

        app
    }

    /// Load item icon textures into `item_icon_cache` using `cc.egui_ctx`.
    fn load_item_textures(&mut self, cc: &eframe::CreationContext<'_>) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use image::ImageReader as ImageReader;
            use egui::ColorImage;

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
                            let texture = cc.egui_ctx.load_texture(name.clone(), color_image, egui::TextureOptions::default());
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
            println!("Loaded {} item icons into cache", self.item_icon_cache.len());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Web loading requires fetching assets; skip for now
            eprintln!("Web: item texture loading not implemented");
        }
    }

    /// Build an EditorNode from production model (fill pin names and icons)
    fn build_editor_node(&self, node_id: u64, label: impl Into<String>, node_type: impl Into<String>) -> EditorNode {
        let (input_names, output_names) = self
            .production_app
            .get_node_pin_item_names(node_id)
            .unwrap_or((Vec::new(), Vec::new()));

        // Debug: print mapping of pin names to whether an icon exists in cache
        eprintln!("build_editor_node {}: inputs:", node_id);
        for (i, n) in input_names.iter().enumerate() {
            let has = n.as_ref().map(|s| self.item_icon_cache.contains_key(s)).unwrap_or(false);
            eprintln!("  input[{}]: {:?} -> has_icon={}", i, n, has);
        }
        eprintln!("build_editor_node {}: outputs:", node_id);
        for (i, n) in output_names.iter().enumerate() {
            let has = n.as_ref().map(|s| self.item_icon_cache.contains_key(s)).unwrap_or(false);
            eprintln!("  output[{}]: {:?} -> has_icon={}", i, n, has);
        }

        let input_icons: Vec<Option<egui::TextureId>> = input_names
            .iter()
            .map(|opt_name| opt_name.as_ref().and_then(|n| self.item_icon_cache.get(n).map(|h| h.id())))
            .collect();

        let output_icons: Vec<Option<egui::TextureId>> = output_names
            .iter()
            .map(|opt_name| opt_name.as_ref().and_then(|n| self.item_icon_cache.get(n).map(|h| h.id())))
            .collect();

        EditorNode::with_pins(
            node_id,
            label,
            node_type,
            input_names,
            input_icons,
            output_names,
            output_icons,
        )
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
        egui::TopBottomPanel::top("top_panel")
            .show(ctx, |ui| {
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
                                eprintln!("topbar: drawing icon for {} -> {:?}", name, handle.id());
                                let size = egui::Vec2::splat(28.0);
                                let (rect, _resp) = ui.allocate_exact_size(size, egui::Sense::hover());
                                ui.painter().image(handle.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
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
                    if self.error_message.starts_with("error")  || self.error_message.starts_with("Error")
                        { egui::Color32::RED }
                    else { egui::Color32::GREEN },
                    &self.error_message,
                );
                ui.ctx().request_repaint();
            }

            ui.separator();

            // Node editor (direct snarl widget so it receives events for selection and dragging)
            let snarl_response = egui_snarl::ui::SnarlWidget::new()
                .id(egui::Id::new("production-snarl"))
                .style(self.snarl_style)
                .show(&mut self.snarl, &mut self.snarl_viewer, ui);

            // Detect right-click to show add node popup (like C++ ShowBackgroundContextMenu)
            if snarl_response.secondary_clicked() {
                self.show_add_node_popup = true;
                self.add_node_popup_pos = ui.ctx().pointer_interact_pos().unwrap_or(egui::pos2(300.0, 300.0));
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
        // Add node popup (like C++ AddNewNode)
        if self.show_add_node_popup {
            let mut open = true;
            egui::Window::new("Add Node")
                .open(&mut open)
                .fixed_pos(self.add_node_popup_pos)
                .resizable(false)
                .default_width(300.0)
                .collapsible(false)
                .show(ctx, |ui| {
                    // Basic nodes at top
                    if ui.button("Merger").clicked() {
                        let node_id = self.production_app.add_merger_node();
                        let en = self.build_editor_node(node_id, "Merger", "merger");
                        self.snarl.insert_node(self.add_node_popup_pos, en);
                        self.error_message = "Created Merger".to_string();
                        self.error_time = 2.0;
                        self.show_add_node_popup = false;
                    }
                    
                    if ui.button("Splitter*").on_hover_text("Splitter with independent output rates").clicked() {
                        let node_id = self.production_app.add_custom_splitter_node();
                        let en = self.build_editor_node(node_id, "Splitter*", "custom_splitter");
                        self.snarl.insert_node(self.add_node_popup_pos, en);
                        self.error_message = "Created Custom Splitter".to_string();
                        self.error_time = 2.0;
                        self.show_add_node_popup = false;
                    }
                    
                    if ui.button("Splitter").on_hover_text("Splitter with equal output rates").clicked() {
                        let node_id = self.production_app.add_game_splitter_node();
                        let en = self.build_editor_node(node_id, "Splitter", "game_splitter");
                        self.snarl.insert_node(self.add_node_popup_pos, en);
                        self.error_message = "Created Game Splitter".to_string();
                        self.error_time = 2.0;
                        self.show_add_node_popup = false;
                    }
                    
                    if ui.button("Sink").clicked() {
                        let node_id = self.production_app.add_sink_node();
                        let en = self.build_editor_node(node_id, "Sink", "sink");
                        self.snarl.insert_node(self.add_node_popup_pos, en);
                        self.error_message = "Created Sink".to_string();
                        self.error_time = 2.0;
                        self.show_add_node_popup = false;
                    }
                    
                    ui.separator();
                    
                    // Recipe filter like C++ - auto-focus on first show
                    let filter_response = ui.text_edit_singleline(&mut self.context_menu_recipe_filter);
                    if ui.memory(|mem| mem.focused().is_none()) {
                        filter_response.request_focus();
                    }
                    
                    ui.separator();
                    
                    // Show recipes
                    // Clone Rc's to avoid borrowing self while we mutate production_app in the click handler
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
                        .take(20) // Limit to prevent huge menu
                        .collect();
                    
                    if all_recipes.is_empty() && !self.game_data.recipes.is_empty() {
                        ui.label("No matching recipes");
                    } else if self.game_data.recipes.is_empty() {
                        ui.colored_label(egui::Color32::RED, "⚠ No game data loaded!");
                        ui.label("Check assets/satisfactory.json");
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                // Two-column grid: names | icons
                                egui::Grid::new("recipe_grid").striped(true).show(ui, |ui| {
                                    for recipe in all_recipes {
                                        let mut clicked = false;

                                        // Left column: recipe name (button)
                                        if ui.button(&recipe.display_name).clicked() {
                                            clicked = true;
                                        }

                                        // Move to second column
                                        // Right column: icons laid out as [inputs...] -> [outputs...] aligned to the left
                                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                            let icon_size = egui::Vec2::splat(ui.spacing().interact_size.y * 1.0);

                                            // Temporarily set horizontal item spacing to 0 to eliminate default gaps
                                            let original_spacing = ui.spacing().item_spacing;
                                            ui.spacing_mut().item_spacing.x = 0.0;

                                            // Draw inputs (leftmost group) with no spacing between icons
                                            for inp in recipe.ins.iter().take(4) {
                                                if let Some(handle) = self.item_icon_cache.get(&inp.item_name) {
                                                    let (rect, _resp) = ui.allocate_exact_size(icon_size, egui::Sense::hover());
                                                    ui.painter().image(handle.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(1.0,1.0)), egui::Color32::WHITE);
                                                }
                                            }

                                            // Arrow in middle if both sides exist (tight)
                                            if !recipe.ins.is_empty() && !recipe.outs.is_empty() {
                                                let arrow_size = egui::Vec2::splat(ui.spacing().interact_size.y * 0.6);
                                                let (rect, _resp) = ui.allocate_exact_size(arrow_size, egui::Sense::hover());
                                                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, " --> ", egui::FontId::default(), egui::Color32::WHITE);
                                            }

                                            // Draw outputs (right group) with no spacing between icons
                                            for out in recipe.outs.iter().take(4) {
                                                if let Some(handle) = self.item_icon_cache.get(&out.item_name) {
                                                    let (rect, _resp) = ui.allocate_exact_size(icon_size, egui::Sense::hover());
                                                    ui.painter().image(handle.id(), rect, egui::Rect::from_min_max(egui::pos2(0.0,0.0), egui::pos2(1.0,1.0)), egui::Color32::WHITE);
                                                }
                                            }

                                            // Restore spacing
                                            ui.spacing_mut().item_spacing = original_spacing;
                                        });

                                        ui.end_row();

                                        if clicked {
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
                });
            
            if !open {
                self.show_add_node_popup = false;
                self.context_menu_recipe_filter.clear();
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
                                    match self.production_app.add_craft_node(recipe_name, &self.game_data) {
                                        Ok(node_id) => {
                                            let en = self.build_editor_node(node_id, *recipe_name, "craft");
                                            self.snarl.insert_node(egui::pos2(300.0, 300.0), en);
                                            self.error_message = format!("Created: {}", recipe_name);
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

        let recipe = app.game_data.recipes.get(0).expect("No recipes loaded").clone();
        assert!(!recipe.outs.is_empty(), "Recipe has no outputs");

        let output_item_name = recipe.outs[0].item_name.clone();

        // Insert a fake texture handle into the cache for that item using a local egui context
        let ctx = egui::Context::default();
        let color = egui::ColorImage::example();
        let handle = ctx.load_texture("test", color, egui::TextureOptions::NEAREST);
        app.item_icon_cache.insert(output_item_name.clone(), handle);

        // Add craft node using the recipe
        let node_id = app.production_app.add_craft_node(&recipe.name, &app.game_data).expect("Failed to add craft node");

        // Build the editor node and ensure output icons contains at least one Some
        let en = app.build_editor_node(node_id, &recipe.display_name, "craft");
        assert!(en.output_icons.iter().any(|o| o.is_some()), "Output icons were not mapped from cache");
    }
}