# Bulge Support Design for Cas-OFFinder-rust

## Background

`cas-offinder-rust` currently uses a brute-force search based on bit-packed popcount. Within a fixed-length window it bitwise-ANDs the pattern and the genome and counts mismatches via popcount, which means it **cannot find indels (bulges)**. In CRISPR off-target analysis DNA/RNA bulges are biologically meaningful off-target forms, so bulge search is needed.

The existing `cas-offinder-bulge` is a Python wrapper that mutates patterns (by inserting Ns) and feeds them through the original cas-offinder. That approach inflates the pattern count by tens of times and is slow.

## Goals

1. Add **DNA bulge + RNA bulge search** to Cas-OFFinder-rust
2. Improve speed over `cas-offinder-bulge`
3. Keep the result format compatible with `cas-offinder-bulge`

## Chosen Algorithm

**PAM scan + Myers bit-parallel edit distance** (2-phase)

### Why this combination

- **PAM correctness guarantee**: PAMs like NGG are essential for Cas9 binding and must be exact matches. Bulges must not enter the PAM region.
- **Myers bit-parallel**: a fast algorithm that computes mismatch + indel as a single edit distance. It fits naturally with our existing bit-packed encoding.
- **Faster than cas-offinder-bulge**: no pattern expansion needed, so the workload drops sharply.

### Alternatives considered

| Approach | Bulge | Speed | Notes |
|---|---|---|---|
| Current popcount | ✗ | very fast | feature-incomplete |
| Full Smith-Waterman | ✓ | slow | O(m²) per site; local alignment is unsuitable for CRISPR |
| cas-offinder-bulge | ✓ | slow | pattern expanded ~56× |
| **PAM scan + Myers** | ✓ | fast | adopted |
| Unified Myers (PAM included) | ✓ | slightly faster | **allows bulges inside the PAM region, breaking biological correctness** → rejected |

## Algorithm Details

### Phase 1: PAM scan

Stream the genome and at each position `j` check:
- Whether `genome[j:j+pam_len]` is an exact match to the PAM pattern (e.g. NGG)
- N matches any nucleotide
- All other positions must match exactly

Implementation: bitwise AND in the 4-bit encoding to check 3 characters (O(1)).

NGG positions occur on average at 1/16 of positions. For the human genome (3×10⁹ bp), this gives roughly 400M PAM sites.

### Phase 2: Myers bit-parallel edit distance

For each PAM site `p`, compare the guide RNA (length `m`) against the upstream genome segment with the Myers algorithm.

**Comparison targets**:
- Pattern (fixed): the gRNA sequence (e.g. 20 nt)
- Text: `genome[p - m - max_bulges : p]` (can grow by up to `max_bulges` due to a DNA bulge)

**Myers steps**:
1. Precompute `Peq[σ]` for each σ ∈ {A, C, G, T} (per-character bit-vectors of the pattern)
2. Initialize VP = all 1s, VN = 0
3. For each text character, update VP and VN with 7 bit operations
4. Track the edit distance from HP and HN of the last row
5. Record positions where `edit_distance ≤ max_edits` as matches

**Edit distance threshold**:
```
max_edits = max_mismatches + max_dna_bulges + max_rna_bulges
```

Myers itself treats substitution/insertion/deletion as a single edit, so it does not distinguish DNA from RNA bulges. We first collect all candidates with edit distance ≤ `max_edits`, then in a second-pass traceback split them into DNA bulge / RNA bulge / mismatch. If the traceback exceeds any individual threshold (`max_mismatches`, `max_dna_bulges`, `max_rna_bulges`), drop the match.

### Bulge type classification (traceback)

After Myers identifies candidate positions, run traceback **only at those positions** to recover the alignment:
- Substitution → mismatch
- Gap in genome → RNA bulge (extra on the pattern side)
- Gap in pattern → DNA bulge (extra on the genome side)

Traceback is cheap because the candidate count is small (a tiny fraction of all NGG sites).

## I/O Format

### Input format (cas-offinder-bulge compatible)

