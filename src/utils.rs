use serde_json::Value;
use std::cmp::Ordering;

/// Comparator for Item pointers based on name
pub struct ItemCompare;

impl ItemCompare {
    pub fn compare(a_name: &str, b_name: &str) -> Ordering {
        a_name.cmp(b_name)
    }
}

/// Comparator for Recipe pointers based on name
pub struct RecipeCompare;

impl RecipeCompare {
    pub fn compare(a_name: &str, b_name: &str) -> Ordering {
        a_name.cmp(b_name)
    }
}

/// Load texture from file path
/// Returns true if loaded, false if file doesn't exist or failed to load
/// For now, returns a placeholder since we're using egui which handles textures differently
pub fn load_texture_from_file(_path: &str) -> Option<Vec<u8>> {
    // In a real implementation, this would:
    // 1. Check if file exists
    // 2. Use image crate to load the file
    // 3. Cache the result
    // 4. Return image data
    // For egui, textures are typically handled through TextureHandle
    None
}

/// Update a save JSON to a given version
/// Handles backward compatibility by migrating save data between versions
pub fn update_save(save: &mut Value, to_version: i64) -> bool {
    let mut current_version = save["save_version"].as_i64().unwrap_or(1);

    if current_version == to_version {
        return true;
    }

    // No forward compatibility
    if current_version > to_version {
        return false;
    }

    // Version 1 -> 2: Remove "is_out" from pins as they are now directional
    if current_version == 1 {
        if let Some(links) = save["links"].as_array_mut() {
            for link in links {
                if let Some(start) = link["start"].as_object_mut() {
                    start.remove("is_out");
                }
                if let Some(end) = link["end"].as_object_mut() {
                    end.remove("is_out");
                }
            }
        }
        current_version = 2;
        save["save_version"] = Value::Number(2.into());
    }

    if current_version == to_version {
        return true;
    }

    // Version 2 -> 3: Add num_somersloop to nodes
    if current_version == 2 {
        if let Some(nodes) = save["nodes"].as_array_mut() {
            for node in nodes {
                if node["num_somersloop"].is_null() {
                    node["num_somersloop"] = Value::Number(0.into());
                }
            }
        }
        current_version = 3;
        save["save_version"] = Value::Number(3.into());
    }

    if current_version == to_version {
        return true;
    }

    // Version 3 -> 4: Add built flag to craft nodes
    if current_version == 3 {
        update_nodes_built(save, &mut |node: &mut Value| {
            if let Some(kind) = node["kind"].as_i64() {
                if kind == 0 {
                    // Node::Kind::Craft = 0
                    node["built"] = Value::Bool(false);
                }
            }
        });
        current_version = 4;
        save["save_version"] = Value::Number(4.into());
    }

    if current_version == to_version {
        return true;
    }

    // Version 4 -> 5: Save pin locked state
    if current_version == 4 {
        update_nodes_locked(save, &mut |node: &mut Value| {
            if let Some(kind) = node["kind"].as_i64() {
                match kind {
                    0 => {
                        // Craft
                        node["locked"] = Value::Bool(false);
                    }
                    4 => {
                        // Group
                        node["locked"] = Value::Bool(false);
                        if let Some(children) = node["nodes"].as_array_mut() {
                            for child in children {
                                update_nodes_locked_recursive(child);
                            }
                        }
                    }
                    1 | 2 | 3 | 5 => {
                        // CustomSplitter, Merger, GameSplitter, Sink
                        if let Some(ins) = node["ins"].as_array_mut() {
                            for pin in ins {
                                pin["locked"] = Value::Bool(false);
                            }
                        }
                        if let Some(outs) = node["outs"].as_array_mut() {
                            for pin in outs {
                                pin["locked"] = Value::Bool(false);
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
        current_version = 5;
        save["save_version"] = Value::Number(5.into());
    }

    if current_version == to_version {
        return true;
    }

    false
}

/// Helper to recursively update built flags on nodes
fn update_nodes_built(save: &mut Value, update_fn: &mut dyn FnMut(&mut Value)) {
    if let Some(nodes) = save["nodes"].as_array_mut() {
        for node in nodes {
            update_fn(node);
            if let Some(children) = node["nodes"].as_array_mut() {
                for child in children {
                    update_fn(child);
                    update_nodes_built_recursive(child);
                }
            }
        }
    }
}

/// Recursive helper for updating built flags
fn update_nodes_built_recursive(node: &mut Value) {
    if let Some(children) = node["nodes"].as_array_mut() {
        for child in children {
            if let Some(kind) = child["kind"].as_i64() {
                if kind == 0 {
                    // Craft
                    child["built"] = Value::Bool(false);
                }
            }
            update_nodes_built_recursive(child);
        }
    }
}

/// Helper to recursively update locked flags on nodes
fn update_nodes_locked(save: &mut Value, update_fn: &mut dyn FnMut(&mut Value)) {
    if let Some(nodes) = save["nodes"].as_array_mut() {
        for node in nodes {
            update_fn(node);
        }
    }
}

/// Recursive helper for updating locked flags
fn update_nodes_locked_recursive(node: &mut Value) {
    if let Some(kind) = node["kind"].as_i64() {
        match kind {
            1 | 2 | 3 | 5 => {
                // CustomSplitter, Merger, GameSplitter, Sink
                if let Some(ins) = node["ins"].as_array_mut() {
                    for pin in ins {
                        pin["locked"] = Value::Bool(false);
                    }
                }
                if let Some(outs) = node["outs"].as_array_mut() {
                    for pin in outs {
                        pin["locked"] = Value::Bool(false);
                    }
                }
            }
            4 => {
                // Group
                if let Some(children) = node["nodes"].as_array_mut() {
                    for child in children {
                        update_nodes_locked_recursive(child);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_compare() {
        assert_eq!(ItemCompare::compare("Apple", "Banana"), Ordering::Less);
        assert_eq!(ItemCompare::compare("Banana", "Apple"), Ordering::Greater);
        assert_eq!(ItemCompare::compare("Apple", "Apple"), Ordering::Equal);
    }

    #[test]
    fn test_recipe_compare() {
        assert_eq!(
            RecipeCompare::compare("Recipe A", "Recipe B"),
            Ordering::Less
        );
        assert_eq!(
            RecipeCompare::compare("Recipe B", "Recipe A"),
            Ordering::Greater
        );
        assert_eq!(
            RecipeCompare::compare("Recipe A", "Recipe A"),
            Ordering::Equal
        );
    }

    #[test]
    fn test_save_version_no_change() {
        let mut save = serde_json::json!({
            "save_version": 3,
            "nodes": [],
            "links": []
        });
        assert!(update_save(&mut save, 3));
        assert_eq!(save["save_version"], 3);
    }

    #[test]
    fn test_save_version_forward_fails() {
        let mut save = serde_json::json!({
            "save_version": 3,
            "nodes": [],
            "links": []
        });
        assert!(!update_save(&mut save, 2)); // Can't downgrade
    }

    #[test]
    fn test_save_version_1_to_2() {
        let mut save = serde_json::json!({
            "save_version": 1,
            "links": [
                {
                    "start": { "is_out": false, "id": 1 },
                    "end": { "is_out": true, "id": 2 }
                }
            ]
        });
        assert!(update_save(&mut save, 2));
        assert_eq!(save["save_version"], 2);
        assert!(!save["links"][0]["start"].get("is_out").is_some());
        assert!(!save["links"][0]["end"].get("is_out").is_some());
    }
}
