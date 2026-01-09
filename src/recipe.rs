use crate::fractional_number::FractionalNumber;
use serde::{Deserialize, Serialize};

/// Represents an item in the game (Iron Ore, Copper Wire, etc.)
#[derive(Debug, Clone)]
pub struct Item {
    pub name: String,
    pub new_line_name: String, // Name with spaces replaced by newlines for display
    pub icon_path: String,     // Relative path (from assets/icons) or filename
    pub icon_texture_id: Option<egui::TextureId>, // Will be loaded from image file
    pub sink_value: i32,       // Points when sent to AWESOME Sink
}

impl Item {
    pub fn new(name: String, icon_path: &str, sink_value: i32) -> Self {
        let new_line_name = name.replace(' ', "\n");
        Self {
            name,
            new_line_name,
            icon_path: icon_path.to_string(),
            icon_texture_id: None,
            sink_value,
        }
    }

    /// Set the texture ID after loading the image
    pub fn set_texture(&mut self, texture_id: egui::TextureId) {
        self.icon_texture_id = Some(texture_id);
    }
}

/// Item with quantity (used in recipes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountedItem {
    #[serde(skip)]
    pub item: Option<std::rc::Rc<Item>>, // Will be resolved after deserialization
    pub item_name: String, // For serialization
    pub quantity: FractionalNumber,
}

impl CountedItem {
    pub fn new(item: std::rc::Rc<Item>, quantity: FractionalNumber) -> Self {
        Self {
            item_name: item.name.clone(),
            item: Some(item),
            quantity,
        }
    }
}

/// Represents a crafting recipe
#[derive(Debug, Clone)]
pub struct Recipe {
    pub name: String,
    pub display_name: String, // Includes "*" prefix for alternate recipes
    pub ins: Vec<CountedItem>,
    pub outs: Vec<CountedItem>,
    pub building: Option<std::rc::Rc<crate::building::Building>>, // Will be filled after loading
    pub building_name: String,                                    // For serialization
    pub alternate: bool,
    pub is_spoiler: bool,
    pub power: f64, // Power consumption/generation
    // For search
    lower_name: String,
    lower_ingredients: Vec<String>,
}

impl Recipe {
    pub fn new(
        ins: Vec<CountedItem>,
        outs: Vec<CountedItem>,
        building_name: String,
        alternate: bool,
        power: f64,
        name: String,
        is_spoiler: bool,
    ) -> Self {
        let display_name = if alternate {
            format!("*{}", name)
        } else {
            name.clone()
        };

        let lower_name = name.to_lowercase();

        let mut lower_ingredients = Vec::new();
        for item in ins.iter() {
            lower_ingredients.push(item.item_name.to_lowercase());
        }
        for item in outs.iter() {
            lower_ingredients.push(item.item_name.to_lowercase());
        }

        Self {
            name,
            display_name,
            ins,
            outs,
            building: None,
            building_name,
            alternate,
            is_spoiler,
            power,
            lower_name,
            lower_ingredients,
        }
    }

    /// Search for a string in recipe name (case insensitive)
    pub fn find_in_name(&self, s: &str) -> Option<usize> {
        let lower_s = s.to_lowercase();
        self.lower_name.find(&lower_s)
    }

    /// Search for a string in recipe ingredients (case insensitive)
    pub fn find_in_ingredients(&self, s: &str) -> Option<usize> {
        let lower_s = s.to_lowercase();
        self.lower_ingredients
            .iter()
            .filter_map(|ingredient| ingredient.find(&lower_s))
            .min()
    }

    /// Set the building reference after all buildings are loaded
    pub fn set_building(&mut self, building: std::rc::Rc<crate::building::Building>) {
        self.building = Some(building);
    }
}
