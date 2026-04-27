use super::*;

fn test_chunk() -> SimChunkData {
    SimChunkData {
        coord: ChunkCoord::new(0, 0),
        layer: 0,
        tiles: vec![0; CHUNK_TILE_COUNT],
        resources: vec![
            ResourceCell {
                kind: RES_NONE,
                amount: 0,
            };
            CHUNK_TILE_COUNT
        ],
        placed: vec![
            PlacedCell {
                kind: PLACED_NONE,
                object_id: 0,
            };
            CHUNK_TILE_COUNT
        ],
        chests: Vec::new(),
        furnaces: Vec::new(),
        inserters: Vec::new(),
        drills: Vec::new(),
        entities: Vec::new(),
        saved_tick: 0,
    }
}

fn local_idx(tx: usize, ty: usize) -> usize {
    ty * CHUNK_EDGE as usize + tx
}

fn map_pixel(pixels: &[u8], tx: usize, ty: usize) -> [u8; 4] {
    let edge = CHUNK_EDGE as usize;
    let row = edge - 1 - ty;
    let idx = (row * edge + tx) * 4;
    [
        pixels[idx],
        pixels[idx + 1],
        pixels[idx + 2],
        pixels[idx + 3],
    ]
}

fn raw_pixel(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let edge = CHUNK_EDGE as usize;
    let idx = (y * edge + x) * 4;
    [
        pixels[idx],
        pixels[idx + 1],
        pixels[idx + 2],
        pixels[idx + 3],
    ]
}

fn padded_map_pixel(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let padded_edge = CHUNK_EDGE as usize + 2;
    let idx = (y * padded_edge + x) * 4;
    [
        pixels[idx],
        pixels[idx + 1],
        pixels[idx + 2],
        pixels[idx + 3],
    ]
}

fn chunk_pixel(pixels: &[u8], tx: usize, ty: usize) -> [u8; 4] {
    let edge = CHUNK_EDGE as usize;
    let padded = edge + 2;
    let ox = tx + 1;
    let oy = edge - ty;
    let idx = (oy * padded + ox) * 4;
    [
        pixels[idx],
        pixels[idx + 1],
        pixels[idx + 2],
        pixels[idx + 3],
    ]
}

fn blank_map_chunk() -> MapChunk {
    MapChunk {
        rgba: vec![0; MAP_CHUNK_BYTES],
        resource_kinds: vec![RES_NONE; CHUNK_TILE_COUNT],
        resource_amounts: vec![0; CHUNK_TILE_COUNT],
        image: Handle::<Image>::default(),
        updated_at_ms: 0,
    }
}

#[test]
fn map_snapshot_has_one_rgba_pixel_per_tile() {
    let data = test_chunk();
    let pixels = map_snapshot_pixels(&data, 7);

    assert_eq!(pixels.len(), MAP_CHUNK_BYTES);
    assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] == 255));
}

#[test]
fn padded_map_chunk_pixels_duplicate_all_edges() {
    let edge = CHUNK_EDGE as usize;
    let padded_edge = edge + 2;
    let mut rgba = Vec::with_capacity(MAP_CHUNK_BYTES);
    for y in 0..edge {
        for x in 0..edge {
            rgba.extend_from_slice(&[x as u8, y as u8, (x + y) as u8, 255]);
        }
    }

    let padded = padded_map_chunk_pixels(&rgba);

    assert_eq!(padded.len(), padded_edge * padded_edge * 4);
    for y in 0..edge {
        for x in 0..edge {
            assert_eq!(
                padded_map_pixel(&padded, x + 1, y + 1),
                raw_pixel(&rgba, x, y)
            );
        }
    }
    for x in 0..edge {
        assert_eq!(padded_map_pixel(&padded, x + 1, 0), raw_pixel(&rgba, x, 0));
        assert_eq!(
            padded_map_pixel(&padded, x + 1, padded_edge - 1),
            raw_pixel(&rgba, x, edge - 1)
        );
    }
    for y in 0..edge {
        assert_eq!(padded_map_pixel(&padded, 0, y + 1), raw_pixel(&rgba, 0, y));
        assert_eq!(
            padded_map_pixel(&padded, padded_edge - 1, y + 1),
            raw_pixel(&rgba, edge - 1, y)
        );
    }
    assert_eq!(padded_map_pixel(&padded, 0, 0), raw_pixel(&rgba, 0, 0));
    assert_eq!(
        padded_map_pixel(&padded, padded_edge - 1, 0),
        raw_pixel(&rgba, edge - 1, 0)
    );
    assert_eq!(
        padded_map_pixel(&padded, 0, padded_edge - 1),
        raw_pixel(&rgba, 0, edge - 1)
    );
    assert_eq!(
        padded_map_pixel(&padded, padded_edge - 1, padded_edge - 1),
        raw_pixel(&rgba, edge - 1, edge - 1)
    );
}

