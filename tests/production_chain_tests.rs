use ficsit_companion_rs::production_app::ProductionApp;
use ficsit_companion_rs::game_data::GameData;

fn load_game_data() -> GameData {
    let json = include_str!("../assets/satisfactory.json");
    let mut game_data = GameData::new();
    game_data.load_from_json(json).expect("Failed to load game data");
    game_data
}

#[test]
fn test_load_production_chain() {
    let json = include_str!("../tests/production_chain.fcs");
    let game_data = load_game_data();
    
    let mut app = ProductionApp::new();
    let result = app.load_from_json(json, Some(&game_data));
    
    assert!(result.is_ok(), "Failed to load production chain: {:?}", result.err());
    
    // Verify basic structure
    assert_eq!(app.node_count(), 8, "Expected 8 nodes");
    assert_eq!(app.links.len(), 6, "Expected 6 links");
}

#[test]
fn test_save_and_reload_production_chain() {
    let original_json = include_str!("../tests/production_chain.fcs");
    let game_data = load_game_data();
    
    // Load original
    let mut app = ProductionApp::new();
    app.load_from_json(original_json, Some(&game_data)).expect("Failed to load original");
    
    // Save to JSON
    let saved_json = app.save_to_json().expect("Failed to save");
    
    // Reload from saved JSON (also needs game data for craft nodes)
    let mut app2 = ProductionApp::new();
    app2.load_from_json(&saved_json, Some(&game_data)).expect("Failed to reload");
    
    // Verify structure is preserved
    assert_eq!(app.node_count(), app2.node_count(), "Node count mismatch after reload");
    assert_eq!(app.links.len(), app2.links.len(), "Link count mismatch after reload");
}

#[test]
fn test_production_chain_structure() {
    let json = include_str!("../tests/production_chain.fcs");
    let game_data = load_game_data();
    
    let mut app = ProductionApp::new();
    app.load_from_json(json, Some(&game_data)).expect("Failed to load");
    
    // The sample file has:
    // - 3 craft nodes (Crystal Oscillator, Quartz Crystal, Cable, Reinforced Iron Plate, Iron Plate, Iron Ingot, Iron Ore)
    // - Actually it's 7 craft nodes
    // - 1 sink node
    // - 6 links
    
    assert_eq!(app.node_count(), 8);
    assert_eq!(app.links.len(), 6);
    
    // Verify all links reference valid pins
    for link in &app.links {
        assert!(app.find_pin_location(link.start_pin_id).is_some(),
            "Link start pin {} not found", link.start_pin_id);
        assert!(app.find_pin_location(link.end_pin_id).is_some(),
            "Link end pin {} not found", link.end_pin_id);
    }
}
