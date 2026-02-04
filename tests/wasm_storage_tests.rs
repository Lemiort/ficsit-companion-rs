//! WASM-specific tests for localStorage save/load functionality.
//!
//! Run with: `wasm-pack test --headless --chrome` or `wasm-pack test --headless --firefox`

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;
use ficsit_companion_rs::production_app::ProductionApp;

wasm_bindgen_test_configure!(run_in_browser);

/// Helper to get localStorage
fn get_storage() -> web_sys::Storage {
    web_sys::window()
        .expect("no window")
        .local_storage()
        .expect("localStorage error")
        .expect("no localStorage")
}

/// Helper to clear all test saves from localStorage
fn clear_test_saves() {
    let storage = get_storage();
    let mut keys_to_remove = Vec::new();
    
    if let Ok(len) = storage.length() {
        for i in 0..len {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with("saves/test_") {
                    keys_to_remove.push(key);
                }
            }
        }
    }
    
    for key in keys_to_remove {
        let _ = storage.remove_item(&key);
    }
}

#[wasm_bindgen_test]
fn test_localstorage_save_and_load() {
    clear_test_saves();
    
    let storage = get_storage();
    let key = "saves/test_save_load.fcs";
    let content = r#"{"save_version":1,"nodes":[],"links":[]}"#;
    
    // Save
    storage.set_item(key, content).expect("set_item failed");
    
    // Load
    let loaded = storage.get_item(key).expect("get_item failed");
    assert_eq!(loaded, Some(content.to_string()));
    
    // Cleanup
    storage.remove_item(key).expect("remove_item failed");
    
    // Verify removal
    let after_remove = storage.get_item(key).expect("get_item failed");
    assert_eq!(after_remove, None);
}

#[wasm_bindgen_test]
fn test_localstorage_list_saves() {
    clear_test_saves();
    
    let storage = get_storage();
    
    // Create multiple test saves
    storage.set_item("saves/test_alpha.fcs", "{}").unwrap();
    storage.set_item("saves/test_beta.fcs", "{}").unwrap();
    storage.set_item("saves/test_gamma.fcs", "{}").unwrap();
    storage.set_item("other_key", "{}").unwrap(); // Should not be included
    
    // Enumerate saves
    let mut found_saves = Vec::new();
    if let Ok(len) = storage.length() {
        for i in 0..len {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with("saves/") && key.ends_with(".fcs") {
                    let name = key
                        .strip_prefix("saves/")
                        .and_then(|s| s.strip_suffix(".fcs"))
                        .map(|s| s.to_owned());
                    if let Some(n) = name {
                        found_saves.push(n);
                    }
                }
            }
        }
    }
    
    // Verify we found our test saves
    assert!(found_saves.contains(&"test_alpha".to_string()));
    assert!(found_saves.contains(&"test_beta".to_string()));
    assert!(found_saves.contains(&"test_gamma".to_string()));
    
    // Cleanup
    storage.remove_item("saves/test_alpha.fcs").unwrap();
    storage.remove_item("saves/test_beta.fcs").unwrap();
    storage.remove_item("saves/test_gamma.fcs").unwrap();
    storage.remove_item("other_key").unwrap();
}

#[wasm_bindgen_test]
fn test_localstorage_overwrite_save() {
    clear_test_saves();
    
    let storage = get_storage();
    let key = "saves/test_overwrite.fcs";
    
    // Initial save
    storage.set_item(key, "version1").unwrap();
    assert_eq!(storage.get_item(key).unwrap(), Some("version1".to_string()));
    
    // Overwrite
    storage.set_item(key, "version2").unwrap();
    assert_eq!(storage.get_item(key).unwrap(), Some("version2".to_string()));
    
    // Cleanup
    storage.remove_item(key).unwrap();
}

#[wasm_bindgen_test]
fn test_localstorage_delete_save() {
    clear_test_saves();
    
    let storage = get_storage();
    let key = "saves/test_delete.fcs";
    
    // Create
    storage.set_item(key, "content").unwrap();
    assert!(storage.get_item(key).unwrap().is_some());
    
    // Delete
    storage.remove_item(key).unwrap();
    assert!(storage.get_item(key).unwrap().is_none());
    
    // Delete non-existent (should not error)
    storage.remove_item(key).unwrap();
}

#[wasm_bindgen_test]
fn test_production_app_serialization_roundtrip() {
    // Test that ProductionApp can serialize and deserialize correctly
    // This tests the JSON format compatibility
    
    let mut app = ficsit_companion_rs::production_app::ProductionApp::new();
    
    // Serialize empty state
    let json = app.save_to_json().expect("serialize failed");
    
    // Verify it's valid JSON
    assert!(json.contains("save_version"));
    assert!(json.contains("nodes"));
    assert!(json.contains("links"));
    
    // Deserialize back
    app.load_from_json(&json, None).expect("deserialize failed");
    
    // Verify state is still empty
    assert!(app.nodes.is_empty());
    assert!(app.links.is_empty());
}

