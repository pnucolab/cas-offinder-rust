pub const CHUNK_SIZE: usize = 1 << 16;
pub const CHUNK_SIZE_BYTES: usize = CHUNK_SIZE / 2;

pub struct ChromChunkInfo {
    pub chr_name: String,
    // fixed size chunk data
    pub data: Box<[u8; CHUNK_SIZE_BYTES]>,
    // start and end of data within chromosome, by nucleotide
    pub chunk_start: u64,
    pub chunk_end: u64,
}

impl ChromChunkInfo {
    pub fn size(&self) -> usize {
        (self.chunk_end - self.chunk_start) as usize
    }
}
pub struct Match {
    pub chr_name: String,
    pub dna_seq: Vec<u8>,
    pub rna_seq: Vec<u8>,
    pub chrom_idx: u64,
    pub pattern_idx: u32,
    pub mismatches: u32,
    pub is_forward: bool,
}
