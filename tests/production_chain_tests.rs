use ficsit_companion_rs::game_data::GameData;
use ficsit_companion_rs::pin::PinDirection;
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
        app.set_node_position(node_id, new_pos)
            .expect("set position failed");
    }

    let saved = app.save_to_json().expect("Failed to save");
    let mut app2 = ProductionApp::new();
    app2.load_from_json(&saved, Some(&game_data))
        .expect("Failed to reload");

    // Compare positions by index
    for i in 0..app.node_count() {
        let p1 = app.get_node_position(i).expect("pos missing");
        let p2 = app2.get_node_position(i).expect("pos missing after reload");
        assert_eq!(
            p1, p2,
            "Position mismatch at index {}: {:?} vs {:?}",
            i, p1, p2
        );
    }
}

#[test]
fn test_iron_ingot_node_rate() {
    // Load file at runtime to avoid include_str path issues with spaces
    let json = include_str!("../tests/production_chain.fcs");
    let game_data = load_game_data();

    let mut app = ProductionApp::new();
    app.load_from_json(&json, Some(&game_data))
        .expect("Failed to load production_chain copy.fcs");

    // Find node id for the "Iron Ingot" craft node
    let mut iron_node_id = None;
    for i in 0..app.node_count() {
        if let Some(node_id) = app.find_node_by_index(i) {
            if let Some(label) = app.get_node_label(node_id) {
                if label == "Iron Ingot" {
                    iron_node_id = Some(node_id);
                    break;
                }
            }
        }
    }

    let node_id = iron_node_id.expect("Iron Ingot node not found");

    // Get building info (current rate string and building name)
    let (rate_str, _building) = app
        .get_node_building_info(node_id)
        .expect("Failed to get building info for Iron Ingot node");

    // Verify we got the rate string (should be non-empty)
    assert!(!rate_str.is_empty(), "Rate string should not be empty");
}

#[test]
fn test_power_generator_detection() {
    let game_data = load_game_data();
    let mut app = ProductionApp::new();

    // Test Iron Plate (Constructor - should NOT be a power generator)
    let iron_plate_id = app
        .add_craft_node("Iron Plate", &game_data)
        .expect("Failed to create Iron Plate node");
    let is_power_gen = app.get_node_is_power_generator(iron_plate_id);
    assert!(
        !is_power_gen,
        "Iron Plate (Constructor) should NOT be detected as a power generator"
    );

    // Test Power (Coal) - a power generator recipe that SHOULD be detected as a power generator
    let power_coal_id = app
        .add_craft_node("Power (Coal)", &game_data)
        .expect("Failed to create Power (Coal) node");
    let is_power_coal_gen = app.get_node_is_power_generator(power_coal_id);
    assert!(
        is_power_coal_gen,
        "Power (Coal) SHOULD be detected as a power generator (negative power generation)"
    );

    // Verify power generator buildings have negative power in the game data
    if let Some(coal_gen_building) = game_data.buildings.get("Coal-Powered Generator") {
        assert!(
            coal_gen_building.power < 0.0,
            "Coal-Powered Generator building should have negative power (-75.0)"
        );
    } else {
        panic!("Coal-Powered Generator building not found in game data");
    }

    if let Some(fuel_gen_building) = game_data.buildings.get("Fuel-Powered Generator") {
        assert!(
            fuel_gen_building.power < 0.0,
            "Fuel-Powered Generator building should have negative power (-250.0)"
        );
    } else {
        panic!("Fuel-Powered Generator building not found in game data");
    }

    if let Some(nuclear_building) = game_data.buildings.get("Nuclear Power Plant") {
        assert!(
            nuclear_building.power < 0.0,
            "Nuclear Power Plant building should have negative power (-2500.0)"
        );
    } else {
        panic!("Nuclear Power Plant building not found in game data");
    }
}