```
/path/to/genome
NNNNNNNNNNNNNNNNNNNNNNNGG 2 1
GGCCGACCTGTCGCTGACGC 5
CGCCAGCGTCAGCGACAGGT 5
...
```

- Line 1: genome path (file or directory, same as before)
- Line 2: `search_filter` + `max_dna_bulges` + `max_rna_bulges`
  - `search_filter` specifies the PAM position (same as the existing cas-offinder)
  - If bulge parameters are absent, both default to 0 (compatible with the existing cas-offinder-rust)
- Line 3 onward: `pattern` + `max_mismatches`

### Output format (cas-offinder-bulge compatible)

```
#Bulge type	crRNA	DNA	Chromosome	Position	Direction	Mismatches	Bulge Size
X	GGCCGACCTGTCGCTGACGCNGG	GGCCGACCTGTCGCTGACGCAGG	chr1	1234567	+	0	0
DNA	GGCCGACCTGTCGC-TGACGCNGG	GGCCGACCTGTCGCATGACGCAGG	chr1	2345678	+	2	1
RNA	GGCCGACCTGTCGCTGACGCNGG	GGCCGACCTGTCGCGACGCAGG	chr1	3456789	-	1	1
...
```

- **Bulge type**: `X` (no bulge), `DNA`, `RNA`
- **crRNA**: gRNA sequence (gaps shown as `-` when bulges are present)
- **DNA**: matched genome sequence (gaps shown as `-` when bulges are present)
- **Chromosome, Position, Direction**: same as before
- **Mismatches**: substitution count (excludes bulges)
- **Bulge Size**: bulge size

### Log file (new)

A `<output_filename>.log` file is **always** generated automatically (in the same path as the output file). The log file is written exactly once, just before the main thread exits, so its impact on runtime is negligible (`<0.01s`).

```
=== Cas-OFFinder Rust (Myers bit-parallel) ===
Run date: 2026-04-16 11:45:23

Device: GPU (NVIDIA GeForce RTX 5090)
  (or: CPU (32 threads, native Rust))

Input:
  Genome: /path/to/genome.fa
  Genome size: 3,098,825,702 bp
  Patterns: 4
  Search filter: NNNNNNNNNNNNNNNNNNNNNNNGG
  Max mismatches: 5
  Max DNA bulges: 2
  Max RNA bulges: 1

Phase 1 (PAM scan):
  NGG sites found: 387,456,123
  Elapsed: 2.341s

Phase 2 (Myers edit distance):
  Candidates checked: 387,456,123
  Matches found: 12,345
  Elapsed: 8.765s

Total elapsed: 11.107s
```

## Architecture

### Files affected

**cas-offinder-lib**:
- `src/search.rs` — main search logic; replace popcount with Myers
- `src/kernel.cl` — GPU kernel; rewritten in Myers bit-parallel
- `src/bit4ops.rs` — add a Peq-table builder for Myers
- New: `src/myers.rs` — Myers algorithm implementation (CPU)
- New: `src/traceback.rs` — alignment recovery from candidate positions
- New: `src/pam_scan.rs` — Phase 1 PAM scan
- New: `src/log_writer.rs` — log file generation

**cas-offinder-cli**:
- `src/cli_utils.rs` — input parser; parse the bulge parameters on line 2
- `src/main.rs` — output format change (cas-offinder-bulge compatible) and log file generation

### Data flow

```
[Genome stream (chunked)]
          ↓
  [read_fasta/read_2bit]       (existing code reused)
          ↓
  [ChromChunkInfo channel]      (existing code reused)
          ↓
  [chunks_to_searchchunk]       (existing code reused)
          ↓
  [SearchChunkInfo channel]
          ↓
  ┌───────────────────────────────────────┐
  │  Phase 1: PAM Scan                    │
  │  - check search_filter at each pos    │
  │  - emit list of NGG positions         │
  └───────────────────────────────────────┘
          ↓
  [PamHitList (position + direction)]
          ↓
  ┌───────────────────────────────────────┐
  │  Phase 2: Myers per PAM site          │
  │  - edit distance vs. gRNA at each NGG │
  │  - collect edit_distance ≤ threshold  │
  │  - both forward and reverse           │
  └───────────────────────────────────────┘
          ↓
  [MyersMatch (position, pattern_idx, edit_dist, traceback)]
          ↓
  [Traceback (recover bulge type/position)]
          ↓
  [Match struct (existing fields + bulge fields)]
          ↓
  [Output writer (cas-offinder-bulge format)]
          ↓
  [output.txt] + [output.txt.log]
```

