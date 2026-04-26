use crate::{
    ITEM_CHEST, ITEM_COAL, ITEM_COPPER_ORE, ITEM_COPPER_PLATE, ITEM_FURNACE, ITEM_IRON_ORE,
    ITEM_IRON_PLATE, ITEM_STONE, Inventory, ItemId, ObjectId, PLACED_CHEST, PLACED_FURNACE,
    PlacedId, RES_COAL, RES_COPPER, RES_IRON, RES_STONE, ResourceId, mix64,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Recipe {
    pub output: ItemId,
    pub output_amount: u32,
    pub inputs: &'static [(ItemId, u32)],
}

pub const INVENTORY_ITEMS: [ItemId; 8] = [
    ITEM_STONE,
    ITEM_COPPER_ORE,
    ITEM_COAL,
    ITEM_IRON_ORE,
    ITEM_IRON_PLATE,
    ITEM_COPPER_PLATE,
    ITEM_FURNACE,
    ITEM_CHEST,
];

pub const PLACEABLE_ITEMS: [ItemId; 2] = [ITEM_FURNACE, ITEM_CHEST];

pub const RECIPE_FURNACE: Recipe = Recipe {
    output: ITEM_FURNACE,
    output_amount: 1,
    inputs: &[(ITEM_STONE, 10)],
};

pub const RECIPE_CHEST: Recipe = Recipe {
    output: ITEM_CHEST,
    output_amount: 1,
    inputs: &[(ITEM_STONE, 10)],
};

pub const RECIPES: [&Recipe; 2] = [&RECIPE_FURNACE, &RECIPE_CHEST];

pub const FURNACE_PROGRESS_PER_ITEM: u16 = 1000;
pub const FURNACE_SECONDS_PER_ITEM: f32 = 2.0;
pub const FURNACE_PROGRESS_PER_SEC: f32 =
    FURNACE_PROGRESS_PER_ITEM as f32 / FURNACE_SECONDS_PER_ITEM;

pub fn resource_to_item(kind: ResourceId) -> Option<ItemId> {
    match kind {
        RES_IRON => Some(ITEM_IRON_ORE),
        RES_COPPER => Some(ITEM_COPPER_ORE),
        RES_COAL => Some(ITEM_COAL),
        RES_STONE => Some(ITEM_STONE),
        _ => None,
    }
}

pub fn smelt_output_for_input(item: ItemId) -> Option<ItemId> {
    match item {
        ITEM_IRON_ORE => Some(ITEM_IRON_PLATE),
        ITEM_COPPER_ORE => Some(ITEM_COPPER_PLATE),
        _ => None,
    }
}

pub fn try_craft(inv: &mut Inventory, recipe: &Recipe) -> bool {
    if !can_craft(inv, recipe) {
        return false;
    }
    for (item, amount) in recipe.inputs {
        let _ = inv.try_remove(*item, *amount);
    }
    inv.add(recipe.output, recipe.output_amount);
    true
}

pub fn can_craft(inv: &Inventory, recipe: &Recipe) -> bool {
    recipe
        .inputs
        .iter()
        .all(|(item, amount)| inv.count(*item) >= *amount)
}

pub fn recipe_for_index(index: usize) -> Option<&'static Recipe> {
    RECIPES.get(index).copied()
}

pub fn is_placeable_item(item: ItemId) -> bool {
    item_to_placed_kind(item).is_some()
}

pub fn item_name(item: ItemId) -> &'static str {
    match item {
        ITEM_IRON_ORE => "Iron Ore",
        ITEM_COPPER_ORE => "Copper Ore",
        ITEM_COAL => "Coal",
        ITEM_STONE => "Stone",
        ITEM_IRON_PLATE => "Iron Plate",
        ITEM_COPPER_PLATE => "Copper Plate",
        ITEM_FURNACE => "Furnace",
        ITEM_CHEST => "Chest",
        _ => "Unknown",
    }
}

pub fn item_to_placed_kind(item: ItemId) -> Option<PlacedId> {
    match item {
        ITEM_FURNACE => Some(PLACED_FURNACE),
        ITEM_CHEST => Some(PLACED_CHEST),
        _ => None,
    }
}

pub fn object_id_for_tile(world_seed: u64, gx: i32, gy: i32, kind: PlacedId) -> ObjectId {
    let mut z = world_seed ^ (kind as u64).wrapping_mul(0x9e3779b97f4a7c15);
    z ^= (gx as i64 as u64).wrapping_mul(0xbf58476d1ce4e5b9);
    z ^= (gy as i64 as u64).wrapping_mul(0x94d049bb133111eb);
    let id = mix64(z);
    if id == 0 { 1 } else { id }
}

#[cfg(test)]
mod tests;
