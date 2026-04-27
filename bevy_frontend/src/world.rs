#![allow(unused_imports)]
use crate::imports::*;
use crate::{
    app::*, camera::*, components::*, gameplay::*, map::*, player::*, rendering::*, resources::*,
    storage::*, ui::*,
};

pub(crate) struct FoundryWorldPlugin;

impl Plugin for FoundryWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (active_area_chunk_request_system,).in_set(UpdateSet::World),
        );
    }
}

pub(crate) fn active_area_chunk_request_system(
    windows: Query<&Window>,
    camera_query: Query<(&Transform, &Projection, &Camera), With<Camera2d>>,
    config: Res<WorldRenderConfig>,
    cache: Res<ChunkCacheConfig>,
    session: Res<WorldSession>,
    mut runtime: ResMut<WorldRuntime>,
    mut requests: EventWriter<ChunkLoadRequest>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera_transform, projection, camera)) = camera_query.single() else {
        return;
    };

    let chunk_size = chunk_world_size(&config);
    let viewport = camera
        .logical_viewport_size()
        .unwrap_or_else(|| Vec2::new(window.width(), window.height()));
    let scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    };
    let camera_pos = camera_transform.translation.truncate();
    let center_coord = world_to_chunk_coord(camera_pos, chunk_size);
    let margin = config.active_radius_chunks.max(1);
    let (rx, ry) = required_radius_chunks(viewport, scale, chunk_size, margin);
    let keep_rx = (rx + cache.keep_radius_chunks).max(rx);
    let keep_ry = (ry + cache.keep_radius_chunks).max(ry);
    let mut active_set = HashSet::new();
    let mut keep_set = HashSet::new();

    for dy in -ry..=ry {
        for dx in -rx..=rx {
            let coord = ChunkCoord::new(center_coord.cx + dx, center_coord.cy + dy);
            let key = ChunkKey::new(session.world_id.clone(), coord, config.layer);
            active_set.insert(key.clone());
            if runtime.ensure_loaded(&key) {
                runtime.touch(&key);
            } else if runtime.requested.insert(key.clone()) {
                requests.write(ChunkLoadRequest { key });
            }
        }
    }
    for dy in -keep_ry..=keep_ry {
        for dx in -keep_rx..=keep_rx {
            let coord = ChunkCoord::new(center_coord.cx + dx, center_coord.cy + dy);
            let key = ChunkKey::new(session.world_id.clone(), coord, config.layer);
            keep_set.insert(key);
        }
    }
    runtime.active_set = active_set;
    runtime.keep_set = keep_set;
}

pub(crate) fn world_to_chunk_coord(world_pos: Vec2, chunk_size: f32) -> ChunkCoord {
    ChunkCoord::new(
        (world_pos.x / chunk_size).floor() as i32,
        (world_pos.y / chunk_size).floor() as i32,
    )
}

pub(crate) fn chunk_center_world(coord: ChunkCoord, chunk_size: f32) -> Vec2 {
    Vec2::new(
        (coord.cx as f32 + 0.5) * chunk_size,
        (coord.cy as f32 + 0.5) * chunk_size,
    )
}

pub(crate) fn open_panel_for_tile(
    tile_x: i32,
    tile_y: i32,
    config: &WorldRenderConfig,
    session: &WorldSession,
    runtime: &WorldRuntime,
) -> Option<UiMode> {
    let (coord, local_x, local_y) = tile_to_chunk_local(tile_x, tile_y);
    let key = ChunkKey::new(session.world_id.clone(), coord, config.layer);
    let Some(loaded) = runtime.loaded.get(&key) else {
        return None;
    };
    let edge = CHUNK_EDGE as i32;
    let idx = (local_y as usize) * (edge as usize) + (local_x as usize);
    let cell = loaded.data.placed.get(idx)?;
    if cell.kind == PLACED_CHEST && cell.object_id != 0 {
        return Some(UiMode::Chest {
            object_id: cell.object_id,
        });
    }
    if cell.kind == PLACED_FURNACE && cell.object_id != 0 {
        return Some(UiMode::Furnace {
            object_id: cell.object_id,
        });
    }
    if cell.kind == PLACED_INSERTER && cell.object_id != 0 {
        return Some(UiMode::Inserter {
            object_id: cell.object_id,
        });
    }
    if cell.kind == PLACED_MINING_DRILL && cell.object_id != 0 {
        return Some(UiMode::MiningDrill {
            object_id: cell.object_id,
        });
    }
    if cell.kind == PLACED_ALEMBIC && cell.object_id != 0 {
        return Some(UiMode::Alembic {
            object_id: cell.object_id,
        });
    }
    if cell.kind == PLACED_CRUCIBLE && cell.object_id != 0 {
        return Some(UiMode::Crucible {
            object_id: cell.object_id,
        });
    }
    None
}

pub(crate) fn find_chest<'a>(
    runtime: &'a WorldRuntime,
    object_id: ObjectId,
) -> Option<&'a ChestRecord> {
    runtime
        .loaded
        .values()
        .find_map(|loaded| loaded.data.chests.iter().find(|c| c.object_id == object_id))
}

