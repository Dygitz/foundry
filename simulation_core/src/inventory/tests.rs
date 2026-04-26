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
