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
}

impl EditorNode {
    pub fn new(id: u64, label: impl Into<String>, node_type: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            node_type: node_type.into(),
        }
    }
}

#[derive(Default, Debug)]
struct SnarlViewer;

impl egui_snarl::ui::SnarlViewer<EditorNode> for SnarlViewer {
    fn title(&mut self, node: &EditorNode) -> String {
        node.label.clone()
    }

    fn inputs(&mut self, _node: &EditorNode) -> usize {
        1 // Simplified: all nodes have 1 input for now
    }

    fn outputs(&mut self, _node: &EditorNode) -> usize {
        1 // Simplified: all nodes have 1 output for now
    }

    fn show_input(
        &mut self,
        _pin: &egui_snarl::InPin,
        ui: &mut egui::Ui,
        _snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        ui.label("In");
        egui_snarl::ui::PinInfo::circle()
    }

    fn show_output(
        &mut self,
        _pin: &egui_snarl::OutPin,
        ui: &mut egui::Ui,
        _snarl: &mut egui_snarl::Snarl<EditorNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        ui.label("Out");
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
    snarl: egui_snarl::Snarl<EditorNode>,

    #[serde(skip)]
    snarl_viewer: SnarlViewer,

    #[serde(skip)]
    snarl_style: egui_snarl::ui::SnarlStyle,

    // UI State
    #[serde(skip)]
    show_save_dialog: bool,

    #[serde(skip)]
    show_load_dialog: bool,

    #[serde(skip)]
    show_recipe_selector: bool,

    #[serde(skip)]
    selected_recipe: Option<String>,

    #[serde(skip)]
    recipe_search: String,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let mut app = Self {
            production_app: ProductionApp::new(),
            snarl: egui_snarl::Snarl::new(),
            snarl_viewer: SnarlViewer::default(),
            snarl_style: egui_snarl::ui::SnarlStyle::new(),
            show_save_dialog: false,
            show_load_dialog: false,
            show_recipe_selector: false,
            selected_recipe: None,
            recipe_search: String::new(),
        };

        // Add demo nodes if no game data is loaded
        if app.production_app.get_recipe_names().is_empty() {
            app.snarl.insert_node(
                egui::pos2(0.0, 0.0),
                EditorNode::new(1, "Craft Node A", "craft"),
            );
            app.snarl.insert_node(
                egui::pos2(300.0, 0.0),
                EditorNode::new(2, "Sink Node B", "sink"),
            );
        }

        app
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any).
        if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show_top_panel(ctx);
        self.show_side_panel(ctx);
        self.show_central_panel(ctx);
    }
}

impl TemplateApp {
    fn show_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let is_web = cfg!(target_arch = "wasm32");

                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("New").clicked() {
                            self.production_app = ProductionApp::new();
                            self.snarl = egui_snarl::Snarl::new();
                            ui.close();
                        }

                        if ui.button("Open...").clicked() {
                            self.show_load_dialog = true;
                            ui.close();
                        }

                        if ui.button("Save...").clicked() {
                            self.show_save_dialog = true;
                            ui.close();
                        }

