use crate::{
    CHUNK_EDGE, CHUNK_TILE_COUNT, ChunkCoord, ChunkLayer, PLACED_NONE, PlacedCell, RES_COAL,
    RES_COPPER, RES_IRON, RES_NONE, RES_STONE, ResourceCell, ResourceId, SimChunkData, TileId,
};

pub const WATER_TILE: TileId = 6;

pub fn generate_chunk_data(
    coord: ChunkCoord,
    layer: ChunkLayer,
    world_seed: u64,
    saved_tick: u64,
) -> SimChunkData {
    let edge = CHUNK_EDGE as usize;
    let base_x = coord.cx * CHUNK_EDGE as i32;
    let base_y = coord.cy * CHUNK_EDGE as i32;
    let mut tiles = Vec::with_capacity(CHUNK_TILE_COUNT);
    for y in 0..edge {
        let gy = base_y + y as i32;
        for x in 0..edge {
            let gx = base_x + x as i32;
            let tile = terrain_tile_id(gx, gy, layer, world_seed);
            tiles.push(tile);
        }
    }
    let resources = generate_resources(coord, layer, world_seed, &tiles);
    let placed = vec![
        PlacedCell {
            kind: PLACED_NONE,
            object_id: 0,
        };
        CHUNK_TILE_COUNT
    ];
    SimChunkData {
        coord,
        layer,
        tiles,
        resources,
        placed,
        chests: Vec::new(),
        furnaces: Vec::new(),
        inserters: Vec::new(),
        entities: Vec::new(),
        saved_tick,
    }
}

pub fn generate_resources(
    coord: ChunkCoord,
    layer: ChunkLayer,
    world_seed: u64,
    tiles: &[TileId],
) -> Vec<ResourceCell> {
    let edge = CHUNK_EDGE as usize;
    let base_x = coord.cx * CHUNK_EDGE as i32;
    let base_y = coord.cy * CHUNK_EDGE as i32;
    let mut resources = Vec::with_capacity(CHUNK_TILE_COUNT);

    for y in 0..edge {
        for x in 0..edge {
            let idx = y * edge + x;
            if tiles.get(idx).copied().map(is_water).unwrap_or(false) {
                resources.push(ResourceCell {
                    kind: RES_NONE,
                    amount: 0,
                });
                continue;
            }
            let gx = base_x + x as i32;
            let gy = base_y + y as i32;
            resources.push(resource_at_global(gx, gy, layer, world_seed));
        }
    }

    resources
}

fn pick_resource_kind(value: u32) -> ResourceId {
    let roll = value % 100;
    if roll < 40 {
        RES_IRON
    } else if roll < 65 {
        RES_COPPER
    } else if roll < 85 {
        RES_COAL
    } else {
        RES_STONE
    }
}

pub fn resource_at_global(gx: i32, gy: i32, layer: ChunkLayer, world_seed: u64) -> ResourceCell {
    const ORE_CELL_SIZE: i32 = 48;
    const ORE_MIN_RADIUS: i32 = 4;
    const ORE_MAX_RADIUS: i32 = 8;
    const ORE_PATCH_CHANCE: u32 = 30;
    const ORE_CELL_MARGIN: i32 = ORE_MAX_RADIUS + 1;

    let max_offset = ORE_CELL_SIZE - (ORE_CELL_MARGIN * 2);
    if max_offset <= 0 {
        return ResourceCell {
            kind: RES_NONE,
            amount: 0,
        };
    }

    let seed = world_seed ^ (layer as u64).wrapping_mul(0x7f4a7c15d14b5b5d);
    let cell_x = gx.div_euclid(ORE_CELL_SIZE);
    let cell_y = gy.div_euclid(ORE_CELL_SIZE);
    let mut best_amount = 0u16;
    let mut best_kind = RES_NONE;

    for cy in (cell_y - 1)..=(cell_y + 1) {
        for cx in (cell_x - 1)..=(cell_x + 1) {
            let cell_seed = mix64(
                seed ^ (cx as i64 as u64).wrapping_mul(0x9e3779b97f4a7c15)
                    ^ (cy as i64 as u64).wrapping_mul(0xc2b2ae3d27d4eb4f),
            );
            if (cell_seed as u32 % 100) >= ORE_PATCH_CHANCE {
                continue;
            }

            let kind = pick_resource_kind(((cell_seed >> 8) as u32) % 100);
            let offset_x = ((cell_seed >> 16) as u32 % max_offset as u32) as i32;
            let offset_y = ((cell_seed >> 24) as u32 % max_offset as u32) as i32;
            let center_x = cx * ORE_CELL_SIZE + ORE_CELL_MARGIN + offset_x;
            let center_y = cy * ORE_CELL_SIZE + ORE_CELL_MARGIN + offset_y;
            let radius_range = (ORE_MAX_RADIUS - ORE_MIN_RADIUS + 1) as u32;
            let radius = ORE_MIN_RADIUS + ((cell_seed >> 32) as u32 % radius_range) as i32;
            let base_amount = 18 + ((cell_seed >> 40) as u32 % 60) as i32;
            let dx = gx - center_x;
            let dy = gy - center_y;
            let dist_sq = dx * dx + dy * dy;
            let radius_sq = radius * radius;
            if dist_sq > radius_sq {
                continue;
            }
            let falloff = (dist_sq * base_amount) / (radius_sq + 1);
            let amount = (base_amount - falloff).max(0) as u16;
            if amount > best_amount {
                best_amount = amount;
                best_kind = kind;
            }
        }
    }

    if best_amount == 0 {
        ResourceCell {
            kind: RES_NONE,
            amount: 0,
        }
    } else {
        ResourceCell {
            kind: best_kind,
            amount: best_amount,
        }
    }
}