#[wasm_bindgen_test]
fn test_localstorage_json_content() {
    clear_test_saves();
    
    let storage = get_storage();
    let key = "saves/test_json.fcs";
    
    // Create a ProductionApp and save its JSON
    let app = ficsit_companion_rs::production_app::ProductionApp::new();
    let json = app.save_to_json().expect("serialize failed");
    
    // Save to localStorage
    storage.set_item(key, &json).unwrap();
    
    // Load and verify
    let loaded = storage.get_item(key).unwrap().expect("not found");
    
    // Parse as JSON to verify format
    let parsed: serde_json::Value = serde_json::from_str(&loaded).expect("invalid JSON");
    assert!(parsed.get("save_version").is_some());
    assert!(parsed.get("nodes").is_some());
    assert!(parsed.get("links").is_some());
    
    // Cleanup
    storage.remove_item(key).unwrap();
}

#[wasm_bindgen_test]
fn test_localstorage_special_characters_in_name() {
    clear_test_saves();
    
    let storage = get_storage();
    
    // Test with special characters in save name
    let key = "saves/test_special äöü 日本語.fcs";
    let content = "test content";
    
    storage.set_item(key, content).unwrap();
    let loaded = storage.get_item(key).unwrap();
    assert_eq!(loaded, Some(content.to_string()));
    
    storage.remove_item(key).unwrap();
}

#[wasm_bindgen_test]
fn test_localstorage_large_save() {
    clear_test_saves();
    
    let storage = get_storage();
    let key = "saves/test_large.fcs";
    
    // Create a large-ish content (100KB of JSON-like data)
    let large_content = format!(
        r#"{{"data": "{}"}}"#, 
        "x".repeat(100_000)
    );
    
    storage.set_item(key, &large_content).unwrap();
    let loaded = storage.get_item(key).unwrap().expect("not found");
    assert_eq!(loaded.len(), large_content.len());
    
    storage.remove_item(key).unwrap();
}

// Include real test files at compile time
const PRODUCTION_CHAIN_FCS: &str = include_str!("production_chain.fcs");
const PRODUCTION_CHAIN2_FCS: &str = include_str!("production_chain2.fcs");
const MERGER_SPLITTER_TEST_FCS: &str = include_str!("merger_splitter_test.fcs");
const NUCLEAR_PLANT_FCS: &str = include_str!("nuclear_plant.fcs");
const GROUPS_TEST_FCS: &str = include_str!("groups_test.fcs");

/// Helper to load game data for tests
fn load_game_data() -> ficsit_companion_rs::game_data::GameData {
    let mut game_data = ficsit_companion_rs::game_data::GameData::new();
    let json_str = include_str!("../assets/satisfactory.json");
    game_data.load_from_json(json_str).expect("Failed to load game data");
    game_data
}

#[wasm_bindgen_test]
fn test_localstorage_save_real_production_chain() {
    clear_test_saves();
    let storage = get_storage();
    let game_data = load_game_data();
    
    // Load the real save file
    let mut app = ProductionApp::new();
    app.load_from_json(PRODUCTION_CHAIN_FCS, Some(&game_data)).expect("load failed");
    
    // Save to JSON
    let json = app.save_to_json().expect("save failed");
    
    // Store in localStorage
    let key = "saves/test_production_chain.fcs";
    storage.set_item(key, &json).expect("localStorage set failed");
    
    // Load from localStorage
    let loaded = storage.get_item(key).unwrap().expect("not found");
    
    // Load back into a new app
    let mut app2 = ProductionApp::new();
    app2.load_from_json(&loaded, Some(&game_data)).expect("reload failed");
    
    // Verify counts match
    assert_eq!(app.nodes.len(), app2.nodes.len(), "Node count mismatch");
    assert_eq!(app.links.len(), app2.links.len(), "Link count mismatch");
    
    storage.remove_item(key).unwrap();
}

