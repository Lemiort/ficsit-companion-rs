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
    app.set_node_item_name(merger_id, Some("Iron Ore".to_owned()))
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

#[test]
fn test_lock_sink_input_pin() {
    let mut app = ProductionApp::new();

    // Create a sink node and verify initial lock state is unlocked
    let sink_id = app.add_sink_node();

    // Add a second input so we have two independent pins
    app.add_input_pin_to_node(sink_id)
        .expect("Failed to add second sink input");

    let (ins_locked, _outs_locked) = app
        .get_node_pin_locked_flags(sink_id)
        .expect("Failed to get locked flags for sink");
    assert_eq!(ins_locked.len(), 2);
    assert!(!ins_locked[0], "Sink input 0 should initially be unlocked");
    assert!(!ins_locked[1], "Sink input 1 should initially be unlocked");

    // Lock the first sink input pin (simulate UI pin lock)
    let pin0_id = app
        .get_pin_id(sink_id, PinDirection::Input, 0)
        .expect("Failed to get sink input pin 0 id");
    app.set_pin_locked(pin0_id, true)
        .expect("Failed to set pin locked");

    let (ins_locked_after, _outs_locked_after) = app
        .get_node_pin_locked_flags(sink_id)
        .expect("Failed to get locked flags for sink after lock");
    // First pin should be locked, second should remain unlocked
    assert!(
        ins_locked_after[0],
        "Sink input 0 should be locked after set_pin_locked"
    );
    assert!(
        !ins_locked_after[1],
        "Sink input 1 should remain unlocked when locking pin 0"
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

    // Debug after first link
    let (m_ins1, m_outs1) = app.get_node_pin_rates(merger_id).unwrap();
    let (m_ins_locked1, m_outs_locked1) = app.get_node_pin_locked_flags(merger_id).unwrap();
    println!("After first link:");
    println!("  merger ins: {:?}, locked: {:?}", m_ins1, m_ins_locked1);
    println!("  merger outs: {:?}, locked: {:?}", m_outs1, m_outs_locked1);

    // Second connection: merger output -> craft input (creates a loop)
    let result2 = app.create_link(merger_out_pin, craft_in_pin);
    if let Err(ref e) = result2 {
        println!("Second link error: {:?}", e);
    }
    if let Ok((_, ref warn)) = result2 {
        println!("Second link warning: {:?}", warn);
    }
    assert!(
        result2.is_ok(),
        "Second create_link (loop) should succeed, got error: {:?}",
        result2.err()
    );

    // Debug after second link
    let (m_ins2, m_outs2) = app.get_node_pin_rates(merger_id).unwrap();
    let (m_ins_locked2, m_outs_locked2) = app.get_node_pin_locked_flags(merger_id).unwrap();
    println!("After second link:");
    println!("  merger ins: {:?}, locked: {:?}", m_ins2, m_ins_locked2);
    println!("  merger outs: {:?}, locked: {:?}", m_outs2, m_outs_locked2);

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

/// Test connecting an unlocked craft output to a merger input.
/// The merger input should receive the craft's output rate.
#[test]
fn test_connect_craft_output_to_merger_input() {
    let game_data = load_game_data();
    let mut app = ProductionApp::new();

    // Create a craft node with rate=20
    let craft_id = app
        .add_craft_node("Iron Ingot", &game_data)
        .expect("Failed to add craft node");

    // Set craft rate to 20
    app.set_pin_rate(
        craft_id,
        PinDirection::Output,
        0,
        FractionalNumber::new(20, 1),
    )
    .expect("Failed to set craft rate");

    // Verify craft output is 20
    let (_c_ins, c_outs) = app.get_node_pin_rates(craft_id).unwrap();
    let craft_out_rate = c_outs[0].as_ref().unwrap();
    assert!(
        craft_out_rate == "20" || craft_out_rate == "20/1",
        "Craft output should be 20, got: {}",
        craft_out_rate
    );

    // Create a merger node
    let merger_id = app.add_merger_node();

    // Check merger initial state
    let (m_ins_before, m_outs_before) = app.get_node_pin_rates(merger_id).unwrap();
    println!(
        "Merger before link: ins={:?}, outs={:?}",
        m_ins_before, m_outs_before
    );

    // Connect craft output -> merger input 0
    let craft_out_pin = app
        .get_pin_id(craft_id, PinDirection::Output, 0)
        .expect("Failed to get craft output pin");
    let merger_in_pin = app
        .get_pin_id(merger_id, PinDirection::Input, 0)
        .expect("Failed to get merger input pin");

    let result = app.create_link(craft_out_pin, merger_in_pin);
    if let Err(ref e) = result {
        println!("Link creation error: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "create_link should succeed, got error: {:?}",
        result.err()
    );

    // After connection, the merger input should have rate=20 (from craft)
    let (m_ins_after, m_outs_after) = app.get_node_pin_rates(merger_id).unwrap();
    println!(
        "Merger after link: ins={:?}, outs={:?}",
        m_ins_after, m_outs_after
    );

    // Connected input (index 0) should have rate 20
    let in0_rate = m_ins_after[0]
        .as_ref()
        .expect("Input 0 rate should be Some");
    assert!(
        in0_rate == "20" || in0_rate == "20/1",
        "Merger input 0 should have rate 20 (from craft), got: {}",
        in0_rate
    );

    // Merger output should be 20 (sum of inputs: 20 + 0 = 20)
    let out_rate = m_outs_after[0]
        .as_ref()
        .expect("Output rate should be Some");
    assert!(
        out_rate == "20" || out_rate == "20/1",
        "Merger output should be 20, got: {}",
        out_rate
    );
}

/// Test that when all merger inputs are connected and one changes,
/// other inputs keep their values and the output adjusts.
#[test]
fn test_merger_inputs_independent_output_adjusts() {
    let game_data = load_game_data();
    let mut app = ProductionApp::new();

    // Create two craft nodes
    let craft1_id = app
        .add_craft_node("Iron Ingot", &game_data)
        .expect("Failed to add craft1");
    let craft2_id = app
        .add_craft_node("Iron Ingot", &game_data)
        .expect("Failed to add craft2");

    // Set craft1 rate to 10, craft2 rate to 30
    app.set_pin_rate(
        craft1_id,
        PinDirection::Output,
        0,
        FractionalNumber::new(10, 1),
    )
    .expect("Failed to set craft1 rate");
    app.set_pin_rate(
        craft2_id,
        PinDirection::Output,
        0,
        FractionalNumber::new(30, 1),
    )
    .expect("Failed to set craft2 rate");

    // Create a merger node
    let merger_id = app.add_merger_node();

    // Connect craft1 output -> merger input 0
    let c1_out = app.get_pin_id(craft1_id, PinDirection::Output, 0).unwrap();
    let m_in0 = app.get_pin_id(merger_id, PinDirection::Input, 0).unwrap();
    app.create_link(c1_out, m_in0).expect("First link failed");

    // Connect craft2 output -> merger input 1
    let c2_out = app.get_pin_id(craft2_id, PinDirection::Output, 0).unwrap();
    let m_in1 = app.get_pin_id(merger_id, PinDirection::Input, 1).unwrap();

    println!(
        "Before second link: craft2 out rate = {:?}",
        app.get_node_pin_rates(craft2_id).unwrap().1[0]
    );

    let result2 = app.create_link(c2_out, m_in1);
    if let Err(ref e) = result2 {
        println!("Second link error: {:?}", e);
    }
    if let Ok((_id, ref warn)) = result2 {
        println!("Second link warning: {:?}", warn);
    }
    result2.expect("Second link failed");

    // Check merger state: inputs=[10, 30], output=40
    let (m_ins, m_outs) = app.get_node_pin_rates(merger_id).unwrap();
    println!(
        "Merger after both links: ins={:?}, outs={:?}",
        m_ins, m_outs
    );

    let in0 = m_ins[0].as_ref().unwrap();
    let in1 = m_ins[1].as_ref().unwrap();
    let out = m_outs[0].as_ref().unwrap();
    assert!(
        in0 == "10" || in0 == "10/1",
        "Input 0 should be 10, got: {}",
        in0
    );
    assert!(
        in1 == "30" || in1 == "30/1",
        "Input 1 should be 30, got: {}",
        in1
    );
    assert!(
        out == "40" || out == "40/1",
        "Output should be 40 (10+30), got: {}",
        out
    );

    // Now change craft1's rate to 50
    app.set_pin_rate(
        craft1_id,
        PinDirection::Output,
        0,
        FractionalNumber::new(50, 1),
    )
    .expect("Failed to update craft1 rate");

    // Check merger state: inputs=[50, 30], output=80
    // IMPORTANT: craft2's rate should remain 30, not change
    let (m_ins2, m_outs2) = app.get_node_pin_rates(merger_id).unwrap();
    let (c2_ins, c2_outs) = app.get_node_pin_rates(craft2_id).unwrap();
    println!(
        "After craft1 change: merger ins={:?}, outs={:?}, craft2 outs={:?}",
        m_ins2, m_outs2, c2_outs
    );

    let in0_after = m_ins2[0].as_ref().unwrap();
    let in1_after = m_ins2[1].as_ref().unwrap();
    let out_after = m_outs2[0].as_ref().unwrap();
    let c2_out_rate = c2_outs[0].as_ref().unwrap();

    assert!(
        in0_after == "50" || in0_after == "50/1",
        "Input 0 should be 50, got: {}",
        in0_after
    );
    // Craft2 and merger input 1 should remain 30, NOT reset to 0
    assert!(
        in1_after == "30" || in1_after == "30/1",
        "Input 1 should remain 30 (unchanged), got: {}",
        in1_after
    );
    assert!(
        c2_out_rate == "30" || c2_out_rate == "30/1",
        "Craft2 output should remain 30, got: {}",
        c2_out_rate
    );
    assert!(
        out_after == "80" || out_after == "80/1",
        "Output should be 80 (50+30), got: {}",
        out_after
    );
}

/// Test simulating the scenario of dropping a wire from a merger output
/// to create a new craft node - the craft input should receive the merger output's rate.
#[test]
fn test_connect_merger_output_to_new_craft_input() {
    let game_data = load_game_data();
    let mut app = ProductionApp::new();

    // Create a merger with some inputs set up
    let merger_id = app.add_merger_node();

    // Set merger input 0 to have rate 20 (simulating a connected source)
    let m_in0 = app.get_pin_id(merger_id, PinDirection::Input, 0).unwrap();
    app.set_pin_rate(
        merger_id,
        PinDirection::Input,
        0,
        FractionalNumber::new(20, 1),
    )
    .expect("Failed to set merger input rate");

    // Check merger state: input 0 = 20, output should be 20
    let (m_ins, m_outs) = app.get_node_pin_rates(merger_id).unwrap();
    println!("Merger before link: ins={:?}, outs={:?}", m_ins, m_outs);

    // Verify merger output is 20
    let m_out_rate = m_outs[0].as_ref().unwrap();
    assert!(
        m_out_rate == "20" || m_out_rate == "20/1",
        "Merger output should be 20, got: {}",
        m_out_rate
    );

    // Create a new craft node (simulating what UI does when you drop wire to create node)
    let craft_id = app
        .add_craft_node("Iron Ingot", &game_data)
        .expect("Failed to add craft node");

    // Check craft initial state (should be rate 0)
    let (c_ins, _c_outs) = app.get_node_pin_rates(craft_id).unwrap();
    println!("Craft before link: ins={:?}", c_ins);

    // Connect merger output -> craft input
    // This is what happens when you drop a wire from merger output onto a new craft node
    let m_out = app.get_pin_id(merger_id, PinDirection::Output, 0).unwrap();
    let c_in = app.get_pin_id(craft_id, PinDirection::Input, 0).unwrap();

    let result = app.create_link(m_out, c_in);
    if let Err(ref e) = result {
        println!("Link creation error: {:?}", e);
    }
    if let Ok((_, ref warn)) = result {
        println!("Link creation warning: {:?}", warn);
    }
    assert!(
        result.is_ok(),
        "create_link should succeed, got: {:?}",
        result.err()
    );

    // After the link, the craft input should receive the merger output's rate (20)
    let (c_ins_after, c_outs_after) = app.get_node_pin_rates(craft_id).unwrap();
    let (m_ins_after, m_outs_after) = app.get_node_pin_rates(merger_id).unwrap();
    println!(
        "After link: craft ins={:?}, outs={:?}",
        c_ins_after, c_outs_after
    );
    println!(
        "After link: merger ins={:?}, outs={:?}",
        m_ins_after, m_outs_after
    );

    // Craft input 0 should have rate 20 (from merger output)
    let c_in0_rate = c_ins_after[0]
        .as_ref()
        .expect("Craft input 0 should have a rate");
    assert!(
        c_in0_rate == "20" || c_in0_rate == "20/1",
        "Craft input should receive merger output rate (20), got: {}",
        c_in0_rate
    );
}

/// Test that splitter outputs are independent of each other.
/// When one output is edited, other outputs should stay constant, and input adjusts.
#[test]
fn test_splitter_outputs_independent_input_adjusts() {
    let _game_data = load_game_data();
    let mut app = ProductionApp::new();

    // Create a splitter (now has 3 outputs)
    let splitter_id = app.add_custom_splitter_node();

    // Set splitter input to 60
    app.set_pin_rate(
        splitter_id,
        PinDirection::Input,
        0,
        FractionalNumber::new(60, 1),
    )
    .expect("Failed to set splitter input rate");

    // Check splitter state: input = 60, outputs = [20, 20, 20] (equal distribution for 3 outputs)
    let (s_ins, s_outs) = app.get_node_pin_rates(splitter_id).unwrap();
    println!("Splitter initial: ins={:?}, outs={:?}", s_ins, s_outs);

    // Input should be 60
    let in_rate = s_ins[0].as_ref().unwrap();
    assert!(
        in_rate == "60" || in_rate == "60/1",
        "Input should be 60, got: {}",
        in_rate
    );

    // All outputs should be 20 (60 / 3)
    for (i, out) in s_outs.iter().enumerate() {
        let out_rate = out.as_ref().unwrap();
        assert!(
            out_rate == "20" || out_rate == "20/1",
            "Output {} should be 20, got: {}",
            i,
            out_rate
        );
    }

    // Now set output 1 to 30 - this should adjust the input (output 0 and 2 stay at 20)
    app.set_pin_rate(
        splitter_id,
        PinDirection::Output,
        1,
        FractionalNumber::new(30, 1),
    )
    .expect("Failed to set splitter output 1");

    let (s_ins_after, s_outs_after) = app.get_node_pin_rates(splitter_id).unwrap();
    println!(
        "Splitter after setting out1=30: ins={:?}, outs={:?}",
        s_ins_after, s_outs_after
    );

    // Output 1 should be 30
    let out1_rate = s_outs_after[1].as_ref().unwrap();
    assert!(
        out1_rate == "30" || out1_rate == "30/1",
        "Output 1 should be 30, got: {}",
        out1_rate
    );

    // Output 0 should STILL be 20 (independent)
    let out0_after = s_outs_after[0].as_ref().unwrap();
    assert!(
        out0_after == "20" || out0_after == "20/1",
        "Output 0 should remain 20 (independent), got: {}",
        out0_after
    );

    // Output 2 should STILL be 20 (independent)
    let out2_after = s_outs_after[2].as_ref().unwrap();
    assert!(
        out2_after == "20" || out2_after == "20/1",
        "Output 2 should remain 20 (independent), got: {}",
        out2_after
    );

    // Input should be sum of outputs: 20 + 30 + 20 = 70
    let in_after = s_ins_after[0].as_ref().unwrap();
    assert!(
        in_after == "70" || in_after == "70/1",
        "Input should be 70 (sum of outputs), got: {}",
        in_after
    );
}
