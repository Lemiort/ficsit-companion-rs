use ficsit_companion_rs::game_data::GameData;
use ficsit_companion_rs::production_app::ProductionApp;

fn load_game_data() -> GameData {
    let json = include_str!("../assets/satisfactory.json");
    let mut game_data = GameData::new();
    game_data
        .load_from_json(json)
        .expect("Failed to load game data");
    game_data
}

#[test]
fn test_load_production_chain() {
    let json = include_str!("../tests/production_chain.fcs");
    let game_data = load_game_data();

    let mut app = ProductionApp::new();
    let result = app.load_from_json(json, Some(&game_data));

    assert!(
        result.is_ok(),
        "Failed to load production chain: {:?}",
        result.err()
    );

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
    app.load_from_json(original_json, Some(&game_data))
        .expect("Failed to load original");

    // Save to JSON
    let saved_json = app.save_to_json().expect("Failed to save");

    // Reload from saved JSON (also needs game data for craft nodes)
    let mut app2 = ProductionApp::new();
    app2.load_from_json(&saved_json, Some(&game_data))
        .expect("Failed to reload");

    // Verify structure is preserved
    assert_eq!(
        app.node_count(),
        app2.node_count(),
        "Node count mismatch after reload"
    );
    assert_eq!(
        app.links.len(),
        app2.links.len(),
        "Link count mismatch after reload"
    );
}

#[test]
fn test_production_chain_structure() {
    let json = include_str!("../tests/production_chain.fcs");
    let game_data = load_game_data();

    let mut app = ProductionApp::new();
    app.load_from_json(json, Some(&game_data))
        .expect("Failed to load");

    // The sample file has:
    // - 3 craft nodes (Crystal Oscillator, Quartz Crystal, Cable, Reinforced Iron Plate, Iron Plate, Iron Ingot, Iron Ore)
    // - Actually it's 7 craft nodes
    // - 1 sink node
    // - 6 links

    assert_eq!(app.node_count(), 8);
    assert_eq!(app.links.len(), 6);

    // Verify all links reference valid pins
    for link in &app.links {
        assert!(
            app.find_pin_location(link.start_pin_id).is_some(),
            "Link start pin {} not found",
            link.start_pin_id
        );
        assert!(
            app.find_pin_location(link.end_pin_id).is_some(),
            "Link end pin {} not found",
            link.end_pin_id
        );
    }
}

#[test]
fn test_positions_preserved_on_save_reload() {
    let json = include_str!("../tests/production_chain.fcs");
    let game_data = load_game_data();

    let mut app = ProductionApp::new();
    app.load_from_json(json, Some(&game_data))
        .expect("Failed to load");

    // Mutate positions to known values per index
    for i in 0..app.node_count() {
        let new_pos = (i as f32 * 10.0 + 1.0, i as f32 * 20.0 + 2.0);
        let node_id = match app.find_node_by_index(i) {
            Some(id) => id,
            None => continue,
        };
        app.set_node_position(node_id, new_pos).expect("set position failed");
    }

    let saved = app.save_to_json().expect("Failed to save");
    let mut app2 = ProductionApp::new();
    app2.load_from_json(&saved, Some(&game_data))
        .expect("Failed to reload");

    // Compare positions by index
    for i in 0..app.node_count() {
        let p1 = app.get_node_position(i).expect("pos missing");
        let p2 = app2.get_node_position(i).expect("pos missing after reload");
        assert_eq!(p1, p2, "Position mismatch at index {}: {:?} vs {:?}", i, p1, p2);
    }
}
