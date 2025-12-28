#[derive(Debug, Clone)]
pub struct SimChunkData {
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct SimChunkView<'a> {
    pub payload: &'a [u8],
}