#[test]
fn map_chunk_source_rects_select_interior_and_minimap_bleed() {
    let edge = CHUNK_EDGE as f32;

    let normal = map_chunk_source_rect(false);
    assert_eq!(normal.min, Vec2::new(1.0, 1.0));
    assert_eq!(normal.max, Vec2::new(edge + 1.0, edge + 1.0));

    let minimap = map_chunk_source_rect(true);
    assert_eq!(minimap.min, Vec2::new(1.0, 1.0));
    assert_eq!(minimap.max, Vec2::new(edge + 2.0, edge + 2.0));
}

#[test]
fn map_snapshot_includes_resource_overlay() {
    let mut data = test_chunk();
    let base = map_snapshot_pixels(&data, 7);
    data.resources[local_idx(5, 6)] = ResourceCell {
        kind: RES_IRON,
        amount: 12,
    };
    let with_resource = map_snapshot_pixels(&data, 7);

    assert_ne!(map_pixel(&base, 5, 6), map_pixel(&with_resource, 5, 6));
}

#[test]
fn map_resource_metadata_tracks_known_resource_amounts() {
    let mut data = test_chunk();
    data.resources[local_idx(3, 4)] = ResourceCell {
        kind: RES_COAL,
        amount: 9,
    };
    data.resources[local_idx(5, 6)] = ResourceCell {
        kind: RES_IRON,
        amount: 0,
    };

    let (resource_kinds, resource_amounts) = map_resource_metadata(&data);

    assert_eq!(resource_kinds.len(), CHUNK_TILE_COUNT);
    assert_eq!(resource_amounts.len(), CHUNK_TILE_COUNT);
    assert_eq!(resource_kinds[local_idx(3, 4)], RES_COAL);
    assert_eq!(resource_amounts[local_idx(3, 4)], 9);
    assert_eq!(resource_kinds[local_idx(5, 6)], RES_NONE);
    assert_eq!(resource_amounts[local_idx(5, 6)], 0);
}

#[test]
fn map_snapshot_includes_placed_overlay() {
    let mut data = test_chunk();
    let base = map_snapshot_pixels(&data, 7);
    data.placed[local_idx(8, 9)] = PlacedCell {
        kind: PLACED_CHEST,
        object_id: 42,
    };
    let with_placed = map_snapshot_pixels(&data, 7);

    assert_ne!(map_pixel(&base, 8, 9), map_pixel(&with_placed, 8, 9));
}

#[test]
fn map_snapshot_orientation_matches_chunk_texture_interior() {
    let data = test_chunk();
    let config = WorldRenderConfig::default();
    let map_pixels = map_snapshot_pixels(&data, 7);
    let chunk_pixels = chunk_pixels(&data, &config, 7, None);

    assert_eq!(
        map_pixel(&map_pixels, 11, 13),
        chunk_pixel(&chunk_pixels, 11, 13)
    );
}

#[test]
fn map_resource_node_summary_sums_connected_explored_tiles_across_chunks() {
    let session = WorldSession::default();
    let layer = 0;
    let mut map = MapState::default();
    let mut left_chunk = blank_map_chunk();
    let mut right_chunk = blank_map_chunk();

    left_chunk.resource_kinds[local_idx(31, 0)] = RES_IRON;
    left_chunk.resource_amounts[local_idx(31, 0)] = 5;
    right_chunk.resource_kinds[local_idx(0, 0)] = RES_IRON;
    right_chunk.resource_amounts[local_idx(0, 0)] = 7;
    right_chunk.resource_kinds[local_idx(2, 0)] = RES_IRON;
    right_chunk.resource_amounts[local_idx(2, 0)] = 100;

    map.explored.insert(
        ChunkKey::new(session.world_id.clone(), ChunkCoord::new(0, 0), layer),
        left_chunk,
    );
    map.explored.insert(
        ChunkKey::new(session.world_id.clone(), ChunkCoord::new(1, 0), layer),
        right_chunk,
    );

    let summary = map_resource_node_summary(&map, &session, layer, 31, 0).unwrap();

    assert_eq!(summary.kind, RES_IRON);
    assert_eq!(summary.total, 12);
}
