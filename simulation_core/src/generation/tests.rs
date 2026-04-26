use super::*;

#[test]
fn generated_chunks_are_deterministic() {
    let a = generate_chunk_data(ChunkCoord::new(-2, 3), 0, 1337, 11);
    let b = generate_chunk_data(ChunkCoord::new(-2, 3), 0, 1337, 99);

    assert_eq!(a.tiles, b.tiles);
    assert_eq!(a.resources, b.resources);
    assert_eq!(a.placed, b.placed);
    assert_eq!(a.saved_tick, 11);
    assert_eq!(b.saved_tick, 99);
}

#[test]
fn generated_resources_skip_water_tiles() {
    let tiles = vec![WATER_TILE; CHUNK_TILE_COUNT];
    let resources = generate_resources(ChunkCoord::new(0, 0), 0, 1337, &tiles);

    assert_eq!(resources.len(), CHUNK_TILE_COUNT);
    assert!(resources.iter().all(|cell| cell.kind == RES_NONE));
    assert!(resources.iter().all(|cell| cell.amount == 0));
}
