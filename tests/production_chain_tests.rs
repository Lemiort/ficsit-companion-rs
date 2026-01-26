use ficsit_companion_rs::FractionalNumber;
use ficsit_companion_rs::game_data::GameData;
use ficsit_companion_rs::node;
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
fn test_pin_rates_preserved_on_save_reload() {
    let game_data = load_game_data();
    let mut app = ProductionApp::new();

    // Create a craft node and set its output pin rate to 10
    let node_id = app
        .add_craft_node("Encased Uranium Cell", &game_data)
        .expect("Failed to create craft node");

    // Ensure at least one output pin is present
    let out_pin_id = app
        .get_pin_id(node_id, PinDirection::Output, 0)
        .expect("Failed to get craft output pin");

    // Set output pin rate to 10 (this updates node rate and propagates)
    app.set_pin_rate(
        node_id,
        PinDirection::Output,
        0,
        FractionalNumber::new(10, 1),
    )
    .expect("Failed to set pin rate");

    // Verify current rate before save
    let (_ins, outs) = app
        .get_node_pin_rates(node_id)
        .expect("Failed to get pin rates");
    let before = outs[0].as_ref().expect("Output rate should be Some");
    assert!(
        before == "10" || before == "10/1",
        "Unexpected rate before save: {}",
        before
    );

    // Save and reload
    let saved = app.save_to_json().expect("Failed to save");
    let mut app2 = ProductionApp::new();
    app2.load_from_json(&saved, Some(&game_data))
        .expect("Failed to reload");

    // Find the same craft node by label in reloaded app
    let mut reloaded_node_id = None;
    for i in 0..app2.node_count() {
        if let Some(node_id) = app2.find_node_by_index(i) {
            if let Some(label) = app2.get_node_label(node_id) {
                if label == "Encased Uranium Cell" {
                    reloaded_node_id = Some(node_id);
                    break;
                }
            }
        }
    }
    let rnode = reloaded_node_id.expect("Reloaded craft node not found");

    // Verify the output rate was preserved after reload
    let (_rins, routs) = app2
        .get_node_pin_rates(rnode)
        .expect("Failed to get pin rates after reload");
    let after = routs[0]
        .as_ref()
        .expect("Output rate should be Some after reload");
    assert!(
        after == "10" || after == "10/1",
        "Rate not preserved after reload: {}",
        after
    );
}

#[test]
fn test_merger_splitter_import_compatibility() {
    // Load the C++ exported example and verify organizer pins are restored
    let json = include_str!("../tests/merger_splitter_test.fcs");
    let game_data = load_game_data();

    let mut app = ProductionApp::new();
    app.load_from_json(json, Some(&game_data))
        .expect("Failed to load merger_splitter_test.fcs");

    // Node index 9 in the file is the Merger (kind=2)
    let merger_node_id = app.find_node_by_index(9).expect("Merger node missing");
    // Merger should have 2 inputs and 1 output
    let (ins, outs) = app
        .get_node_pin_rates(merger_node_id)
        .expect("Failed to get merger pin rates");
    assert_eq!(ins.len(), 2, "Merger should have 2 inputs");
    assert_eq!(outs.len(), 1, "Merger should have 1 output");

    // Rates should match saved values (10, 30 -> out 40)
    assert_eq!(ins[0].as_ref().unwrap(), "10");
    assert_eq!(ins[1].as_ref().unwrap(), "30");
    assert_eq!(outs[0].as_ref().unwrap(), "40");

    // Node index 10 is a CustomSplitter (kind=1) with 1 input, 3 outputs
    let splitter_node_id = app.find_node_by_index(10).expect("Splitter node missing");
    let (s_ins, s_outs) = app
        .get_node_pin_rates(splitter_node_id)
        .expect("Failed to get splitter pin rates");
    assert_eq!(s_ins.len(), 1, "Splitter should have 1 input");
    assert_eq!(s_outs.len(), 3, "Splitter should have 3 outputs");

    // Output rates should match saved values (15, 36, 0)
    assert_eq!(s_outs[0].as_ref().unwrap(), "15");
    assert_eq!(s_outs[1].as_ref().unwrap(), "36");
    assert_eq!(s_outs[2].as_ref().unwrap(), "0");
}

#[test]
fn test_organizer_ins_outs_saved() {
    let mut app = ProductionApp::new();

    // Create a merger and set its item name so pins have item metadata
    let merger_id = app.add_merger_node();
    app.set_node_item_name(merger_id, Some("Iron Ore".to_string()))
        .expect("Failed to set item name");

    // Lock the first input pin (isolated node -> only that pin gets locked)
    let in_pin_id = app
        .get_pin_id(merger_id, PinDirection::Input, 0)
        .expect("Failed to get input pin");
    app.set_pin_locked(in_pin_id, true)
        .expect("Failed to lock pin");

    // Save to JSON and inspect serialized structure
    let saved = app.save_to_json().expect("Failed to save");
    let file: ficsit_companion_rs::serialization::ProductionChainFile =
        serde_json::from_str(&saved).expect("Failed to parse saved JSON");

    // Find the organizer node we created in the saved file
    let maybe_org = file.nodes.iter().find_map(|node| {
        if let ficsit_companion_rs::serialization::SerializedNode::Organizer(org) = node {
            Some(org)
        } else {
            None
        }
    });

    let org = maybe_org.expect("No organizer node found in saved file");
    assert!(org.ins.is_some(), "Expected ins to be emitted");
    assert!(org.outs.is_some(), "Expected outs to be emitted");

    let ins = org.ins.as_ref().unwrap();
    let outs = org.outs.as_ref().unwrap();
    assert_eq!(ins.len(), 2, "Expected 2 input entries");
    assert_eq!(outs.len(), 1, "Expected 1 output entry");

    // First input should have item "Iron Ore" and be locked
    assert_eq!(ins[0].item.as_ref().unwrap(), "Iron Ore");
    assert!(ins[0].locked, "First input should be locked");

    // Outputs should have item name set but not locked
    assert_eq!(outs[0].item.as_ref().unwrap(), "Iron Ore");
    assert!(!outs[0].locked, "Output should not be locked");
}

#[test]
fn test_organizer_item_propagates_to_pins_on_load() {
    // Create a saved file which specifies the organizer item but not explicit ins/outs
    let json = r#"{
        "game_version": "1.0",
        "save_version": 5,
        "nodes": [
            { "kind": 2, "pos": { "x": 0.0, "y": 0.0 }, "item": "Sulfur" }
        ],
        "links": []
    }"#;

    let mut app = ProductionApp::new();
    app.load_from_json(json, None).expect("Failed to load");

    // After load, organizer should have default pins and item propagated to pin names
    let node_index = 0;
    let node_id = app.nodes[node_index]
        .downcast_ref::<node::OrganizerNode>()
        .unwrap()
        .base
        .id;
    let (ins, outs) = app
        .get_node_pin_item_names(node_id)
        .expect("Expected pin names");
    assert!(
        ins.iter()
            .all(|n| n.as_ref().map(|s| s == "Sulfur").unwrap_or(false))
    );
    assert!(
        outs.iter()
            .all(|n| n.as_ref().map(|s| s == "Sulfur").unwrap_or(false))
    );
}

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
