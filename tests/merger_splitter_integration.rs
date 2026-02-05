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
fn test_merger_and_splitter_rates_from_cpp_export() {
    let json = include_str!("../tests/merger_splitter_test.fcs");
    let game_data = load_game_data();

    let mut app = ProductionApp::new();
    app.load_from_json(json, Some(&game_data))
        .expect("Failed to load merger_splitter_test.fcs");

    // Merger at node index 9: inputs 10 and 30 -> output 40
    let merger_node_id = app.find_node_by_index(9).expect("Merger node missing");
    let (ins, outs) = app
        .get_node_pin_rates(merger_node_id)
        .expect("Failed to get merger pin rates");
    assert_eq!(ins.len(), 2, "Merger should have 2 inputs");
    assert_eq!(outs.len(), 1, "Merger should have 1 output");
    assert_eq!(
        ins.first()
            .and_then(|o| o.as_ref())
            .expect("Merger input 0 missing"),
        "10",
        "Merger input 0 should be 10"
    );
    assert_eq!(
        ins.get(1)
            .and_then(|o| o.as_ref())
            .expect("Merger input 1 missing"),
        "30",
        "Merger input 1 should be 30"
    );
    assert_eq!(
        outs.first()
            .and_then(|o| o.as_ref())
            .expect("Merger output missing"),
        "40",
        "Merger output should be 40"
    );

    // Custom Splitter at node index 10: input 51 -> outputs 15,36,0
    let splitter_node_id = app.find_node_by_index(10).expect("Splitter node missing");
    let (s_ins, s_outs) = app
        .get_node_pin_rates(splitter_node_id)
        .expect("Failed to get splitter pin rates");
    assert_eq!(s_ins.len(), 1, "Splitter should have 1 input");
    assert_eq!(s_outs.len(), 3, "Splitter should have 3 outputs");
    assert_eq!(
        s_ins
            .first()
            .and_then(|o| o.as_ref())
            .expect("Splitter input missing"),
        "51",
        "Splitter input should be 51"
    );
    assert_eq!(
        s_outs
            .first()
            .and_then(|o| o.as_ref())
            .expect("Splitter out0 missing"),
        "15",
        "Splitter output 0 should be 15"
    );
    assert_eq!(
        s_outs
            .get(1)
            .and_then(|o| o.as_ref())
            .expect("Splitter out1 missing"),
        "36",
        "Splitter output 1 should be 36"
    );
    assert_eq!(
        s_outs
            .get(2)
            .and_then(|o| o.as_ref())
            .expect("Splitter out2 missing"),
        "0",
        "Splitter output 2 should be 0"
    );

    // Concrete craft node at index 11 should have node rate 17/5
    let concrete_node_id = app.find_node_by_index(11).expect("Concrete node missing");
    let (_c_ins, _c_outs) = app
        .get_node_pin_rates(concrete_node_id)
        .expect("Failed to get concrete pin rates");
    let (rate_str, _building) = app
        .get_node_building_info(concrete_node_id)
        .expect("Failed to get building info for Concrete node");
    assert_eq!(rate_str, "17/5", "Concrete node rate should be 17/5");
}
