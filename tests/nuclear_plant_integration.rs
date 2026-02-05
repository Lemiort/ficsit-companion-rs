use ficsit_companion_rs::game_data::GameData;
use ficsit_companion_rs::production_app::ProductionApp;
use serde_json::Value;

fn load_game_data() -> GameData {
    let json = include_str!("../assets/satisfactory.json");
    let mut game_data = GameData::new();
    game_data
        .load_from_json(json)
        .expect("Failed to load game data");
    game_data
}

#[expect(clippy::too_many_lines)]
#[test]
fn test_nuclear_plant_load_and_file_consistency() {
    let json = include_str!("../tests/nuclear_plant.fcs");
    let game_data = load_game_data();

    let mut app = ProductionApp::new();
    app.load_from_json(json, Some(&game_data))
        .expect("Failed to load nuclear_plant.fcs");

    // Parse raw JSON to drive assertions so the test stays in sync with the save file
    let parsed: Value = serde_json::from_str(json).expect("Failed to parse nuclear_plant.fcs JSON");
    let file_nodes = parsed
        .get("nodes")
        .and_then(|n| n.as_array())
        .expect("nodes array missing in file");

    // Ensure node count matches file
    assert_eq!(
        app.node_count(),
        file_nodes.len(),
        "Node count should match file"
    );

    // For each file node, validate craft rates and organizer ins/outs where present
    for (idx, fnode) in file_nodes.iter().enumerate() {
        // If it's a craft node (has "recipe"), compare the saved rate to the loaded node rate
        if fnode.get("recipe").is_some() {
            let nid = app
                .find_node_by_index(idx)
                .unwrap_or_else(|| panic!("Craft node {idx} missing"));
            let (rate_str, _building) = app
                .get_node_building_info(nid)
                .unwrap_or_else(|| panic!("Failed to get building info for craft node {idx}"));
            // Extract rate from file
            let num = fnode
                .get("rate")
                .and_then(|r| r.get("num"))
                .and_then(|n| n.as_i64())
                .expect("Missing rate.num");
            let den = fnode
                .get("rate")
                .and_then(|r| r.get("den"))
                .and_then(|d| d.as_i64())
                .expect("Missing rate.den");
            let expected = if den == 1 {
                num.to_string()
            } else {
                format!("{num}/{den}")
            };
            assert_eq!(rate_str, expected, "Craft node {idx} rate mismatch");

            // If file marked the node locked, ensure pins are locked accordingly
            if let Some(locked_val) = fnode.get("locked")
                && locked_val.as_bool().unwrap_or(false)
            {
                let (ins_locked, outs_locked) = app
                    .get_node_pin_locked_flags(nid)
                    .expect("Failed to get pin locked flags");
                assert!(
                    ins_locked.iter().chain(outs_locked.iter()).all(|b| *b),
                    "All pins of craft node {idx} should be locked",
                );
            }
        }

        // Organizer nodes: check ins/outs if present
        if fnode.get("ins").is_some() || fnode.get("outs").is_some() {
            let nid = app
                .find_node_by_index(idx)
                .unwrap_or_else(|| panic!("Organizer node {idx} missing"));
            let (ins, outs) = app
                .get_node_pin_rates(nid)
                .expect("Failed to get organizer pin rates");

            if let Some(ins_file) = fnode.get("ins") {
                let ins_array = ins_file.as_array().expect("ins not an array");
                assert_eq!(
                    ins_array.len(),
                    ins.len(),
                    "Organizer {idx} input count mismatch",
                );
                for (i, pin) in ins_array.iter().enumerate() {
                    let num = pin["num"].as_i64().expect("ins.num missing");
                    let den = pin["den"].as_i64().expect("ins.den missing");
                    let expected = if den == 1 {
                        num.to_string()
                    } else {
                        format!("{num}/{den}")
                    };
                    assert_eq!(
                        ins.get(i)
                            .and_then(|o| o.as_ref())
                            .expect("Organizer input missing"),
                        &expected,
                        "Organizer {idx} input {i} rate mismatch",
                    );
                    if let Some(lock_val) = pin.get("locked") {
                        let locked_flags = app
                            .get_node_pin_locked_flags(nid)
                            .expect("Failed to get pin locked flags");
                        assert_eq!(
                            locked_flags.0.get(i).copied().unwrap_or(false),
                            lock_val.as_bool().unwrap_or(false),
                            "Organizer {idx} input {i} locked mismatch",
                        );
                    }
                }
            }

            if let Some(outs_file) = fnode.get("outs") {
                let outs_array = outs_file.as_array().expect("outs not an array");
                assert_eq!(
                    outs_array.len(),
                    outs.len(),
                    "Organizer {idx} output count mismatch",
                );
                for (i, pin) in outs_array.iter().enumerate() {
                    let num = pin["num"].as_i64().expect("outs.num missing");
                    let den = pin["den"].as_i64().expect("outs.den missing");
                    let expected = if den == 1 {
                        num.to_string()
                    } else {
                        format!("{num}/{den}")
                    };
                    assert_eq!(
                        outs.get(i)
                            .and_then(|o| o.as_ref())
                            .expect("Organizer output missing"),
                        &expected,
                        "Organizer {idx} output {i} rate mismatch",
                    );
                    if let Some(lock_val) = pin.get("locked") {
                        let locked_flags = app
                            .get_node_pin_locked_flags(nid)
                            .expect("Failed to get pin locked flags");
                        assert_eq!(
                            locked_flags.1.get(i).copied().unwrap_or(false),
                            lock_val.as_bool().unwrap_or(false),
                            "Organizer {idx} output {i} locked mismatch",
                        );
                    }
                }
            }
        }
    }

    // Ensure single merger node in file uses expected item type (Sulfuric Acid)
    let merger_indices: Vec<usize> = file_nodes
        .iter()
        .enumerate()
        .filter_map(|(i, n)| {
            if n.get("kind").and_then(|k| k.as_i64()) == Some(2) {
                Some(i)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        merger_indices.len(),
        1,
        "Expected exactly one merger in the file"
    );
    let merger_idx = *merger_indices
        .first()
        .expect("Expected exactly one merger index");

    // Check file's declared item
    let merger_file_item = file_nodes
        .get(merger_idx)
        .and_then(|n| n.get("item"))
        .and_then(|v| v.as_str())
        .expect("Merger item missing in file");
    assert_eq!(
        merger_file_item, "Sulfuric Acid",
        "Merger file item mismatch"
    );

    // Check loaded app's organizer node item
    let merger_nid = app
        .find_node_by_index(merger_idx)
        .expect("Merger node missing in app");
    let merger_item = app
        .get_node_item_name(merger_nid)
        .expect("Merger node item missing in app");
    assert_eq!(merger_item, "Sulfuric Acid", "Merger node item mismatch");

    // Power-generator sanity checks are in a separate test below
    // to keep the function size reasonable.
}

#[test]
fn test_nuclear_plant_power_generators() {
    let json = include_str!("../tests/nuclear_plant.fcs");
    let game_data = load_game_data();

    let mut app = ProductionApp::new();
    app.load_from_json(json, Some(&game_data))
        .expect("Failed to load nuclear_plant.fcs");

    // Parse raw JSON to drive assertions
    let parsed: Value = serde_json::from_str(json).expect("Failed to parse nuclear_plant.fcs JSON");
    let file_nodes = parsed
        .get("nodes")
        .and_then(|n| n.as_array())
        .expect("nodes array missing in file");

    // Sanity check: ensure power-generating craft nodes have negative power and non-zero totals
    let mut found_power_gen = false;
    for (idx, fnode) in file_nodes.iter().enumerate() {
        if let Some(recipe_name) = fnode.get("recipe").and_then(|r| r.as_str())
            && recipe_name.starts_with("Power (")
        {
            let nid = app
                .find_node_by_index(idx)
                .unwrap_or_else(|| panic!("Power craft node {idx} missing"));
            // Node should be marked as power generator
            assert!(
                app.get_node_is_power_generator(nid),
                "Node {idx} should be a power generator",
            );
            if let Some((same_str, last_str, _variable)) = app.get_node_power_info(nid) {
                // Parse as FractionalNumber and ensure non-zero (and likely negative)
                let same = match ficsit_companion_rs::FractionalNumber::from_string(&same_str) {
                    Ok(v) => v,
                    Err(e) => panic!("Failed parsing same power '{same_str}': {e}"),
                };
                let last = match ficsit_companion_rs::FractionalNumber::from_string(&last_str) {
                    Ok(v) => v,
                    Err(e) => panic!("Failed parsing last power '{last_str}': {e}"),
                };
                assert!(
                    same.value() != 0.0 || last.value() != 0.0,
                    "Power generator node {idx} reports zero power",
                );
                // At least one should be negative (generation)
                assert!(
                    same.value() < 0.0 || last.value() < 0.0,
                    "Power generator node {idx} should have negative power",
                );
                found_power_gen = true;
            } else {
                panic!("No power info for node {idx}");
            }
        }
    }
    assert!(found_power_gen, "No power generator found in loaded file");
}
