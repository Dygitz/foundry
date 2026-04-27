use crate::{ITEM_COAL, ITEM_NONE, ItemId, ObjectId};

pub const CHEST_SLOT_COUNT: usize = 16;
pub const INSERTER_SLOT_COUNT: usize = 4;

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
pub struct InserterInv {
    pub slots: [Slot; INSERTER_SLOT_COUNT],
}

impl Default for InserterInv {
    fn default() -> Self {
        Self {
            slots: [Slot::default(); INSERTER_SLOT_COUNT],
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InserterDirection {
    Up,
    Right,
    Down,
    Left,
}

impl Default for InserterDirection {
    fn default() -> Self {
        Self::Right
    }
}

impl InserterDirection {
    pub fn next_clockwise(self) -> Self {
        match self {
            Self::Up => Self::Right,
            Self::Right => Self::Down,
            Self::Down => Self::Left,
            Self::Left => Self::Up,
        }
    }

    pub fn forward_offset(self) -> (i32, i32) {
        match self {
            Self::Up => (0, 1),
            Self::Right => (1, 0),
            Self::Down => (0, -1),
            Self::Left => (-1, 0),
        }
    }

    pub fn back_offset(self) -> (i32, i32) {
        let (dx, dy) = self.forward_offset();
        (-dx, -dy)
    }

    pub fn to_u8(self) -> u8 {
        match self {
            Self::Up => 0,
            Self::Right => 1,
            Self::Down => 2,
            Self::Left => 3,
        }
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Up),
            1 => Some(Self::Right),
            2 => Some(Self::Down),
            3 => Some(Self::Left),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Right => "Right",
            Self::Down => "Down",
            Self::Left => "Left",
        }
    }

    pub fn arrow(self) -> &'static str {
        match self {
            Self::Up => "^",
            Self::Right => ">",
            Self::Down => "v",
            Self::Left => "<",
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
pub struct DrillState {
    pub fuel: Slot,
    pub output: Slot,
    pub progress: u16,
}

impl Default for DrillState {
    fn default() -> Self {
        Self {
            fuel: Slot::default(),
            output: Slot::default(),
            progress: 0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DrillRecord {
    pub object_id: ObjectId,
    pub state: DrillState,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct InserterRecord {
    pub object_id: ObjectId,
    pub direction: InserterDirection,
    pub inv: InserterInv,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FurnaceSlot {
    Input,
    Fuel,
    Output,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DrillSlot {
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

pub fn take_from_drill(
    drills: &mut [DrillRecord],
    object_id: ObjectId,
    slot_kind: DrillSlot,
    amount: u32,
) -> Option<Slot> {
    let drill = drills
        .iter_mut()
        .find(|drill| drill.object_id == object_id)?;
    let slot = match slot_kind {
        DrillSlot::Fuel => &mut drill.state.fuel,
        DrillSlot::Output => &mut drill.state.output,
    };
    take_from_slot(slot, amount)
}

pub fn deposit_to_drill_fuel(
    drills: &mut [DrillRecord],
    object_id: ObjectId,
    item: ItemId,
    amount: u32,
) -> u32 {
    if item != ITEM_COAL {
        return 0;
    }
    let drill = drills.iter_mut().find(|drill| drill.object_id == object_id);
    let Some(drill) = drill else {
        return 0;
    };
    deposit_to_slot(&mut drill.state.fuel, item, amount)
}

pub fn take_from_inserter(
    inserters: &mut [InserterRecord],
    object_id: ObjectId,
    slot_idx: usize,
    amount: u32,
) -> Option<Slot> {
    let inserter = inserters
        .iter_mut()
        .find(|inserter| inserter.object_id == object_id)?;
    let slot = inserter.inv.slots.get_mut(slot_idx)?;
    take_from_slot(slot, amount)
}

pub fn deposit_to_inserter(
    inserters: &mut [InserterRecord],
    object_id: ObjectId,
    item: ItemId,
    amount: u32,
) -> u32 {
    let inserter = inserters
        .iter_mut()
        .find(|inserter| inserter.object_id == object_id);
    let Some(inserter) = inserter else {
        return 0;
    };
    for slot in &mut inserter.inv.slots {
        let moved = deposit_to_slot(slot, item, amount);
        if moved > 0 {
            return moved;
        }
    }
    0
}

#[cfg(test)]
mod tests;
