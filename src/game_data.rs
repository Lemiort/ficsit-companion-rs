use crate::building::Building;
use crate::fractional_number::FractionalNumber;
use crate::recipe::{CountedItem, Item, Recipe};
use serde_json::Value;
use std::collections::HashMap;
use std::rc::Rc;

/// Game data container holding all recipes, items, and buildings
#[derive(Debug)]
pub struct GameData {
    pub version: String,
    pub items: HashMap<String, Rc<Item>>,
    pub buildings: HashMap<String, Rc<Building>>,
    pub recipes: Vec<Rc<Recipe>>,
}

impl GameData {
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            items: HashMap::new(),
            buildings: HashMap::new(),
            recipes: Vec::new(),
        }
    }

    /// Load game data from JSON
    pub fn load_from_json(&mut self, json_data: &str) -> Result<(), String> {
        let data: Value = serde_json::from_str(json_data)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        // Load version
        if let Some(version) = data.get("version").and_then(|v| v.as_str()) {
            self.version = version.to_string();
        }

        // Load items
        if let Some(items_array) = data.get("items").and_then(|v| v.as_array()) {
            for item_obj in items_array {
                if let (Some(name), Some(icon), Some(sink_value)) = (
                    item_obj.get("name").and_then(|v| v.as_str()),
                    item_obj.get("icon").and_then(|v| v.as_str()),
                    item_obj.get("sink").and_then(|v| v.as_i64()),
                ) {
                    let item = Rc::new(Item::new(
                        name.to_string(),
                        icon,
                        sink_value as i32,
                    ));
                    self.items.insert(name.to_string(), item);
                }
            }
        }

        // Load buildings
        if let Some(buildings_array) = data.get("buildings").and_then(|v| v.as_array()) {
            for building_obj in buildings_array {
                if let Some(name) = building_obj.get("name").and_then(|v| v.as_str()) {
                    let somersloop_mult = building_obj
                        .get("somersloop_mult")
                        .and_then(|v| v.as_f64())
                        .map(|v| FractionalNumber::from((v * 1000.0) as i64) / FractionalNumber::from(1000))
                        .unwrap_or_else(|| FractionalNumber::new(1, 1));

                    let power = building_obj
                        .get("power")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);

                    let power_exponent = building_obj
                        .get("power_exponent")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0);

                    let somersloop_power_exponent = building_obj
                        .get("somersloop_power_exponent")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.0);

                    let variable_power = building_obj
                        .get("variable_power")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let building = Rc::new(Building::new(
                        name.to_string(),
                        somersloop_mult,
                        power,
                        power_exponent,
                        somersloop_power_exponent,
                        variable_power,
                    ));
                    self.buildings.insert(name.to_string(), building);
                }
            }
        }

        // Load recipes
        if let Some(recipes_array) = data.get("recipes").and_then(|v| v.as_array()) {
            for recipe_obj in recipes_array {
                if let (Some(name), Some(building_name)) = (
                    recipe_obj.get("name").and_then(|v| v.as_str()),
                    recipe_obj.get("building").and_then(|v| v.as_str()),
                ) {
                    // Parse inputs
                    let mut inputs = Vec::new();
                    if let Some(inputs_array) = recipe_obj.get("inputs").and_then(|v| v.as_array()) {
                        for input_obj in inputs_array {
                            if let (Some(item_name), Some(amount)) = (
                                input_obj.get("name").and_then(|v| v.as_str()),
                                input_obj.get("amount").and_then(|v| v.as_f64()),
                            ) {
                                if let Some(item) = self.items.get(item_name) {
                                    let quantity = FractionalNumber::from(
                                        (amount * 1000.0) as i64,
                                    ) / FractionalNumber::from(1000);
                                    inputs.push(CountedItem::new(item.clone(), quantity));
                                }
                            }
                        }
                    }

                    // Parse outputs
                    let mut outputs = Vec::new();
                    if let Some(outputs_array) = recipe_obj.get("outputs").and_then(|v| v.as_array()) {
                        for output_obj in outputs_array {
                            if let (Some(item_name), Some(amount)) = (
                                output_obj.get("name").and_then(|v| v.as_str()),
                                output_obj.get("amount").and_then(|v| v.as_f64()),
                            ) {
                                if let Some(item) = self.items.get(item_name) {
                                    let quantity = FractionalNumber::from(
                                        (amount * 1000.0) as i64,
                                    ) / FractionalNumber::from(1000);
                                    outputs.push(CountedItem::new(item.clone(), quantity));
                                }
                            }
                        }
                    }

                    let power = recipe_obj
                        .get("power")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);

                    let alternate = recipe_obj
                        .get("alternate")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let is_spoiler = recipe_obj
                        .get("is_spoiler")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let mut recipe = Recipe::new(
                        inputs,
                        outputs,
                        building_name.to_string(),
                        alternate,
                        power,
                        name.to_string(),
                        is_spoiler,
                    );

                    // Link building reference
                    if let Some(building) = self.buildings.get(building_name) {
                        recipe.set_building(building.clone());
                    }

                    self.recipes.push(Rc::new(recipe));
                }
            }
        }

        Ok(())
    }

    pub fn items(&self) -> &HashMap<String, Rc<Item>> {
        &self.items
    }

    pub fn buildings(&self) -> &HashMap<String, Rc<Building>> {
        &self.buildings
    }

    pub fn recipes(&self) -> &[Rc<Recipe>] {
        &self.recipes
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

impl Default for GameData {
    fn default() -> Self {
        Self::new()
    }
}