                        ui.separator();

                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                }

                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() {
                        ui.close();
                    }
                    if ui.button("Redo").clicked() {
                        ui.close();
                    }
                });

                ui.menu_button("Add", |ui| {
                    if ui.button("Craft Node...").clicked() {
                        self.show_recipe_selector = true;
                        ui.close();
                    }
                    if ui.button("Splitter").clicked() {
                        ui.close();
                    }
                    if ui.button("Merger").clicked() {
                        ui.close();
                    }
                    if ui.button("Sink").clicked() {
                        ui.close();
                    }
                });

                ui.add_space(16.0);
                egui::widgets::global_theme_preference_buttons(ui);
            });
        });
    }

    fn show_side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("📊 Production Info");

                ui.label(format!("📦 Nodes: {}", self.production_app.node_count()));
                ui.label(format!(
                    "⚙️ Recipes: {}",
                    self.production_app.get_recipe_names().len()
                ));

                ui.separator();

                ui.group(|ui| {
                    ui.label("Status:");
                    if self.production_app.has_unsaved_changes() {
                        ui.colored_label(egui::Color32::YELLOW, "⚠ Unsaved changes");
                    } else {
                        ui.colored_label(egui::Color32::GREEN, "✓ All saved");
                    }
                });

                ui.separator();

                // Recipe list with search
                ui.label("🔍 Quick Recipes:");
                ui.text_edit_singleline(&mut self.recipe_search);

                let recipes = self.production_app.get_recipe_names();
                let filtered_recipes: Vec<_> = recipes
                    .iter()
                    .filter(|r| {
                        self.recipe_search.is_empty()
                            || r.to_lowercase().contains(&self.recipe_search.to_lowercase())
                    })
                    .collect();

                ui.label(format!("Showing {}/{} recipes", filtered_recipes.len(), recipes.len()));

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for recipe_name in filtered_recipes.iter().take(20) {
                            if ui.button(format!("+ {}", recipe_name)).clicked() {
                                // In a real implementation, this would create a craft node
                                self.selected_recipe = Some((*recipe_name).clone());
                            }
                        }

                        if filtered_recipes.len() > 20 {
                            ui.label(format!("... and {} more", filtered_recipes.len() - 20));
                        }
                    });

                ui.separator();

                if ui.button("📁 Save Game Data").clicked() {
                    // Would open file dialog
                }

                if ui.button("🔄 Reload Game Data").clicked() {
                    // Would reload game data from file
                }
            });
    }

    fn show_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("⚙️ Production Graph");

            // Node editor
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui_snarl::ui::SnarlWidget::new()
                        .id(egui::Id::new("production-snarl"))
                        .style(self.snarl_style)
                        .show(&mut self.snarl, &mut self.snarl_viewer, ui);
                });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label(format!("Unsaved changes: {}", self.production_app.has_unsaved_changes()));
                if ui.button("Clear Search").clicked() {
                    self.recipe_search.clear();
                }
            });
        });

        // Recipe selection dialog
        if self.show_recipe_selector {
            let mut open = true;
            egui::Window::new("Select Recipe for Craft Node")
                .open(&mut open)
                .resizable(true)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.label("Search recipes:");
                    let mut temp_search = String::new();
                    ui.text_edit_singleline(&mut temp_search);

                    let recipes = self.production_app.get_recipe_names();
                    let filtered_recipes: Vec<_> = recipes
                        .iter()
                        .filter(|r| {
                            temp_search.is_empty()
                                || r.to_lowercase().contains(&temp_search.to_lowercase())
                        })
                        .collect();

                    ui.label(format!("Found {} recipes", filtered_recipes.len()));

                    egui::ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for recipe_name in filtered_recipes.iter() {
                                if ui.button(*recipe_name).clicked() {
                                    self.selected_recipe = Some((*recipe_name).clone());
                                    self.show_recipe_selector = false;
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

        // Save dialog (skeleton)
        if self.show_save_dialog {
            let mut open = true;
            egui::Window::new("Save Production Chain")
                .open(&mut open)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Filename:");
                    let mut filename = String::new();
                    ui.text_edit_singleline(&mut filename);

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            // Would save file
                            self.show_save_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_save_dialog = false;
                        }
                    });
                });
            self.show_save_dialog = open;
        }

        // Load dialog (skeleton)
        if self.show_load_dialog {
            let mut open = true;
            egui::Window::new("Open Production Chain")
                .open(&mut open)
                .resizable(true)
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.label("Select a file to load:");

                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("Open").clicked() {
                            // Would load file
                            self.show_load_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_load_dialog = false;
                        }
                    });
                });
            self.show_load_dialog = open;
        }
    }
}

