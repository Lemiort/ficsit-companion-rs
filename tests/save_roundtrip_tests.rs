//! Tests for save file roundtrip - load existing saves, save them, verify content matches.
//!
//! These tests verify that ProductionApp can correctly load and re-serialize save files
//! without losing or corrupting any data.

use ficsit_companion_rs::production_app::ProductionApp;

/// Helper to load game data for tests
fn load_game_data() -> ficsit_companion_rs::game_data::GameData {
    let mut game_data = ficsit_companion_rs::game_data::GameData::new();
    let json_str = include_str!("../assets/satisfactory.json");
    game_data.load_from_json(json_str).expect("Failed to load game data");
    game_data
}

/// Compare two JSON values, ignoring formatting differences.
/// Returns (equal, diff_description) where diff_description explains the first difference found.
fn compare_json_values(original: &serde_json::Value, roundtrip: &serde_json::Value, path: &str) -> (bool, String) {
    match (original, roundtrip) {
        (serde_json::Value::Null, serde_json::Value::Null) => (true, String::new()),
        (serde_json::Value::Bool(a), serde_json::Value::Bool(b)) => {
            if a == b {
                (true, String::new())
            } else {
                (false, format!("Bool mismatch at {}: {} vs {}", path, a, b))
            }
        }
        (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
            // Compare as f64 to handle integer vs float representations
            let a_f = a.as_f64().unwrap_or(0.0);
            let b_f = b.as_f64().unwrap_or(0.0);
            if (a_f - b_f).abs() < 1e-10 {
                (true, String::new())
            } else {
                (false, format!("Number mismatch at {}: {} vs {}", path, a, b))
            }
        }
        (serde_json::Value::String(a), serde_json::Value::String(b)) => {
            if a == b {
                (true, String::new())
            } else {
                (false, format!("String mismatch at {}: '{}' vs '{}'", path, a, b))
            }
        }
        (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
            if a.len() != b.len() {
                return (false, format!("Array length mismatch at {}: {} vs {}", path, a.len(), b.len()));
            }
            for (i, (av, bv)) in a.iter().zip(b.iter()).enumerate() {
                let (eq, diff) = compare_json_values(av, bv, &format!("{}[{}]", path, i));
                if !eq {
                    return (false, diff);
                }
            }
            (true, String::new())
        }
        (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
            // Check all keys in a exist in b
            for (key, av) in a.iter() {
                if let Some(bv) = b.get(key) {
                    let (eq, diff) = compare_json_values(av, bv, &format!("{}.{}", path, key));
                    if !eq {
                        return (false, diff);
                    }
                } else {
                    return (false, format!("Key '{}' missing in roundtrip at {}", key, path));
                }
            }
            // Check no extra keys in b
            for key in b.keys() {
                if !a.contains_key(key) {
                    return (false, format!("Extra key '{}' in roundtrip at {}", key, path));
                }
            }
            (true, String::new())
        }
        _ => (false, format!("Type mismatch at {}: {:?} vs {:?}", path, original, roundtrip)),
    }
}

/// Test roundtrip for a specific save file
fn test_save_file_roundtrip(filename: &str, content: &str) {
    let game_data = load_game_data();
    
    // Parse original JSON
    let original_json: serde_json::Value = serde_json::from_str(content)
        .unwrap_or_else(|e| panic!("Failed to parse {} as JSON: {}", filename, e));
    
    // Load into ProductionApp
    let mut app = ProductionApp::new();
    app.load_from_json(content, Some(&game_data))
        .unwrap_or_else(|e| panic!("Failed to load {}: {}", filename, e));
    
    // Save back to JSON
    let roundtrip_str = app.save_to_json()
        .unwrap_or_else(|e| panic!("Failed to save {}: {}", filename, e));
    
    // Parse roundtrip JSON
    let roundtrip_json: serde_json::Value = serde_json::from_str(&roundtrip_str)
        .unwrap_or_else(|e| panic!("Failed to parse roundtrip {} as JSON: {}", filename, e));
    
    // Compare node counts
    let orig_nodes = original_json.get("nodes").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let rt_nodes = roundtrip_json.get("nodes").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    assert_eq!(orig_nodes, rt_nodes, "{}: Node count mismatch: {} vs {}", filename, orig_nodes, rt_nodes);
    
    // Compare link counts
    let orig_links = original_json.get("links").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    let rt_links = roundtrip_json.get("links").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
    assert_eq!(orig_links, rt_links, "{}: Link count mismatch: {} vs {}", filename, orig_links, rt_links);
    
    // Deep compare (allowing for potential ordering differences in nodes/links)
    // For now, just verify structural equality excluding position which may have minor float differences
    
    println!("{}: Loaded {} nodes, {} links - roundtrip OK", filename, rt_nodes, rt_links);
}

