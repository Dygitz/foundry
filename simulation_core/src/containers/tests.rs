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

#[test]
fn inserter_deposit_uses_small_internal_buffer() {
    let mut inserters = [InserterRecord {
        object_id: 11,
        direction: InserterDirection::default(),
        inv: InserterInv::default(),
    }];

    assert_eq!(deposit_to_inserter(&mut inserters, 11, ITEM_STONE, 1), 1);
    assert_eq!(deposit_to_inserter(&mut inserters, 11, ITEM_COAL, 1), 1);

    assert_eq!(inserters[0].inv.slots[0].item, ITEM_STONE);
    assert_eq!(inserters[0].inv.slots[1].item, ITEM_COAL);
}

#[test]
fn taking_from_inserter_clears_slot() {
    let mut inserters = [InserterRecord {
        object_id: 11,
        direction: InserterDirection::default(),
        inv: InserterInv::default(),
    }];
    deposit_to_inserter(&mut inserters, 11, ITEM_STONE, 1);

    let taken = take_from_inserter(&mut inserters, 11, 0, 1).unwrap();

    assert_eq!(taken.item, ITEM_STONE);
    assert_eq!(taken.count, 1);
    assert!(inserters[0].inv.slots[0].is_empty());
}

#[test]
fn drill_fuel_and_output_are_separate_slots() {
    let mut drills = [DrillRecord {
        object_id: 13,
        state: DrillState::default(),
    }];

    assert_eq!(deposit_to_drill_fuel(&mut drills, 13, ITEM_COAL, 2), 2);
    drills[0].state.output = Slot {
        item: ITEM_STONE,
        count: 3,
    };

    let fuel = take_from_drill(&mut drills, 13, DrillSlot::Fuel, 1).unwrap();
    let output = take_from_drill(&mut drills, 13, DrillSlot::Output, u32::MAX).unwrap();

    assert_eq!(fuel.item, ITEM_COAL);
    assert_eq!(fuel.count, 1);
    assert_eq!(output.item, ITEM_STONE);
    assert_eq!(output.count, 3);
    assert_eq!(drills[0].state.fuel.count, 1);
    assert!(drills[0].state.output.is_empty());
}
