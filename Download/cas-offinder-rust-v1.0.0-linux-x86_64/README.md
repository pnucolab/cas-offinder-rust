# cas-offinder-rust v1.0.0 (Linux x86_64)

A high-performance Rust + CUDA reimplementation of [cas-offinder](https://github.com/snugel/cas-offinder), the CRISPR off-target search tool.

## What's in this package

```
cas-offinder-cli      The executable (~1.3 MB)
README.md             This file
INSTALL.md            Setup instructions for CUDA dependencies
LICENSE
examples/             Example input files
```

## Quick start

```bash
# 1. Set up CUDA paths (one-time, see INSTALL.md for details)
export CUDA_PATH=/path/to/cuda
export LD_LIBRARY_PATH=$CUDA_PATH/lib:$LD_LIBRARY_PATH

# 2. Run a search
./cas-offinder-cli examples/ngg_example.in G output.txt
```

This produces three files:

- `output.txt` — per-match results (TSV, one row per off-target hit)
- `output_summary.txt` — match counts grouped by `(mismatches, dna_bulges, rna_bulges)`
- `output_log.txt` — run metadata (genome, parameters, runtime)

## Usage

```
cas-offinder-cli {input_file} {C|G|A}[device_id] {output_file}
```

- `C` — use CPUs
- `G` — use GPUs (default for performance)
- `A` — use accelerators
- `output_file = -` writes matches to stdout (no summary/log file)

## Input format

```
/path/to/genome.fa                              <- 1st line: FASTA path (or 2bit, or directory)
NNNNNNNNNNNNNNNNNNNNNNNNNGG 1 1                 <- 2nd line: PAM mask + DNA bulge + RNA bulge
CCGTGGTTCAACATTTGCTTAGCANNN 5 my_TP53_g1        <- 3rd line+: guide + max mismatches + (optional) label
GATGTTGGTAAGTGGGATATGGCANNN 5 my_BRCA1_g2
```

- **Line 1**: path to a single FASTA / 2bit file, or a directory of FASTA files
- **Line 2**: `<PAM_mask> <DNA_bulges> <RNA_bulges>` — IUPAC codes accepted (R Y S W K M B D H V N + ACGT)
- **Lines 3+**: `<guide_pattern> <max_mismatches> [<label>]` — label is optional; if omitted, an auto-numeric ID (0, 1, 2, ...) is used

The PAM mask uses N for variable PAM positions and specific bases (or IUPAC codes) for required PAM positions. Each guide must be the same length as the mask, with N at the PAM region.

## Output format

```
#Id      Bulge type   crRNA                          DNA                            Chromosome  Position   Direction  Mismatches  Bulge Size
my_TP53  X            CCGTGGTTCAACATTTGCTTAGCANNN    CCGTGGTTCAACATTTGCTTAGCAGGG    chr1 1     1234567    +          0           0
my_TP53  DNA          CC-GTGGTTCAACATTTGCTTAGCANNN   CCAGTGGTTCAACATTTGCTTAGCAGGG   chr2 2     8765432    -          1           1
...
```

## Supported PAMs

Any PAM expressible with IUPAC codes:

- **SpCas9 (NGG)**: `NGG`
- **SaCas9 (NNGRRT)**: `NNGRRT`
- **Cpf1 / Cas12a (TTTV)**: `TTTV` (PAM-first)
- **Custom variants** (verified working): `NRTA`, `NNGTGA`, `NGT`, `NNNNCWAA`, `NNAGAAW`, `NNNVRYAC`, `TTTR`, `NGNG`, etc.

PAM-first (5') and PAM-last (3') both supported.

## Bulges

DNA bulges (gaps in the genome side) and RNA bulges (gaps in the guide side) up to the user-specified maximum each. The 2nd line of the input file controls these:

```
NNNNNNNNNNNNNNNNNNNNNNNNNGG 2 1   <- up to 2 DNA bulges + 1 RNA bulge
```

When `db = rb = 0`, the fast popcount path is used; otherwise the Myers bit-parallel + DP traceback path runs.

## Verified against C++ cas-offinder

For all PAM × bulge configurations tested (NGG, NGT, NNGTGA, NNNVRYAC, TTTN, TTTR, NRTA, NNNNCWAA, NNAGAAW, NGNG), Rust produces hit counts identical to the C++ reference. Runtime is typically several × faster.

## Source

https://github.com/pnucolab/cas-offinder-rust