### GPU kernel structure

**Before**: the `find_matches` kernel computes mismatches via popcount.
**After**: the `find_matches_myers` kernel runs Phase 1 (PAM) + Phase 2 (Myers) **fused together**.

Conceptually 2-phase, but in the GPU kernel a single work-item handles both steps sequentially to avoid an intermediate buffer:

- Each work-item processes one genome position
- Within the work-item:
  1. Check PAM (3-nt) for an exact match → early return on failure
  2. Run Myers bit-parallel to compute edit distance against the gRNA
  3. If `edit_distance ≤ max_edits`, record the result via atomic_inc
- 2D work-group: (genome positions) × (patterns)

Traceback is implemented by post-processing the GPU result buffer on the CPU (the candidate count is small enough that the CPU is sufficient).

**Memory**:
- `Peq[4]` (per-{A, C, G, T} pattern bit-vector) — constant memory
- Text window: `genome[p - m - max_bulges : p]` — local memory or direct load

## Test Strategy

### Unit tests

1. **Myers algorithm correctness**: verify against standard DP results across edit distance cases
   - Substitution only (mismatch)
   - DNA bulge 1, 2
   - RNA bulge 1, 2
   - Mixed (mismatch + bulge)
   - Edge cases (full pattern match, full mismatch, empty pattern)

2. **PAM scan correctness**: verify against a known list of NGG positions

3. **Traceback correctness**: verify recovered alignment matches the actual edit sequence

### Integration tests

1. **Compare against cas-offinder-bulge**: for the same input, our results must match `cas-offinder-bulge` (regression test)
   - `upstream1000.fa` (small test data)
   - Multiple patterns
   - Default config: bulge_dna=2, bulge_rna=1, mismatches=5

2. **cas-offinder-rust regression**: with bulge=0, results must match the existing implementation

3. **CPU vs GPU consistency**: for the same input, CPU/GPU outputs must be byte-identical

### Performance benchmarks

1. **Baseline**: current popcount (no bulge)
2. **After change**: Myers (bulge=0, bulge=1, bulge=2 each)
3. **cas-offinder-bulge**: comparison under the same configuration

Datasets:
- `upstream1000.fa` (small, ~13 KB)
- A slice of the human genome (around chr22, ~50 MB)
- Full human genome where possible

## Open Issues / Risks

1. **Pattern length constraint**: Myers is fastest when the pattern fits in a 64-bit word. CRISPR gRNAs are typically 20–24 nt so this is fine, but supporting longer patterns would require multi-word Myers.

2. **Memory**: a large number of PAM candidates can appear on the GPU (~400M for the human genome). Output buffer sizing may need tuning.

3. **False positives with many bulges**: large `max_bulges` lengthens the text window and increases the candidate count. `OUT_BUF_SIZE` is currently `1<<22` and may need to be tuned.

4. **Reverse strand handling**: the current code searches both the pattern and its reverse complement. Myers handles them the same way.

5. **Traceback performance**: with many candidates, traceback could become a bottleneck. We need to decide whether to implement traceback on the GPU or the CPU. (Current plan: CPU traceback. Even with many candidates, it should fit in a single processing window.)

## Success Criteria

1. **Functional**: produces match results identical to cas-offinder-bulge (regression test passes)
2. **Speed**: with `bulge_dna=2, bulge_rna=1`, at least **5× faster** than cas-offinder-bulge
3. **Backwards compatibility**: with bulge=0, output identical to the existing cas-offinder-rust
4. **CPU/GPU consistency**: identical output between CPU and GPU

## Future Work

- Longer pattern support (multi-word Myers)
- Other PAMs (NAG, NGAN, etc.)
- GPU-side traceback parallelization
- FM-index-based additional acceleration (with preprocessing cost considered)
