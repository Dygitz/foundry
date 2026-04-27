use super::*;

#[test]
fn inventory_ignores_empty_item_and_zero_amount() {
    let mut inv = Inventory::default();

    inv.add(ITEM_NONE, 10);
    inv.add(ITEM_STONE, 0);

    assert_eq!(inv.count(ITEM_NONE), 0);
    assert_eq!(inv.count(ITEM_STONE), 0);
}

#[test]
fn inventory_remove_is_all_or_nothing() {
    let mut inv = Inventory::default();
    inv.add(ITEM_STONE, 2);

    assert!(!inv.try_remove(ITEM_STONE, 3));
    assert_eq!(inv.count(ITEM_STONE), 2);
    assert!(inv.try_remove(ITEM_STONE, 2));
    assert_eq!(inv.count(ITEM_STONE), 0);
}

#[test]
fn inventory_splits_counts_into_capped_stacks() {
    let mut inv = Inventory::default();

    assert_eq!(inv.add(ITEM_STONE, 250), 250);
    let stacks = inv.stacks();

    assert_eq!(stacks.len(), 3);
    assert_eq!(stacks[0].count, INVENTORY_MAX_STACK);
    assert_eq!(stacks[1].count, INVENTORY_MAX_STACK);
    assert_eq!(stacks[2].count, 50);
    assert_eq!(inv.used_slots(), 3);
}

#[test]
fn inventory_caps_total_stack_slots() {
    let mut inv = Inventory::default();

    assert_eq!(
        inv.add(
            ITEM_STONE,
            INVENTORY_MAX_STACK * INVENTORY_SLOT_COUNT as u32
        ),
        INVENTORY_MAX_STACK * INVENTORY_SLOT_COUNT as u32
    );
    assert_eq!(inv.add(ITEM_COAL, 1), 0);
    assert_eq!(inv.count(ITEM_COAL), 0);
    assert_eq!(inv.stacks().len(), INVENTORY_SLOT_COUNT);
}

#[test]
fn inventory_fills_partial_stack_before_requiring_free_slot() {
    let mut inv = Inventory::default();

    assert_eq!(inv.add(ITEM_STONE, 99), 99);
    assert_eq!(inv.add(ITEM_STONE, 2), 2);

    let stacks = inv.stacks();
    assert_eq!(stacks.len(), 2);
    assert_eq!(stacks[0].count, INVENTORY_MAX_STACK);
    assert_eq!(stacks[1].count, 1);
}
