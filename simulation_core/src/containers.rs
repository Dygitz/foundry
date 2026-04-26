use crate::{ITEM_NONE, ItemId, ObjectId};

pub const CHEST_SLOT_COUNT: usize = 16;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Slot {
    pub item: ItemId,
    pub count: u32,
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            item: ITEM_NONE,
            count: 0,
        }
    }
}

impl Slot {
    pub fn is_empty(&self) -> bool {
        self.item == ITEM_NONE || self.count == 0
    }

    pub fn clear(&mut self) {
        self.item = ITEM_NONE;
        self.count = 0;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ContainerInv {
    pub slots: [Slot; CHEST_SLOT_COUNT],
}

impl Default for ContainerInv {
    fn default() -> Self {
        Self {
            slots: [Slot::default(); CHEST_SLOT_COUNT],
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FurnaceState {
    pub input: Slot,
    pub fuel: Slot,
    pub output: Slot,
    pub progress: u16,
}

impl Default for FurnaceState {
    fn default() -> Self {
        Self {
            input: Slot::default(),
            fuel: Slot::default(),
            output: Slot::default(),
            progress: 0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ChestRecord {
    pub object_id: ObjectId,
    pub inv: ContainerInv,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FurnaceRecord {
    pub object_id: ObjectId,
    pub state: FurnaceState,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FurnaceSlot {
    Input,
    Fuel,
    Output,
}

fn take_from_slot(slot: &mut Slot, amount: u32) -> Option<Slot> {
    if amount == 0 || slot.is_empty() {
        return None;
    }
    let taken = amount.min(slot.count);
    if taken == 0 {
        return None;
    }
    slot.count -= taken;
    let item = slot.item;
    if slot.count == 0 {
        slot.clear();
    }
    Some(Slot { item, count: taken })
}

fn deposit_to_slot(slot: &mut Slot, item: ItemId, amount: u32) -> u32 {
    if item == ITEM_NONE || amount == 0 {
        return 0;
    }
    if slot.is_empty() {
        slot.item = item;
        slot.count = amount;
        return amount;
    }
    if slot.item == item {
        slot.count = slot.count.saturating_add(amount);
        return amount;
    }
    0
}

pub fn take_from_chest(
    chests: &mut [ChestRecord],
    object_id: ObjectId,
    slot_idx: usize,
    amount: u32,
) -> Option<Slot> {
    let chest = chests
        .iter_mut()
        .find(|chest| chest.object_id == object_id)?;
    let slot = chest.inv.slots.get_mut(slot_idx)?;
    take_from_slot(slot, amount)
}

pub fn deposit_to_chest(
    chests: &mut [ChestRecord],
    object_id: ObjectId,
    item: ItemId,
    amount: u32,
) -> u32 {
    let chest = chests.iter_mut().find(|chest| chest.object_id == object_id);
    let Some(chest) = chest else {
        return 0;
    };
    for slot in &mut chest.inv.slots {
        let moved = deposit_to_slot(slot, item, amount);
        if moved > 0 {
            return moved;
        }
    }
    0
}

pub fn take_from_furnace(
    furnaces: &mut [FurnaceRecord],
    object_id: ObjectId,
    slot_kind: FurnaceSlot,
    amount: u32,
) -> Option<Slot> {
    let furnace = furnaces
        .iter_mut()
        .find(|furnace| furnace.object_id == object_id)?;
    let slot = match slot_kind {
        FurnaceSlot::Input => &mut furnace.state.input,
        FurnaceSlot::Fuel => &mut furnace.state.fuel,
        FurnaceSlot::Output => &mut furnace.state.output,
    };
    take_from_slot(slot, amount)
}

pub fn deposit_to_furnace_input(
    furnaces: &mut [FurnaceRecord],
    object_id: ObjectId,
    item: ItemId,
    amount: u32,
) -> u32 {
    let furnace = furnaces
        .iter_mut()
        .find(|furnace| furnace.object_id == object_id);
    let Some(furnace) = furnace else {
        return 0;
    };
    deposit_to_slot(&mut furnace.state.input, item, amount)
}

pub fn deposit_to_furnace_fuel(
    furnaces: &mut [FurnaceRecord],
    object_id: ObjectId,
    item: ItemId,
    amount: u32,
) -> u32 {
    let furnace = furnaces
        .iter_mut()
        .find(|furnace| furnace.object_id == object_id);
    let Some(furnace) = furnace else {
        return 0;
    };
    deposit_to_slot(&mut furnace.state.fuel, item, amount)
}
