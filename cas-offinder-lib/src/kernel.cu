// CUDA port of the legacy popcount search kernel (db=0, rb=0 path).
//
// Compile-time defines injected by nvcc in build.rs:
//   PATTERN_LEN, BLOCKS_PER_EXEC, PATTERN_CHUNK_SIZE, block_ty
//
// Signed mismatch comparison + match_start_min_nucl threshold carried over
// from the OpenCL version verbatim so the host logic (main.rs post-hoc
// filter, head-overlap dedup) stays unchanged.

// nvrtc JIT doesn't have access to host <stdint.h> / <cstdint>, so declare
// the fixed-width ints we use locally.
typedef unsigned char      uint8_t;
typedef unsigned short     uint16_t;
typedef unsigned int       uint32_t;
typedef unsigned long long uint64_t;

#ifndef PATTERN_LEN
#define PATTERN_LEN 27
#endif
#ifndef BLOCKS_PER_EXEC
#define BLOCKS_PER_EXEC 4
#endif
#ifndef PATTERN_CHUNK_SIZE
#define PATTERN_CHUNK_SIZE 16
#endif
#ifndef block_ty
#define block_ty uint32_t
#endif

#define cdiv(x, y) (((x) + (y) - 1) / (y))
#define BLOCKS_AVAIL (sizeof(block_ty) * 2)
#define BLOCKS_PER_PATTERN (cdiv(PATTERN_LEN, BLOCKS_AVAIL))
#define PATTERN_OFFSET \
    (cdiv(PATTERN_LEN, 2 * PATTERN_CHUNK_SIZE) * PATTERN_CHUNK_SIZE / sizeof(block_ty))

struct s_match {
    uint32_t loc;
    uint32_t pattern_idx;
    uint32_t mismatches;
    uint16_t dna_bulge_size;
    uint16_t rna_bulge_size;
};

// popcount dispatch: nvcc's __popc / __popcll are intrinsics — pick the one
// that matches block_ty's width.
__device__ __forceinline__ uint32_t popcount_block(block_ty v) {
    if (sizeof(block_ty) == 4) {
        return __popc((uint32_t)v);
    } else {
        return __popcll((uint64_t)v);
    }
}

// Match output buffer capacity (in s_match entries). The host allocates a
// buffer of exactly this size; nvrtc bakes in the real value via -D. This
// fallback only applies if the define is somehow missing.
#ifndef OUT_BUF_SIZE
#define OUT_BUF_SIZE (4 * 1024 * 1024)
#endif

extern "C" __global__ void find_matches(
    const block_ty* __restrict__ genome,
    const block_ty* __restrict__ pattern_blocks,
    uint32_t max_mismatches,
    uint32_t match_start_min_nucl,
    uint32_t exec_start,
    uint32_t exec_end,
    uint32_t n_patterns,
    uint32_t pattern_start,
    s_match* __restrict__ match_buffer,
    int* __restrict__ entrycount
) {
    // `exec_start` offsets the thread index so the host can launch the kernel
    // over a sub-range [exec_start, exec_end) of execs. This lets the host
    // bisect the range and re-run when a chunk produces more candidates than
    // the output buffer can hold (mirrors the Myers kernel's j_start/j_end).
    uint32_t exec_idx = blockIdx.x * blockDim.x + threadIdx.x + exec_start;
    // `pattern_start` offsets the y-index so the host can sweep patterns in
    // batches: CUDA caps grid_dim.y at 65535, so large guide sets are launched
    // over pattern batches with the same atomic out_count (mirrors Myers).
    uint32_t pattern_block_idx = blockIdx.y * blockDim.y + threadIdx.y + pattern_start;
    // CUDA rounds the grid up to a multiple of the block size, so ignore the
    // trailing threads that would otherwise read past `exec_end`.
    if (exec_idx >= exec_end) return;
    if (pattern_block_idx >= n_patterns) return;
    size_t genome_idx = (size_t)exec_idx * BLOCKS_PER_EXEC;

    block_ty shifted_blocks[BLOCKS_PER_PATTERN + BLOCKS_PER_EXEC];
    for (size_t i = 0; i < BLOCKS_PER_PATTERN + BLOCKS_PER_EXEC; i++) {
        shifted_blocks[i] = genome[genome_idx + i];
    }
    for (size_t k = 0; k < BLOCKS_AVAIL; k++) {
        uint32_t counts[BLOCKS_PER_EXEC] = {0};
#pragma unroll
        for (size_t l = 0; l < BLOCKS_PER_PATTERN; l++) {
#pragma unroll
            for (size_t o = 0; o < BLOCKS_PER_EXEC; o++) {
                counts[o] += popcount_block(
                    shifted_blocks[l + o] &
                    pattern_blocks[pattern_block_idx * PATTERN_OFFSET + l]
                );
            }
        }
#pragma unroll
        for (size_t l = 0; l < BLOCKS_PER_PATTERN + BLOCKS_PER_EXEC - 1; l++) {
            shifted_blocks[l] =
                (shifted_blocks[l] >> 4) |
                (shifted_blocks[l + 1] << ((BLOCKS_AVAIL - 1) * 4));
        }
        shifted_blocks[BLOCKS_PER_PATTERN + BLOCKS_PER_EXEC - 1] >>= 4;
#pragma unroll
        for (size_t o = 0; o < BLOCKS_PER_EXEC; o++) {
            // Signed comparison: genome 'N' (bit4=0xF) inflates popcount at
            // pattern-N positions. Negative mm still counts as "at most
            // max_mismatches" and must pass the kernel filter; the host
            // recomputes the real cas-offinder mismatch count.
            int mismatches = (int)PATTERN_LEN - (int)counts[o];
            uint32_t loc = (genome_idx + o) * BLOCKS_AVAIL + k;
            if (mismatches <= (int)max_mismatches && loc >= match_start_min_nucl) {
                int next_idx = atomicAdd(entrycount, 1);
                // Bounds guard: keep counting past the buffer (the host reads
                // the final count and bisects the exec range on overflow) but
                // never write out of bounds.
                if (next_idx < (int)OUT_BUF_SIZE) {
                    uint32_t mm_u = (mismatches < 0) ? 0u : (uint32_t)mismatches;
                    s_match next_item;
                    next_item.loc = loc;
                    next_item.pattern_idx = (uint32_t)pattern_block_idx;
                    next_item.mismatches = mm_u;
                    next_item.dna_bulge_size = 0;
                    next_item.rna_bulge_size = 0;
                    match_buffer[next_idx] = next_item;
                }
            }
        }
    }
}