/// Regression test: Connecting a locked craft node output to a new merger input
/// should propagate the rate correctly without solver errors.
///
/// Scenario:
/// 1. Create "Encased Uranium Cell" craft node (has outputs with rates like 10, 25, etc.)
/// 2. Lock the craft node (all pins become locked)
/// 3. Create a merger node  
/// 4. Connect craft output (e.g., rate=10) to merger input
///
/// The merger input should get rate=10, and no solver error should occur.
#[test]
fn test_connect_locked_craft_to_merger() {
    let game_data = load_game_data();
    let mut app = ProductionApp::new();

    // Create "Encased Uranium Cell" craft node
    // This recipe has:
    //   - Inputs: Uranium (50), Concrete (15), Electromagnetic Control Rod (25), Sulfuric Acid (40)
    //   - Outputs: Encased Uranium Cell (50), Sulfuric Acid (10)
    let craft_id = app
        .add_craft_node("Encased Uranium Cell", &game_data)
        .expect("Failed to create Encased Uranium Cell node");

    // Lock the craft node (simulates RMB menu -> lock)
    app.set_node_locked(craft_id, true)
        .expect("Failed to lock craft node");

    // Verify all pins are now locked
    let (ins_locked, outs_locked) = app
        .get_node_pin_locked_flags(craft_id)
        .expect("Failed to get locked flags");
    assert!(
        ins_locked.iter().all(|b| *b),
        "All input pins should be locked"
    );
    assert!(
        outs_locked.iter().all(|b| *b),
        "All output pins should be locked"
    );

    // Create a merger node
    let merger_id = app.add_merger_node();

    // Get pin IDs:
    // - Craft output index 1 (Sulfuric Acid, rate=10)
    // - Merger input index 1
    let craft_out_pin = app
        .get_pin_id(craft_id, PinDirection::Output, 1)
        .expect("Failed to get craft output pin");
    let merger_in_pin = app
        .get_pin_id(merger_id, PinDirection::Input, 1)
        .expect("Failed to get merger input pin");

    // Connect craft output -> merger input
    // This should NOT fail with solver error
    let result = app.create_link(craft_out_pin, merger_in_pin);
    assert!(
        result.is_ok(),
        "create_link should succeed, got error: {:?}",
        result.err()
    );

    // Check that the link was created (even if there was a propagation warning)
    let (link_id, _warning) = result.unwrap();
    assert!(link_id > 0, "Link ID should be positive");

    // Verify merger input now has rate=10
    let (merger_ins, _merger_outs) = app
        .get_node_pin_rates(merger_id)
        .expect("Failed to get merger pin rates");

    // The connected input (index 1) should have rate 10
    assert_eq!(merger_ins.len(), 2, "Merger should have 2 input pins");

    // Check rate of connected input
    let rate_str = merger_ins[1].as_ref().expect("Input 1 rate should be Some");
    assert!(
        rate_str == "10" || rate_str == "10/1",
        "Merger input 1 should have rate 10, got: {}",
        rate_str
    );

    // IMPORTANT: Merger's connected pin should also be locked since it's determined by locked source
    let (merger_ins_locked, merger_outs_locked) = app
        .get_node_pin_locked_flags(merger_id)
        .expect("Failed to get merger locked flags");

    // Only the connected input (index 1) should be locked, not the whole node
    assert!(
        !merger_ins_locked[0],
        "Merger input 0 (unconnected) should NOT be locked"
    );
    assert!(
        merger_ins_locked[1],
        "Merger input 1 (connected to locked craft) should be locked"
    );
    assert!(
        !merger_outs_locked[0],
        "Merger output 0 (unconnected) should NOT be locked"
    );
}

