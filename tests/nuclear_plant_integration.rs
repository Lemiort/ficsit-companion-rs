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
                .expect(&format!("Craft node {} missing", idx));
            let (rate_str, _building) = app.get_node_building_info(nid).expect(&format!(
                "Failed to get building info for craft node {}",
                idx
            ));
            // Extract rate from file
            let num = fnode["rate"]["num"].as_i64().expect("Missing rate.num");
            let den = fnode["rate"]["den"].as_i64().expect("Missing rate.den");
            let expected = if den == 1 {
                num.to_string()
            } else {
                format!("{}/{}", num, den)
            };
            assert_eq!(rate_str, expected, "Craft node {} rate mismatch", idx);

            // If file marked the node locked, ensure pins are locked accordingly
            if let Some(locked_val) = fnode.get("locked") {
                if locked_val.as_bool().unwrap_or(false) {
                    let (ins_locked, outs_locked) = app
                        .get_node_pin_locked_flags(nid)
                        .expect("Failed to get pin locked flags");
                    assert!(
                        ins_locked.iter().chain(outs_locked.iter()).all(|b| *b),
                        "All pins of craft node {} should be locked",
                        idx
                    );
                }
            }
        }

        // Organizer nodes: check ins/outs if present
        if fnode.get("ins").is_some() || fnode.get("outs").is_some() {
            let nid = app
                .find_node_by_index(idx)
                .expect(&format!("Organizer node {} missing", idx));
            let (ins, outs) = app
                .get_node_pin_rates(nid)
                .expect("Failed to get organizer pin rates");

            if let Some(ins_file) = fnode.get("ins") {
                let ins_array = ins_file.as_array().expect("ins not an array");
                assert_eq!(
                    ins_array.len(),
                    ins.len(),
                    "Organizer {} input count mismatch",
                    idx
                );
                for (i, pin) in ins_array.iter().enumerate() {
                    let num = pin["num"].as_i64().expect("ins.num missing");
                    let den = pin["den"].as_i64().expect("ins.den missing");
                    let expected = if den == 1 {
                        num.to_string()
                    } else {
                        format!("{}/{}", num, den)
                    };
                    assert_eq!(
                        ins[i].as_ref().unwrap(),
                        &expected,
                        "Organizer {} input {} rate mismatch",
                        idx,
                        i
                    );
                    if let Some(lock_val) = pin.get("locked") {
                        let locked_flags = app
                            .get_node_pin_locked_flags(nid)
                            .expect("Failed to get pin locked flags");
                        assert_eq!(
                            locked_flags.0[i],
                            lock_val.as_bool().unwrap_or(false),
                            "Organizer {} input {} locked mismatch",
                            idx,
                            i
                        );
                    }
                }
            }

            if let Some(outs_file) = fnode.get("outs") {
                let outs_array = outs_file.as_array().expect("outs not an array");
                assert_eq!(
                    outs_array.len(),
                    outs.len(),
                    "Organizer {} output count mismatch",
                    idx
                );
                for (i, pin) in outs_array.iter().enumerate() {
                    let num = pin["num"].as_i64().expect("outs.num missing");
                    let den = pin["den"].as_i64().expect("outs.den missing");
                    let expected = if den == 1 {
                        num.to_string()
                    } else {
                        format!("{}/{}", num, den)
                    };
                    assert_eq!(
                        outs[i].as_ref().unwrap(),
                        &expected,
                        "Organizer {} output {} rate mismatch",
                        idx,
                        i
                    );
                    if let Some(lock_val) = pin.get("locked") {
                        let locked_flags = app
                            .get_node_pin_locked_flags(nid)
                            .expect("Failed to get pin locked flags");
                        assert_eq!(
                            locked_flags.1[i],
                            lock_val.as_bool().unwrap_or(false),
                            "Organizer {} output {} locked mismatch",
                            idx,
                            i
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
    let merger_idx = merger_indices[0];

    // Check file's declared item
    let merger_file_item = file_nodes[merger_idx]
        .get("item")
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

    // Sanity check: ensure power-generating craft nodes have negative power and non-zero totals
    let mut found_power_gen = false;
    for (idx, fnode) in file_nodes.iter().enumerate() {
        if let Some(recipe_name) = fnode.get("recipe").and_then(|r| r.as_str()) {
            if recipe_name.starts_with("Power (") {
                let nid = app
                    .find_node_by_index(idx)
                    .expect(&format!("Power craft node {} missing", idx));
                // Node should be marked as power generator
                assert!(app.get_node_is_power_generator(nid), "Node {} should be a power generator", idx);
                if let Some((same_str, last_str, _variable)) = app.get_node_power_info(nid) {
                    // Parse as FractionalNumber and ensure non-zero (and likely negative)
                    let same = match ficsit_companion_rs::FractionalNumber::from_string(&same_str) {
                        Ok(v) => v,
                        Err(e) => panic!("Failed parsing same power '{}': {}", same_str, e),
                    };
                    let last = match ficsit_companion_rs::FractionalNumber::from_string(&last_str) {
                        Ok(v) => v,
                        Err(e) => panic!("Failed parsing last power '{}': {}", last_str, e),
                    };
                    assert!(same.value() != 0.0 || last.value() != 0.0, "Power generator node {} reports zero power", idx);
                    // At least one should be negative (generation)
                    assert!(same.value() < 0.0 || last.value() < 0.0, "Power generator node {} should have negative power", idx);
                    found_power_gen = true;
                } else {
                    panic!("No power info for node {}", idx);
                }
            }
        }
    }
    assert!(found_power_gen, "No power generator found in loaded file");
}
