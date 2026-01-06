/// A minimal node payload used for the demo
#[derive(Clone)]
pub struct SimpleNode {
    pub label: String,
}

impl SimpleNode {
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into() }
    }
}

#[derive(Default)]
struct SimpleViewer;

impl egui_snarl::ui::SnarlViewer<SimpleNode> for SimpleViewer {
    fn title(&mut self, node: &SimpleNode) -> String {
        node.label.clone()
    }

    fn inputs(&mut self, _node: &SimpleNode) -> usize {
        1
    }

    fn outputs(&mut self, _node: &SimpleNode) -> usize {
        1
    }

    fn show_input(
        &mut self,
        _pin: &egui_snarl::InPin,
        ui: &mut egui::Ui,
        _snarl: &mut egui_snarl::Snarl<SimpleNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        ui.label("In");
        egui_snarl::ui::PinInfo::circle()
    }

    fn show_output(
        &mut self,
        _pin: &egui_snarl::OutPin,
        ui: &mut egui::Ui,
        _snarl: &mut egui_snarl::Snarl<SimpleNode>,
    ) -> impl egui_snarl::ui::SnarlPin + 'static {
        ui.label("Out");
        egui_snarl::ui::PinInfo::circle()
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    #[serde(skip)] // runtime-only
    value: f32,

    #[serde(skip)]
    snarl: egui_snarl::Snarl<SimpleNode>,

    #[serde(skip)]
    snarl_viewer: SimpleViewer,

    #[serde(skip)]
    snarl_style: egui_snarl::ui::SnarlStyle,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let mut snarl = egui_snarl::Snarl::new();
        // add a couple of demo nodes
        snarl.insert_node(egui::pos2(0.0, 0.0), SimpleNode::new("Node A"));
        snarl.insert_node(egui::pos2(250.0, 0.0), SimpleNode::new("Node B"));

        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            value: 2.7,
            snarl,
            snarl_viewer: SimpleViewer::default(),
            snarl_style: egui_snarl::ui::SnarlStyle::new(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
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

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            ui.collapsing("Node Editor (egui-snarl demo)", |ui| {
                ui.label("Drag pins to connect nodes. Right click for menu.");
                egui::ScrollArea::both().show(ui, |ui| {
                    egui_snarl::ui::SnarlWidget::new()
                        .id(egui::Id::new("snarl-demo"))
                        .style(self.snarl_style)
                        .show(&mut self.snarl, &mut self.snarl_viewer, ui);
                });
            });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
