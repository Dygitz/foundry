use super::*;

#[test]
fn tile_to_chunk_local_handles_negative_tiles() {
    assert_eq!(
        tile_to_chunk_local(-1, -33),
        (ChunkCoord::new(-1, -2), 31, 31)
    );
    assert_eq!(
        tile_to_chunk_local(-32, -32),
        (ChunkCoord::new(-1, -1), 0, 0)
    );
}

#[test]
fn tile_to_chunk_local_handles_positive_boundary() {
    assert_eq!(
        tile_to_chunk_local(CHUNK_EDGE as i32, CHUNK_EDGE as i32),
        (ChunkCoord::new(1, 1), 0, 0)
    );
}
