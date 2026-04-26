use super::*;
use crate::{ITEM_COAL, ITEM_IRON_ORE, ITEM_STONE};

#[test]
fn chest_deposit_uses_first_available_stack() {
    let mut chests = [ChestRecord {
        object_id: 7,
        inv: ContainerInv::default(),
    }];

    assert_eq!(deposit_to_chest(&mut chests, 7, ITEM_STONE, 12), 12);
    assert_eq!(deposit_to_chest(&mut chests, 7, ITEM_STONE, 3), 3);
    assert_eq!(chests[0].inv.slots[0].item, ITEM_STONE);
    assert_eq!(chests[0].inv.slots[0].count, 15);
}

#[test]
fn taking_last_item_clears_slot() {
    let mut chests = [ChestRecord {
        object_id: 7,
        inv: ContainerInv::default(),
    }];
    deposit_to_chest(&mut chests, 7, ITEM_STONE, 4);

    let taken = take_from_chest(&mut chests, 7, 0, 4).unwrap();

    assert_eq!(taken.item, ITEM_STONE);
    assert_eq!(taken.count, 4);
    assert!(chests[0].inv.slots[0].is_empty());
}

#[test]
fn furnace_input_and_fuel_are_separate_slots() {
    let mut furnaces = [FurnaceRecord {
        object_id: 9,
        state: FurnaceState::default(),
    }];

    assert_eq!(
        deposit_to_furnace_input(&mut furnaces, 9, ITEM_IRON_ORE, 2),
        2
    );
    assert_eq!(deposit_to_furnace_fuel(&mut furnaces, 9, ITEM_COAL, 1), 1);

    assert_eq!(furnaces[0].state.input.item, ITEM_IRON_ORE);
    assert_eq!(furnaces[0].state.input.count, 2);
    assert_eq!(furnaces[0].state.fuel.item, ITEM_COAL);
    assert_eq!(furnaces[0].state.fuel.count, 1);
}
