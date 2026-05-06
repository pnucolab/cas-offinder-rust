# Examples

Two example input files. Edit the first line in each to point to your local genome FASTA, then run:

```bash
./cas-offinder-cli examples/ngg_example.in G output.txt
```

## ngg_example.in

SpCas9-style search with NGG PAM, no bulges, up to 5 mismatches. Two guides with custom labels.

## tttn_bulge_example.in

Cas12a / Cpf1-style search with TTTN PAM (PAM-first), 1 DNA bulge + 1 RNA bulge allowed, up to 4 mismatches.

## Input format quick reference

```
<line 1: genome FASTA / 2bit / directory>
<line 2: PAM_mask> <DNA_bulges> <RNA_bulges>
<line 3: guide_pattern> <max_mismatches> [<label>]
<line 4+: more guides...>
```

- PAM mask uses `N` for variable positions, IUPAC codes for specific PAM bases
- Each guide must be the same length as the PAM mask
- Use `N` in the guide where the PAM region is (e.g., `...AGCANNN` for NGG)
- Label (3rd column on guide line) is optional; if omitted, IDs `0, 1, 2, ...` are auto-assigned