/// Regression test: Loop scenario - craft -> merger -> craft (same node)
///
/// Scenario:
/// 1. Create "Encased Uranium Cell" craft node
/// 2. Lock the craft node  
/// 3. Connect craft output (Sulfuric Acid, rate=10) to merger input
/// 4. Connect merger output back to craft input (Sulfuric Acid, rate=40)
///
/// The merger should compute: output = 40 (craft input rate)
/// Since one input is 10, the other input should be 30 (or remain 0 if unconnected and rate propagates)
#[test]
fn test_connect_locked_craft_merger_loop() {
    let game_data = load_game_data();
    let mut app = ProductionApp::new();

    // Create "Encased Uranium Cell" craft node
    let craft_id = app
        .add_craft_node("Encased Uranium Cell", &game_data)
        .expect("Failed to create Encased Uranium Cell node");

    // Lock the craft node
    app.set_node_locked(craft_id, true)
        .expect("Failed to lock craft node");

    // Create a merger node
    let merger_id = app.add_merger_node();

    // Get pin IDs:
    // - Craft output index 1 (Sulfuric Acid, rate=10)
    // - Craft input index 2 (Sulfuric Acid, rate=40)
    // - Merger input index 1
    // - Merger output index 0
    let craft_out_pin = app
        .get_pin_id(craft_id, PinDirection::Output, 1)
        .expect("Failed to get craft output pin");
    let craft_in_pin = app
        .get_pin_id(craft_id, PinDirection::Input, 2)
        .expect("Failed to get craft input pin");
    let merger_in_pin = app
        .get_pin_id(merger_id, PinDirection::Input, 1)
        .expect("Failed to get merger input pin");
    let merger_out_pin = app
        .get_pin_id(merger_id, PinDirection::Output, 0)
        .expect("Failed to get merger output pin");

    // First connection: craft output -> merger input
    let result1 = app.create_link(craft_out_pin, merger_in_pin);
    assert!(
        result1.is_ok(),
        "First create_link should succeed, got error: {:?}",
        result1.err()
    );

    // Second connection: merger output -> craft input (creates a loop)
    let result2 = app.create_link(merger_out_pin, craft_in_pin);
    assert!(
        result2.is_ok(),
        "Second create_link (loop) should succeed, got error: {:?}",
        result2.err()
    );

    // Verify rates after both connections
    let (merger_ins, merger_outs) = app
        .get_node_pin_rates(merger_id)
        .expect("Failed to get merger pin rates");

    // Merger output should be 40 (matches craft input)
    let out_rate_str = merger_outs[0].as_ref().expect("Output rate should be Some");
    assert!(
        out_rate_str == "40" || out_rate_str == "40/1",
        "Merger output should have rate 40 (matching craft input), got: {}",
        out_rate_str
    );

    // Connected merger input should be 10 (from craft output)
    let in1_rate_str = merger_ins[1].as_ref().expect("Input 1 rate should be Some");
    assert!(
        in1_rate_str == "10" || in1_rate_str == "10/1",
        "Merger input 1 should have rate 10, got: {}",
        in1_rate_str
    );

    // IMPORTANT: ALL merger pins should be locked since the solution is uniquely determined
    // When output=40 (locked) and input1=10 (locked), input0=30 is the only solution
    let (merger_ins_locked, merger_outs_locked) = app
        .get_node_pin_locked_flags(merger_id)
        .expect("Failed to get merger locked flags");

    // Connected input (index 1) should be locked
    assert!(
        merger_ins_locked[1],
        "Merger input 1 (connected to locked craft output) should be locked"
    );
    // Connected output (index 0) should be locked
    assert!(
        merger_outs_locked[0],
        "Merger output 0 (connected to locked craft input) should be locked"
    );
    // Unconnected input should ALSO be locked since its value is uniquely determined (40 - 10 = 30)
    assert!(
        merger_ins_locked[0],
        "Merger input 0 should be auto-locked since solution is unique"
    );

    // Verify the unconnected input has the correct rate (30)
    let in0_rate_str = merger_ins[0].as_ref().expect("Input 0 rate should be Some");
    assert!(
        in0_rate_str == "30" || in0_rate_str == "30/1",
        "Merger input 0 should have rate 30 (40 - 10), got: {}",
        in0_rate_str
    );
}