#[wasm_bindgen_test]
fn test_localstorage_save_nuclear_plant() {
    clear_test_saves();
    let storage = get_storage();
    let game_data = load_game_data();
    
    // Load the nuclear plant save
    let mut app = ProductionApp::new();
    app.load_from_json(NUCLEAR_PLANT_FCS, Some(&game_data)).expect("load failed");
    
    let nodes_original = app.nodes.len();
    let links_original = app.links.len();
    
    // Save to JSON
    let json = app.save_to_json().expect("save failed");
    
    // Store in localStorage
    let key = "saves/test_nuclear_plant.fcs";
    storage.set_item(key, &json).expect("localStorage set failed");
    
    // Load from localStorage
    let loaded = storage.get_item(key).unwrap().expect("not found");
    
    // Load back into a new app
    let mut app2 = ProductionApp::new();
    app2.load_from_json(&loaded, Some(&game_data)).expect("reload failed");
    
    // Verify counts match
    assert_eq!(nodes_original, app2.nodes.len(), "Node count mismatch: {} vs {}", nodes_original, app2.nodes.len());
    assert_eq!(links_original, app2.links.len(), "Link count mismatch: {} vs {}", links_original, app2.links.len());
    
    storage.remove_item(key).unwrap();
}

#[wasm_bindgen_test]
fn test_localstorage_roundtrip_all_saves() {
    clear_test_saves();
    let storage = get_storage();
    let game_data = load_game_data();
    
    let saves = [
        ("test_prod1", PRODUCTION_CHAIN_FCS),
        ("test_prod2", PRODUCTION_CHAIN2_FCS),
        ("test_merger", MERGER_SPLITTER_TEST_FCS),
        ("test_nuclear", NUCLEAR_PLANT_FCS),
        ("test_groups", GROUPS_TEST_FCS),
    ];
    
    for (name, content) in saves {
        let key = format!("saves/{}.fcs", name);
        
        // Load original
        let mut app = ProductionApp::new();
        app.load_from_json(content, Some(&game_data))
            .unwrap_or_else(|e| panic!("Failed to load {}: {}", name, e));
        
        let nodes_orig = app.nodes.len();
        let links_orig = app.links.len();
        
        // Save to JSON
        let json = app.save_to_json()
            .unwrap_or_else(|e| panic!("Failed to save {}: {}", name, e));
        
        // Store in localStorage
        storage.set_item(&key, &json)
            .unwrap_or_else(|e| panic!("Failed to store {} in localStorage: {:?}", name, e));
        
        // Load from localStorage
        let loaded = storage.get_item(&key)
            .unwrap_or_else(|e| panic!("Failed to get {} from localStorage: {:?}", name, e))
            .unwrap_or_else(|| panic!("{} not found in localStorage", name));
        
        // Reload into new app
        let mut app2 = ProductionApp::new();
        app2.load_from_json(&loaded, Some(&game_data))
            .unwrap_or_else(|e| panic!("Failed to reload {}: {}", name, e));
        
        // Verify
        assert_eq!(nodes_orig, app2.nodes.len(), "{}: Node count mismatch", name);
        assert_eq!(links_orig, app2.links.len(), "{}: Link count mismatch", name);
        
        // Cleanup
        storage.remove_item(&key).unwrap();
    }
}

#[wasm_bindgen_test]
fn test_localstorage_double_roundtrip() {
    clear_test_saves();
    let storage = get_storage();
    let game_data = load_game_data();
    
    // First roundtrip
    let mut app1 = ProductionApp::new();
    app1.load_from_json(PRODUCTION_CHAIN_FCS, Some(&game_data)).unwrap();
    let json1 = app1.save_to_json().unwrap();
    
    storage.set_item("saves/test_rt1.fcs", &json1).unwrap();
    let loaded1 = storage.get_item("saves/test_rt1.fcs").unwrap().unwrap();
    
    // Second roundtrip
    let mut app2 = ProductionApp::new();
    app2.load_from_json(&loaded1, Some(&game_data)).unwrap();
    let json2 = app2.save_to_json().unwrap();
    
    storage.set_item("saves/test_rt2.fcs", &json2).unwrap();
    let loaded2 = storage.get_item("saves/test_rt2.fcs").unwrap().unwrap();
    
    // Parse and compare
    let parsed1: serde_json::Value = serde_json::from_str(&loaded1).unwrap();
    let parsed2: serde_json::Value = serde_json::from_str(&loaded2).unwrap();
    
    // Compare node and link counts (should be identical after first normalization)
    let nodes1 = parsed1.get("nodes").unwrap().as_array().unwrap().len();
    let nodes2 = parsed2.get("nodes").unwrap().as_array().unwrap().len();
    let links1 = parsed1.get("links").unwrap().as_array().unwrap().len();
    let links2 = parsed2.get("links").unwrap().as_array().unwrap().len();
    
    assert_eq!(nodes1, nodes2, "Node count differs between roundtrips");
    assert_eq!(links1, links2, "Link count differs between roundtrips");
    
    // Cleanup
    storage.remove_item("saves/test_rt1.fcs").unwrap();
    storage.remove_item("saves/test_rt2.fcs").unwrap();
}