pub fn terrain_tile_id(gx: i32, gy: i32, layer: ChunkLayer, world_seed: u64) -> TileId {
    let seed = world_seed ^ (layer as u64).wrapping_mul(0x9e3779b97f4a7c15);
    let coarse_x = gx >> 4;
    let coarse_y = gy >> 4;
    let h = terrain_hash(coarse_x, coarse_y, seed);
    let v = (h & 0xFFFF) as u16;
    let variant = (terrain_hash(gx, gy, seed ^ 0x5bf03635f7d13d9b) >> 8) as u8;
    if v < 5000 {
        WATER_TILE
    } else if v < 16000 {
        4 + (variant % 2) as TileId
    } else {
        (variant % 4) as TileId
    }
}

pub fn tile_at(data: &SimChunkData, tx: i32, ty: i32, world_seed: u64) -> TileId {
    let edge = CHUNK_EDGE as i32;
    if tx >= 0 && tx < edge && ty >= 0 && ty < edge {
        let idx = (ty as usize) * (edge as usize) + (tx as usize);
        return data.tiles.get(idx).copied().unwrap_or(0);
    }
    let gx = data.coord.cx * CHUNK_EDGE as i32 + tx;
    let gy = data.coord.cy * CHUNK_EDGE as i32 + ty;
    terrain_tile_id(gx, gy, data.layer, world_seed)
}

pub fn resource_at(data: &SimChunkData, tx: i32, ty: i32) -> ResourceCell {
    let edge = CHUNK_EDGE as i32;
    if tx >= 0 && tx < edge && ty >= 0 && ty < edge {
        let idx = (ty as usize) * (edge as usize) + (tx as usize);
        return data.resources.get(idx).copied().unwrap_or(ResourceCell {
            kind: RES_NONE,
            amount: 0,
        });
    }
    ResourceCell {
        kind: RES_NONE,
        amount: 0,
    }
}

pub fn placed_at(data: &SimChunkData, tx: i32, ty: i32) -> PlacedCell {
    let edge = CHUNK_EDGE as i32;
    if tx >= 0 && tx < edge && ty >= 0 && ty < edge {
        let idx = (ty as usize) * (edge as usize) + (tx as usize);
        return data.placed.get(idx).copied().unwrap_or(PlacedCell {
            kind: PLACED_NONE,
            object_id: 0,
        });
    }
    PlacedCell {
        kind: PLACED_NONE,
        object_id: 0,
    }
}

pub fn is_water(tile: TileId) -> bool {
    tile == WATER_TILE
}

pub fn tile_jitter(gx: i32, gy: i32, world_seed: u64, tile: TileId) -> i8 {
    let seed = world_seed ^ (tile as u64).wrapping_mul(0x94d049bb133111eb);
    let h = terrain_hash(gx, gy, seed);
    let range = if tile == WATER_TILE { 3 } else { 6 };
    let offset = (h % ((range * 2 + 1) as u32)) as i8 - range;
    offset
}

pub fn terrain_hash(x: i32, y: i32, seed: u64) -> u32 {
    let mut z = seed;
    z ^= (x as i64 as u64).wrapping_mul(0x9e3779b97f4a7c15);
    z ^= (y as i64 as u64).wrapping_mul(0xc2b2ae3d27d4eb4f);
    mix64(z) as u32
}

pub fn mix64(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests;
