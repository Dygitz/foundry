use std::collections::BTreeMap;

pub type ItemId = u16;

pub const ITEM_NONE: ItemId = 0;
pub const ITEM_IRON_ORE: ItemId = 1;
pub const ITEM_COPPER_ORE: ItemId = 2;
pub const ITEM_COAL: ItemId = 3;
pub const ITEM_STONE: ItemId = 4;
pub const ITEM_IRON_PLATE: ItemId = 5;
pub const ITEM_COPPER_PLATE: ItemId = 6;
pub const ITEM_FURNACE: ItemId = 7;
pub const ITEM_CHEST: ItemId = 8;
pub const ITEM_INSERTER: ItemId = 9;
pub const ITEM_MINING_DRILL: ItemId = 10;
pub const ITEM_ALEMBIC: ItemId = 11;
pub const ITEM_FERRIC_ESSENCE: ItemId = 12;
pub const ITEM_CUPRIC_ESSENCE: ItemId = 13;
pub const ITEM_UMBRAL_ESSENCE: ItemId = 14;
pub const ITEM_MINERAL_ESSENCE: ItemId = 15;
pub const ITEM_CRUCIBLE: ItemId = 16;
pub const ITEM_LODESTONE: ItemId = 17;
pub const ITEM_BRASS_CORE: ItemId = 18;
pub const ITEM_CINDER_GLASS: ItemId = 19;
pub const ITEM_QUINTESSENCE: ItemId = 20;

pub const INVENTORY_SLOT_COUNT: usize = 32;
pub const INVENTORY_MAX_STACK: u32 = 100;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct InventoryStack {
    pub item: ItemId,
    pub count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Inventory {
    counts: BTreeMap<ItemId, u32>,
}

impl Inventory {
    pub fn count(&self, item: ItemId) -> u32 {
        *self.counts.get(&item).unwrap_or(&0)
    }

    pub fn add(&mut self, item: ItemId, amount: u32) -> u32 {
        if item == ITEM_NONE || amount == 0 {
            return 0;
        }
        let moved = self.acceptable_amount(item, amount);
        if moved == 0 {
            return 0;
        }
        *self.counts.entry(item).or_insert(0) += moved;
        moved
    }

    pub fn can_add(&self, item: ItemId, amount: u32) -> bool {
        self.acceptable_amount(item, amount) == amount
    }

    pub fn acceptable_amount(&self, item: ItemId, amount: u32) -> u32 {
        if item == ITEM_NONE || amount == 0 {
            return 0;
        }
        let current = self.count(item);
        let used_slots = self.used_slots();
        let partial_space = if current == 0 {
            0
        } else {
            let remainder = current % INVENTORY_MAX_STACK;
            if remainder == 0 {
                0
            } else {
                INVENTORY_MAX_STACK - remainder
            }
        };
        let free_slots = INVENTORY_SLOT_COUNT.saturating_sub(used_slots) as u32;
        let capacity = partial_space.saturating_add(free_slots.saturating_mul(INVENTORY_MAX_STACK));
        amount.min(capacity)
    }

    pub fn used_slots(&self) -> usize {
        self.counts
            .values()
            .map(|count| count.div_ceil(INVENTORY_MAX_STACK) as usize)
            .sum()
    }

    pub fn stacks(&self) -> Vec<InventoryStack> {
        self.counts
            .iter()
            .flat_map(|(item, count)| {
                let full = count / INVENTORY_MAX_STACK;
                let remainder = count % INVENTORY_MAX_STACK;
                let mut stacks = Vec::with_capacity(full as usize + usize::from(remainder > 0));
                for _ in 0..full {
                    stacks.push(InventoryStack {
                        item: *item,
                        count: INVENTORY_MAX_STACK,
                    });
                }
                if remainder > 0 {
                    stacks.push(InventoryStack {
                        item: *item,
                        count: remainder,
                    });
                }
                stacks
            })
            .take(INVENTORY_SLOT_COUNT)
            .collect()
    }

    pub fn try_remove(&mut self, item: ItemId, amount: u32) -> bool {
        if amount == 0 {
            return true;
        }
        let cur = self.count(item);
        if cur < amount {
            return false;
        }
        let next = cur - amount;
        if next == 0 {
            self.counts.remove(&item);
        } else {
            self.counts.insert(item, next);
        }
        true
    }

    pub fn entries(&self) -> impl Iterator<Item = (ItemId, u32)> + '_ {
        self.counts.iter().map(|(k, v)| (*k, *v))
    }
}

#[cfg(test)]
mod tests;
