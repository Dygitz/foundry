use crate::{
    ITEM_ALEMBIC, ITEM_BRASS_CORE, ITEM_CHEST, ITEM_CINDER_GLASS, ITEM_COAL, ITEM_COPPER_ORE,
    ITEM_COPPER_PLATE, ITEM_CRUCIBLE, ITEM_CUPRIC_ESSENCE, ITEM_FERRIC_ESSENCE, ITEM_FURNACE,
    ITEM_INSERTER, ITEM_IRON_ORE, ITEM_IRON_PLATE, ITEM_LODESTONE, ITEM_MINERAL_ESSENCE,
    ITEM_MINING_DRILL, ITEM_QUINTESSENCE, ITEM_STONE, ITEM_UMBRAL_ESSENCE, Inventory, ItemId,
    ObjectId, PLACED_ALEMBIC, PLACED_CHEST, PLACED_CRUCIBLE, PLACED_FURNACE, PLACED_INSERTER,
    PLACED_MINING_DRILL, PlacedId, RES_COAL, RES_COPPER, RES_IRON, RES_STONE, ResourceId, Slot,
    mix64,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Recipe {
    pub output: ItemId,
    pub output_amount: u32,
    pub inputs: &'static [(ItemId, u32)],
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CrucibleFormula {
    pub output: ItemId,
    pub output_amount: u32,
    pub inputs: &'static [(ItemId, u32)],
}

pub const INVENTORY_ITEMS: [ItemId; 20] = [
    ITEM_STONE,
    ITEM_COPPER_ORE,
    ITEM_COAL,
    ITEM_IRON_ORE,
    ITEM_IRON_PLATE,
    ITEM_COPPER_PLATE,
    ITEM_FERRIC_ESSENCE,
    ITEM_CUPRIC_ESSENCE,
    ITEM_UMBRAL_ESSENCE,
    ITEM_MINERAL_ESSENCE,
    ITEM_LODESTONE,
    ITEM_BRASS_CORE,
    ITEM_CINDER_GLASS,
    ITEM_QUINTESSENCE,
    ITEM_FURNACE,
    ITEM_CHEST,
    ITEM_INSERTER,
    ITEM_MINING_DRILL,
    ITEM_ALEMBIC,
    ITEM_CRUCIBLE,
];

pub const PLACEABLE_ITEMS: [ItemId; 6] = [
    ITEM_FURNACE,
    ITEM_CHEST,
    ITEM_INSERTER,
    ITEM_MINING_DRILL,
    ITEM_ALEMBIC,
    ITEM_CRUCIBLE,
];

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

pub const RECIPE_INSERTER: Recipe = Recipe {
    output: ITEM_INSERTER,
    output_amount: 1,
    inputs: &[(ITEM_IRON_PLATE, 2), (ITEM_COPPER_PLATE, 1)],
};

pub const RECIPE_MINING_DRILL: Recipe = Recipe {
    output: ITEM_MINING_DRILL,
    output_amount: 1,
    inputs: &[
        (ITEM_IRON_PLATE, 3),
        (ITEM_COPPER_PLATE, 1),
        (ITEM_STONE, 5),
    ],
};

pub const RECIPE_ALEMBIC: Recipe = Recipe {
    output: ITEM_ALEMBIC,
    output_amount: 1,
    inputs: &[
        (ITEM_COPPER_PLATE, 3),
        (ITEM_IRON_PLATE, 1),
        (ITEM_STONE, 8),
    ],
};

pub const RECIPE_CRUCIBLE: Recipe = Recipe {
    output: ITEM_CRUCIBLE,
    output_amount: 1,
    inputs: &[
        (ITEM_COPPER_PLATE, 4),
        (ITEM_IRON_PLATE, 2),
        (ITEM_STONE, 12),
        (ITEM_MINERAL_ESSENCE, 2),
    ],
};

pub const RECIPES: [&Recipe; 6] = [
    &RECIPE_FURNACE,
    &RECIPE_CHEST,
    &RECIPE_INSERTER,
    &RECIPE_MINING_DRILL,
    &RECIPE_ALEMBIC,
    &RECIPE_CRUCIBLE,
];

pub const FURNACE_PROGRESS_PER_ITEM: u16 = 1000;
pub const FURNACE_SECONDS_PER_ITEM: f32 = 2.0;
pub const FURNACE_PROGRESS_PER_SEC: f32 =
    FURNACE_PROGRESS_PER_ITEM as f32 / FURNACE_SECONDS_PER_ITEM;

pub const MINING_DRILL_PROGRESS_PER_ITEM: u16 = 1000;
pub const MINING_DRILL_SECONDS_PER_ITEM: f32 = 3.0;
pub const MINING_DRILL_PROGRESS_PER_SEC: f32 =
    MINING_DRILL_PROGRESS_PER_ITEM as f32 / MINING_DRILL_SECONDS_PER_ITEM;

pub const ALEMBIC_PROGRESS_PER_ITEM: u16 = 1000;
pub const ALEMBIC_SECONDS_PER_ITEM: f32 = 4.0;
pub const ALEMBIC_PROGRESS_PER_SEC: f32 =
    ALEMBIC_PROGRESS_PER_ITEM as f32 / ALEMBIC_SECONDS_PER_ITEM;

pub const CRUCIBLE_PROGRESS_PER_ITEM: u16 = 1000;
pub const CRUCIBLE_SECONDS_PER_ITEM: f32 = 5.0;
pub const CRUCIBLE_PROGRESS_PER_SEC: f32 =
    CRUCIBLE_PROGRESS_PER_ITEM as f32 / CRUCIBLE_SECONDS_PER_ITEM;

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

pub fn essence_output_for_input(item: ItemId) -> Option<ItemId> {
    match item {
        ITEM_IRON_ORE => Some(ITEM_FERRIC_ESSENCE),
        ITEM_COPPER_ORE => Some(ITEM_CUPRIC_ESSENCE),
        ITEM_COAL => Some(ITEM_UMBRAL_ESSENCE),
        ITEM_STONE => Some(ITEM_MINERAL_ESSENCE),
        _ => None,
    }
}

pub const FORMULA_QUINTESSENCE: CrucibleFormula = CrucibleFormula {
    output: ITEM_QUINTESSENCE,
    output_amount: 1,
    inputs: &[
        (ITEM_FERRIC_ESSENCE, 1),
        (ITEM_CUPRIC_ESSENCE, 1),
        (ITEM_UMBRAL_ESSENCE, 1),
        (ITEM_MINERAL_ESSENCE, 1),
    ],
};

pub const FORMULA_LODESTONE: CrucibleFormula = CrucibleFormula {
    output: ITEM_LODESTONE,
    output_amount: 1,
    inputs: &[(ITEM_FERRIC_ESSENCE, 1), (ITEM_MINERAL_ESSENCE, 1)],
};

pub const FORMULA_BRASS_CORE: CrucibleFormula = CrucibleFormula {
    output: ITEM_BRASS_CORE,
    output_amount: 1,
    inputs: &[(ITEM_CUPRIC_ESSENCE, 1), (ITEM_FERRIC_ESSENCE, 1)],
};

pub const FORMULA_CINDER_GLASS: CrucibleFormula = CrucibleFormula {
    output: ITEM_CINDER_GLASS,
    output_amount: 1,
    inputs: &[(ITEM_UMBRAL_ESSENCE, 1), (ITEM_MINERAL_ESSENCE, 1)],
};

pub const CRUCIBLE_FORMULAS: [&CrucibleFormula; 4] = [
    &FORMULA_QUINTESSENCE,
    &FORMULA_LODESTONE,
    &FORMULA_BRASS_CORE,
    &FORMULA_CINDER_GLASS,
];

pub fn is_essence_item(item: ItemId) -> bool {
    matches!(
        item,
        ITEM_FERRIC_ESSENCE | ITEM_CUPRIC_ESSENCE | ITEM_UMBRAL_ESSENCE | ITEM_MINERAL_ESSENCE
    )
}

pub fn crucible_formula_for_slots(inputs: &[Slot]) -> Option<&'static CrucibleFormula> {
    CRUCIBLE_FORMULAS.iter().copied().find(|formula| {
        formula
            .inputs
            .iter()
            .all(|(item, amount)| slot_item_count(inputs, *item) >= *amount)
    })
}

fn slot_item_count(inputs: &[Slot], item: ItemId) -> u32 {
    inputs
        .iter()
        .filter(|slot| slot.item == item)
        .map(|slot| slot.count)
        .sum()
}

pub fn try_craft(inv: &mut Inventory, recipe: &Recipe) -> bool {
    if !can_craft(inv, recipe) {
        return false;
    }
    for (item, amount) in recipe.inputs {
        let _ = inv.try_remove(*item, *amount);
    }
    inv.add(recipe.output, recipe.output_amount) == recipe.output_amount
}

pub fn can_craft(inv: &Inventory, recipe: &Recipe) -> bool {
    if !recipe
        .inputs
        .iter()
        .all(|(item, amount)| inv.count(*item) >= *amount)
    {
        return false;
    }
    let mut simulated = inv.clone();
    for (item, amount) in recipe.inputs {
        let _ = simulated.try_remove(*item, *amount);
    }
    simulated.can_add(recipe.output, recipe.output_amount)
}

pub fn recipe_for_index(index: usize) -> Option<&'static Recipe> {
    RECIPES.get(index).copied()
}

