use ficsit_companion_rs::game_data::GameData;
use ficsit_companion_rs::node::GroupNode;
use ficsit_companion_rs::production_app::ProductionApp;

/// Test loading a save file with group nodes
#[test]
fn test_load_groups_test_file() {
    // Load game data
    let mut game_data = GameData::new();
    let gd_json = std::fs::read_to_string("assets/satisfactory.json")
        .expect("Failed to read game data");
    game_data.load_from_json(&gd_json).expect("Failed to parse game data");

    // Load the groups test file
    let mut app = ProductionApp::new();
    let content = std::fs::read_to_string("saves/groups_test.fcs")
        .expect("Failed to read groups_test.fcs");
    
    let result = app.load_from_json(&content, Some(&game_data));
    assert!(result.is_ok(), "Failed to load groups_test.fcs: {:?}", result.err());

    // Verify we have nodes loaded
    assert!(!app.nodes.is_empty(), "No nodes were loaded");

    // Count group nodes
    let group_count = app.nodes.iter()
        .filter(|n| n.downcast_ref::<GroupNode>().is_some())
        .count();
    
    // The file should have at least one group (it has nested groups too)
    assert!(group_count >= 1, "Expected at least 1 group node, found {}", group_count);

    println!("Successfully loaded {} nodes including {} group nodes", app.nodes.len(), group_count);

    // Verify group nodes have proper structure
    for node in &app.nodes {
        if let Some(group) = node.downcast_ref::<GroupNode>() {
            println!("Group '{}' contains {} grouped nodes and {} grouped links", 
                group.name, 
                group.grouped_nodes.len(),
                group.grouped_links.len()
            );
            
            // A group should have at least one grouped node
            assert!(!group.grouped_nodes.is_empty(), "Group '{}' has no grouped nodes", group.name);
        }
    }
}

/// Test grouping and ungrouping nodes
#[test]
fn test_group_and_ungroup_nodes() {
    // Load game data
    let mut game_data = GameData::new();
    let gd_json = std::fs::read_to_string("assets/satisfactory.json")
        .expect("Failed to read game data");
    game_data.load_from_json(&gd_json).expect("Failed to parse game data");

    // Create a simple production chain
    let mut app = ProductionApp::new();
    
    // Create a craft node for Iron Ingot
    let result = app.add_craft_node("Iron Ingot", &game_data);
    assert!(result.is_ok());
    let iron_ingot_id = result.unwrap();

    // Create another craft node for Iron Plate
    let result = app.add_craft_node("Iron Plate", &game_data);
    assert!(result.is_ok());
    let iron_plate_id = result.unwrap();

    // We should have 2 nodes
    assert_eq!(app.nodes.len(), 2);

    // Group the two nodes
    let node_ids = vec![iron_ingot_id, iron_plate_id];
    let result = app.group_nodes(&node_ids);
    assert!(result.is_ok(), "Failed to group nodes: {:?}", result.err());
    let group_id = result.unwrap();

    // Now we should have 1 node (the group)
    assert_eq!(app.nodes.len(), 1, "Expected 1 node after grouping");

    // Verify it's a group
    assert!(app.is_group_node(group_id), "Node should be a group");

    // Ungroup
    let result = app.ungroup_node(group_id, Some(&game_data));
    assert!(result.is_ok(), "Failed to ungroup: {:?}", result.err());
    let restored_ids = result.unwrap();

    // Should have restored 2 nodes
    assert_eq!(restored_ids.len(), 2, "Expected 2 restored nodes");
    
    // We should have 2 nodes again
    assert_eq!(app.nodes.len(), 2, "Expected 2 nodes after ungrouping");

    println!("Group/ungroup test passed");
}
