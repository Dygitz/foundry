use std::collections::BTreeMap;

pub type ItemId = u16;

pub const ITEM_NONE: ItemId = 0;
pub const ITEM_IRON_ORE: ItemId = 1;
pub const ITEM_COPPER_ORE: ItemId = 2;
pub const ITEM_COAL: ItemId = 3;
pub const ITEM_STONE: ItemId = 4;

#[derive(Debug, Clone, Default)]
pub struct Inventory {
    counts: BTreeMap<ItemId, u32>,
}

impl Inventory {
    pub fn count(&self, item: ItemId) -> u32 {
        *self.counts.get(&item).unwrap_or(&0)
    }

    pub fn add(&mut self, item: ItemId, amount: u32) {
        if item == ITEM_NONE || amount == 0 {
            return;
        }
        *self.counts.entry(item).or_insert(0) += amount;
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
