use ficsit_companion_rs::game_data::GameData;

#[expect(clippy::too_many_lines)]
#[test]
fn smoke_test_game_data_parser() {
    let json_data = r#"{
    "version": "1.0",
    "buildings": [
        {
            "name": "Coal-Powered Generator",
            "somersloop_mult": 1.0,
            "power": -75.0,
            "power_exponent": 1.0,
            "somersloop_power_exponent": 2.0,
            "variable_power": false
        },
        {
            "name": "Fuel-Powered Generator",
            "somersloop_mult": 1.0,
            "power": -250.0,
            "power_exponent": 1.0,
            "somersloop_power_exponent": 2.0,
            "variable_power": false
        },
        {
            "name": "Constructor",
            "somersloop_mult": 1.0,
            "power": -4.0,
            "power_exponent": 1.0,
            "somersloop_power_exponent": 1.6,
            "variable_power": false
        },
        {
            "name": "Smelter",
            "somersloop_mult": 1.0,
            "power": -4.0,
            "power_exponent": 1.0,
            "somersloop_power_exponent": 1.6,
            "variable_power": false
        }
    ],
    "items": [
        {
            "name": "Packaged Liquid Biofuel",
            "icon": "icons/IconDesc_LiquidBiofuel_64.png",
            "sink": 370
        },
        {
            "name": "Alien Power Matrix",
            "icon": "icons/IconDesc_AlienPowerMatrix_64.png",
            "sink": 210
        },
        {
            "name": "Iron Ingot",
            "icon": "icons/IconDesc_IronIngot_64.png",
            "sink": 2
        },
        {
            "name": "Iron Plate",
            "icon": "icons/IconDesc_IronPlate_64.png",
            "sink": 6
        },
        {
            "name": "Iron Rod",
            "icon": "icons/IconDesc_IronRod_64.png",
            "sink": 4
        },
        {
            "name": "Iron Ore",
            "icon": "icons/IconDesc_OreIron_64.png",
            "sink": 1
        }
    ],
    "recipes": [
        {
            "name": "Iron Plate",
            "alternate": false,
            "time": 6.0,
            "building": "Constructor",
            "inputs": [
                {
                    "name": "Iron Ingot",
                    "amount": 3.0
                }
            ],
            "outputs": [
                {
                    "name": "Iron Plate",
                    "amount": 2.0
                }
            ]
        },
        {
            "name": "Iron Rod",
            "alternate": false,
            "time": 4.0,
            "building": "Constructor",
            "inputs": [
                {
                    "name": "Iron Ingot",
                    "amount": 1.0
                }
            ],
            "outputs": [
                {
                    "name": "Iron Rod",
                    "amount": 1.0
                }
            ]
        },
        {
            "name": "Iron Ingot",
            "alternate": false,
            "time": 2.0,
            "building": "Smelter",
            "inputs": [
                {
                    "name": "Iron Ore",
                    "amount": 1.0
                }
            ],
            "outputs": [
                {
                    "name": "Iron Ingot",
                    "amount": 1.0
                }
            ]
        }
    ]
}"#;

    let mut game_data = GameData::new();
    let result = game_data.load_from_json(json_data);

    assert!(
        result.is_ok(),
        "Failed to parse game data: {:?}",
        result.err()
    );

    // Verify version
    assert_eq!(game_data.version(), "1.0");

    // Verify items were loaded
    assert_eq!(game_data.items().len(), 6, "Expected 6 items");
    assert!(game_data.items().contains_key("Packaged Liquid Biofuel"));
    assert!(game_data.items().contains_key("Alien Power Matrix"));

    let biofuel = game_data
        .items()
        .get("Packaged Liquid Biofuel")
        .expect("Packaged Liquid Biofuel missing");
    assert_eq!(biofuel.name, "Packaged Liquid Biofuel");
    assert_eq!(biofuel.sink_value, 370);

    // Verify buildings were loaded
    assert_eq!(game_data.buildings().len(), 4, "Expected 4 buildings");
    assert!(game_data.buildings().contains_key("Coal-Powered Generator"));
    assert!(game_data.buildings().contains_key("Fuel-Powered Generator"));
    assert!(game_data.buildings().contains_key("Constructor"));
    assert!(game_data.buildings().contains_key("Smelter"));

    let coal_gen = game_data
        .buildings()
        .get("Coal-Powered Generator")
        .expect("Coal-Powered Generator missing");
    assert_eq!(coal_gen.name, "Coal-Powered Generator");
    assert_eq!(coal_gen.power, -75.0);
    assert!(!coal_gen.variable_power);

    // Verify recipes were loaded
    assert_eq!(game_data.recipes().len(), 3, "Expected 3 recipes");

    let iron_plate_recipe = game_data
        .recipes()
        .iter()
        .find(|r| r.name == "Iron Plate")
        .expect("Iron Plate recipe not found");

    assert_eq!(iron_plate_recipe.name, "Iron Plate");
    assert_eq!(iron_plate_recipe.building_name, "Constructor");
    assert!(!iron_plate_recipe.alternate);
    assert_eq!(iron_plate_recipe.ins.len(), 1);
    assert_eq!(iron_plate_recipe.outs.len(), 1);
    assert_eq!(
        iron_plate_recipe
            .ins
            .first()
            .and_then(|i| i.item.as_ref())
            .expect("Iron Plate recipe input missing")
            .name,
        "Iron Ingot"
    );
    assert_eq!(
        iron_plate_recipe
            .outs
            .first()
            .and_then(|i| i.item.as_ref())
            .expect("Iron Plate recipe output missing")
            .name,
        "Iron Plate"
    );

    // Quantities should be normalized to items per minute (60 / time)
    // Iron Plate: inputs 3 per 6s -> 30 per min; outputs 2 per 6s -> 20 per min
    assert_eq!(
        iron_plate_recipe
            .ins
            .first()
            .expect("Iron Plate recipe input missing")
            .quantity
            .numerator(),
        30
    );
    assert_eq!(
        iron_plate_recipe
            .ins
            .first()
            .expect("Iron Plate recipe input missing")
            .quantity
            .denominator(),
        1
    );
    assert_eq!(
        iron_plate_recipe
            .outs
            .first()
            .expect("Iron Plate recipe output missing")
            .quantity
            .numerator(),
        20
    );
    assert_eq!(
        iron_plate_recipe
            .outs
            .first()
            .expect("Iron Plate recipe output missing")
            .quantity
            .denominator(),
        1
    );

    // Iron Rod: 1 per 4s -> 15 per min
    let iron_rod_recipe = game_data
        .recipes()
        .iter()
        .find(|r| r.name == "Iron Rod")
        .expect("Iron Rod recipe not found");
    assert_eq!(
        iron_rod_recipe
            .ins
            .first()
            .expect("Iron Rod recipe input missing")
            .quantity
            .numerator(),
        15
    );
    assert_eq!(
        iron_rod_recipe
            .outs
            .first()
            .expect("Iron Rod recipe output missing")
            .quantity
            .numerator(),
        15
    );

    // Iron Ingot (Smelter): 1 per 2s -> 30 per min
    let iron_ingot_recipe = game_data
        .recipes()
        .iter()
        .find(|r| r.name == "Iron Ingot")
        .expect("Iron Ingot recipe not found");
    assert_eq!(
        iron_ingot_recipe
            .ins
            .first()
            .expect("Iron Ingot recipe input missing")
            .quantity
            .numerator(),
        30
    );
    assert_eq!(
        iron_ingot_recipe
            .outs
            .first()
            .expect("Iron Ingot recipe output missing")
            .quantity
            .numerator(),
        30
    );
}