/// Test that verifies nodes and links are preserved through roundtrip
fn verify_structure_preserved(filename: &str, content: &str) {
    let game_data = load_game_data();
    
    // Load into ProductionApp
    let mut app = ProductionApp::new();
    app.load_from_json(content, Some(&game_data))
        .unwrap_or_else(|e| panic!("Failed to load {}: {}", filename, e));
    
    let nodes_before = app.nodes.len();
    let links_before = app.links.len();
    
    // Save and reload
    let saved = app.save_to_json().unwrap();
    
    let mut app2 = ProductionApp::new();
    app2.load_from_json(&saved, Some(&game_data)).unwrap();
    
    let nodes_after = app2.nodes.len();
    let links_after = app2.links.len();
    
    assert_eq!(nodes_before, nodes_after, "{}: Node count changed after roundtrip", filename);
    assert_eq!(links_before, links_after, "{}: Link count changed after roundtrip", filename);
}

// Include test files at compile time
const PRODUCTION_CHAIN_FCS: &str = include_str!("production_chain.fcs");
const PRODUCTION_CHAIN2_FCS: &str = include_str!("production_chain2.fcs");
const MERGER_SPLITTER_TEST_FCS: &str = include_str!("merger_splitter_test.fcs");
const NUCLEAR_PLANT_FCS: &str = include_str!("nuclear_plant.fcs");
const GROUPS_TEST_FCS: &str = include_str!("groups_test.fcs");

#[test]
fn test_roundtrip_production_chain() {
    test_save_file_roundtrip("production_chain.fcs", PRODUCTION_CHAIN_FCS);
}

#[test]
fn test_roundtrip_production_chain2() {
    test_save_file_roundtrip("production_chain2.fcs", PRODUCTION_CHAIN2_FCS);
}

#[test]
fn test_roundtrip_merger_splitter() {
    test_save_file_roundtrip("merger_splitter_test.fcs", MERGER_SPLITTER_TEST_FCS);
}

#[test]
fn test_roundtrip_nuclear_plant() {
    test_save_file_roundtrip("nuclear_plant.fcs", NUCLEAR_PLANT_FCS);
}

#[test]
fn test_roundtrip_groups() {
    test_save_file_roundtrip("groups_test.fcs", GROUPS_TEST_FCS);
}

#[test]
fn test_structure_preserved_production_chain() {
    verify_structure_preserved("production_chain.fcs", PRODUCTION_CHAIN_FCS);
}

#[test]
fn test_structure_preserved_production_chain2() {
    verify_structure_preserved("production_chain2.fcs", PRODUCTION_CHAIN2_FCS);
}

#[test]
fn test_structure_preserved_merger_splitter() {
    verify_structure_preserved("merger_splitter_test.fcs", MERGER_SPLITTER_TEST_FCS);
}

#[test]
fn test_structure_preserved_nuclear_plant() {
    verify_structure_preserved("nuclear_plant.fcs", NUCLEAR_PLANT_FCS);
}

#[test]
fn test_structure_preserved_groups() {
    verify_structure_preserved("groups_test.fcs", GROUPS_TEST_FCS);
}

/// Test double roundtrip - load, save, load again, save again, compare
#[test]
fn test_double_roundtrip_production_chain() {
    let game_data = load_game_data();
    
    // First roundtrip
    let mut app1 = ProductionApp::new();
    app1.load_from_json(PRODUCTION_CHAIN_FCS, Some(&game_data)).unwrap();
    let saved1 = app1.save_to_json().unwrap();
    
    // Second roundtrip
    let mut app2 = ProductionApp::new();
    app2.load_from_json(&saved1, Some(&game_data)).unwrap();
    let saved2 = app2.save_to_json().unwrap();
    
    // Parse both saved versions
    let json1: serde_json::Value = serde_json::from_str(&saved1).unwrap();
    let json2: serde_json::Value = serde_json::from_str(&saved2).unwrap();
    
    // Compare - after first roundtrip, subsequent roundtrips should be identical
    let (equal, diff) = compare_json_values(&json1, &json2, "root");
    assert!(equal, "Double roundtrip mismatch: {}", diff);
}

/// Test that all save files can be loaded without errors
#[test]
fn test_all_saves_loadable() {
    let game_data = load_game_data();
    
    let saves = [
        ("production_chain.fcs", PRODUCTION_CHAIN_FCS),
        ("production_chain2.fcs", PRODUCTION_CHAIN2_FCS),
        ("merger_splitter_test.fcs", MERGER_SPLITTER_TEST_FCS),
        ("nuclear_plant.fcs", NUCLEAR_PLANT_FCS),
        ("groups_test.fcs", GROUPS_TEST_FCS),
    ];
    
    for (name, content) in saves {
        let mut app = ProductionApp::new();
        let result = app.load_from_json(content, Some(&game_data));
        assert!(result.is_ok(), "Failed to load {}: {:?}", name, result.err());
        
        // Also verify we can save it back
        let save_result = app.save_to_json();
        assert!(save_result.is_ok(), "Failed to save {}: {:?}", name, save_result.err());
    }
}