pub fn is_placeable_item(item: ItemId) -> bool {
    item_to_placed_kind(item).is_some()
}

pub fn item_name(item: ItemId) -> &'static str {
    match item {
        ITEM_IRON_ORE => "Ferric Ore",
        ITEM_COPPER_ORE => "Cupric Ore",
        ITEM_COAL => "Black Salt",
        ITEM_STONE => "Saltstone",
        ITEM_IRON_PLATE => "Iron Calx",
        ITEM_COPPER_PLATE => "Copper Calx",
        ITEM_FERRIC_ESSENCE => "Ferric Essence",
        ITEM_CUPRIC_ESSENCE => "Cupric Essence",
        ITEM_UMBRAL_ESSENCE => "Umbral Essence",
        ITEM_MINERAL_ESSENCE => "Mineral Essence",
        ITEM_LODESTONE => "Lodestone",
        ITEM_BRASS_CORE => "Brass Core",
        ITEM_CINDER_GLASS => "Cinder Glass",
        ITEM_QUINTESSENCE => "Quintessence",
        ITEM_FURNACE => "Athanor",
        ITEM_CHEST => "Reliquary",
        ITEM_INSERTER => "Brass Arm",
        ITEM_MINING_DRILL => "Extractor",
        ITEM_ALEMBIC => "Alembic",
        ITEM_CRUCIBLE => "Crucible",
        _ => "Unknown",
    }
}

pub fn item_to_placed_kind(item: ItemId) -> Option<PlacedId> {
    match item {
        ITEM_FURNACE => Some(PLACED_FURNACE),
        ITEM_CHEST => Some(PLACED_CHEST),
        ITEM_INSERTER => Some(PLACED_INSERTER),
        ITEM_MINING_DRILL => Some(PLACED_MINING_DRILL),
        ITEM_ALEMBIC => Some(PLACED_ALEMBIC),
        ITEM_CRUCIBLE => Some(PLACED_CRUCIBLE),
        _ => None,
    }
}

pub fn placed_kind_to_item(kind: PlacedId) -> Option<ItemId> {
    match kind {
        PLACED_FURNACE => Some(ITEM_FURNACE),
        PLACED_CHEST => Some(ITEM_CHEST),
        PLACED_INSERTER => Some(ITEM_INSERTER),
        PLACED_MINING_DRILL => Some(ITEM_MINING_DRILL),
        PLACED_ALEMBIC => Some(ITEM_ALEMBIC),
        PLACED_CRUCIBLE => Some(ITEM_CRUCIBLE),
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
