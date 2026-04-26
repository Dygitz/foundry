use super::*;

#[test]
fn crafting_consumes_inputs_and_adds_output() {
    let mut inv = Inventory::default();
    inv.add(ITEM_STONE, 10);

    assert!(try_craft(&mut inv, &RECIPE_FURNACE));
    assert_eq!(inv.count(ITEM_STONE), 0);
    assert_eq!(inv.count(ITEM_FURNACE), 1);
}

#[test]
fn crafting_fails_without_required_inputs() {
    let mut inv = Inventory::default();
    inv.add(ITEM_STONE, 9);

    assert!(!try_craft(&mut inv, &RECIPE_FURNACE));
    assert_eq!(inv.count(ITEM_STONE), 9);
    assert_eq!(inv.count(ITEM_FURNACE), 0);
}

#[test]
fn placement_and_smelt_rules_match_item_catalog() {
    assert_eq!(item_to_placed_kind(ITEM_FURNACE), Some(PLACED_FURNACE));
    assert_eq!(item_to_placed_kind(ITEM_CHEST), Some(PLACED_CHEST));
    assert_eq!(item_to_placed_kind(ITEM_STONE), None);
    assert_eq!(smelt_output_for_input(ITEM_IRON_ORE), Some(ITEM_IRON_PLATE));
    assert_eq!(
        smelt_output_for_input(ITEM_COPPER_ORE),
        Some(ITEM_COPPER_PLATE)
    );
    assert_eq!(smelt_output_for_input(ITEM_COAL), None);
}

#[test]
fn resource_pickups_map_to_inventory_items() {
    assert_eq!(resource_to_item(RES_IRON), Some(ITEM_IRON_ORE));
    assert_eq!(resource_to_item(RES_COPPER), Some(ITEM_COPPER_ORE));
    assert_eq!(resource_to_item(RES_COAL), Some(ITEM_COAL));
    assert_eq!(resource_to_item(RES_STONE), Some(ITEM_STONE));
}

#[test]
fn object_ids_are_deterministic_and_non_zero() {
    let a = object_id_for_tile(1337, -8, 12, PLACED_CHEST);
    let b = object_id_for_tile(1337, -8, 12, PLACED_CHEST);

    assert_eq!(a, b);
    assert_ne!(a, 0);
}