pub(crate) fn find_furnace<'a>(
    runtime: &'a WorldRuntime,
    object_id: ObjectId,
) -> Option<&'a FurnaceRecord> {
    runtime.loaded.values().find_map(|loaded| {
        loaded
            .data
            .furnaces
            .iter()
            .find(|f| f.object_id == object_id)
    })
}

pub(crate) fn find_inserter<'a>(
    runtime: &'a WorldRuntime,
    object_id: ObjectId,
) -> Option<&'a InserterRecord> {
    runtime.loaded.values().find_map(|loaded| {
        loaded
            .data
            .inserters
            .iter()
            .find(|i| i.object_id == object_id)
    })
}

pub(crate) fn find_drill<'a>(
    runtime: &'a WorldRuntime,
    object_id: ObjectId,
) -> Option<&'a DrillRecord> {
    runtime
        .loaded
        .values()
        .find_map(|loaded| loaded.data.drills.iter().find(|d| d.object_id == object_id))
}

pub(crate) fn find_alembic<'a>(
    runtime: &'a WorldRuntime,
    object_id: ObjectId,
) -> Option<&'a AlembicRecord> {
    runtime.loaded.values().find_map(|loaded| {
        loaded
            .data
            .alembics
            .iter()
            .find(|a| a.object_id == object_id)
    })
}

pub(crate) fn find_crucible<'a>(
    runtime: &'a WorldRuntime,
    object_id: ObjectId,
) -> Option<&'a CrucibleRecord> {
    runtime.loaded.values().find_map(|loaded| {
        loaded
            .data
            .crucibles
            .iter()
            .find(|c| c.object_id == object_id)
    })
}

pub(crate) fn with_chest_mut<R>(
    runtime: &mut WorldRuntime,
    object_id: ObjectId,
    mut f: impl FnMut(&mut SimChunkData) -> R,
) -> Option<(ChunkKey, SimChunkData, R)> {
    for (key, loaded) in runtime.loaded.iter_mut() {
        if loaded
            .data
            .chests
            .iter()
            .any(|chest| chest.object_id == object_id)
        {
            let result = f(&mut loaded.data);
            let snapshot = loaded.data.clone();
            return Some((key.clone(), snapshot, result));
        }
    }
    None
}

pub(crate) fn with_furnace_mut<R>(
    runtime: &mut WorldRuntime,
    object_id: ObjectId,
    mut f: impl FnMut(&mut SimChunkData) -> R,
) -> Option<(ChunkKey, SimChunkData, R)> {
    for (key, loaded) in runtime.loaded.iter_mut() {
        if loaded
            .data
            .furnaces
            .iter()
            .any(|furnace| furnace.object_id == object_id)
        {
            let result = f(&mut loaded.data);
            let snapshot = loaded.data.clone();
            return Some((key.clone(), snapshot, result));
        }
    }
    None
}

pub(crate) fn with_inserter_mut<R>(
    runtime: &mut WorldRuntime,
    object_id: ObjectId,
    mut f: impl FnMut(&mut SimChunkData) -> R,
) -> Option<(ChunkKey, SimChunkData, R)> {
    for (key, loaded) in runtime.loaded.iter_mut() {
        if loaded
            .data
            .inserters
            .iter()
            .any(|inserter| inserter.object_id == object_id)
        {
            let result = f(&mut loaded.data);
            let snapshot = loaded.data.clone();
            return Some((key.clone(), snapshot, result));
        }
    }
    None
}

pub(crate) fn with_drill_mut<R>(
    runtime: &mut WorldRuntime,
    object_id: ObjectId,
    mut f: impl FnMut(&mut SimChunkData) -> R,
) -> Option<(ChunkKey, SimChunkData, R)> {
    for (key, loaded) in runtime.loaded.iter_mut() {
        if loaded
            .data
            .drills
            .iter()
            .any(|drill| drill.object_id == object_id)
        {
            let result = f(&mut loaded.data);
            let snapshot = loaded.data.clone();
            return Some((key.clone(), snapshot, result));
        }
    }
    None
}

pub(crate) fn with_alembic_mut<R>(
    runtime: &mut WorldRuntime,
    object_id: ObjectId,
    mut f: impl FnMut(&mut SimChunkData) -> R,
) -> Option<(ChunkKey, SimChunkData, R)> {
    for (key, loaded) in runtime.loaded.iter_mut() {
        if loaded
            .data
            .alembics
            .iter()
            .any(|alembic| alembic.object_id == object_id)
        {
            let result = f(&mut loaded.data);
            let snapshot = loaded.data.clone();
            return Some((key.clone(), snapshot, result));
        }
    }
    None
}

pub(crate) fn with_crucible_mut<R>(
    runtime: &mut WorldRuntime,
    object_id: ObjectId,
    mut f: impl FnMut(&mut SimChunkData) -> R,
) -> Option<(ChunkKey, SimChunkData, R)> {
    for (key, loaded) in runtime.loaded.iter_mut() {
        if loaded
            .data
            .crucibles
            .iter()
            .any(|crucible| crucible.object_id == object_id)
        {
            let result = f(&mut loaded.data);
            let snapshot = loaded.data.clone();
            return Some((key.clone(), snapshot, result));
        }
    }
    None
}
