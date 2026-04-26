use super::*;

#[test]
fn player_state_round_trips_inventory_counts() {
    let mut inventory = Inventory::default();
    inventory.add(ITEM_STONE, 42);
    inventory.add(ITEM_CHEST, 2);

    let decoded = decode_player_state_v1(&encode_player_state_v1(&inventory)).unwrap();

    assert_eq!(decoded.count(ITEM_STONE), 42);
    assert_eq!(decoded.count(ITEM_CHEST), 2);
}

#[test]
fn player_state_rejects_trailing_bytes() {
    let inventory = Inventory::default();
    let mut blob = encode_player_state_v1(&inventory);
    blob.push(0);

    assert_eq!(
        decode_player_state_v1(&blob).unwrap_err(),
        "player_state trailing bytes"
    );
}

#[test]
fn player_state_rejects_bad_magic() {
    let inventory = Inventory::default();
    let mut blob = encode_player_state_v1(&inventory);
    blob[0] = b'X';

    assert_eq!(
        decode_player_state_v1(&blob).unwrap_err(),
        "player_state magic mismatch"
    );
}
