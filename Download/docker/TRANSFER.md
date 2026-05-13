# cas-offinder-rust 1.0.0 — Docker Image Transfer Guide

## Package Contents

```
docker/
├── Dockerfile                              # Build recipe (for reference)
├── cas-offinder-cli                        # Built binary (1.3 MB)
├── cas-offinder-rust-1.0.0-image.tar.gz    # Docker image (2.1 GB) ← this is what you transfer
└── TRANSFER.md                             # This document
```

## Image Details

- Base: `nvidia/cuda:12.8.0-runtime-ubuntu22.04`
- Extra packages: `cuda-nvrtc-12-8` (for kernel JIT compilation)
- Install location: `/app/cas-offinder-cli`
- Working directory: `/workspace` (user mounts a host directory here)

## Build Machine → Target Machine Transfer

### 1. Transfer the file

```bash
# Option A: scp
scp cas-offinder-rust-1.0.0-image.tar.gz user@target-server:~/

# Option B: rsync (resumable, recommended for large files)
rsync -avP --partial cas-offinder-rust-1.0.0-image.tar.gz user@target-server:~/

# Option C: USB / external disk
cp cas-offinder-rust-1.0.0-image.tar.gz /mnt/usb/
```

### 2. Target machine prerequisites

```bash
# 2-1. Verify NVIDIA driver (525+)
nvidia-smi

# 2-2. Verify Docker is installed
docker --version

# 2-3. Install NVIDIA Container Toolkit (one-time setup)
# Ubuntu/Debian:
distribution=$(. /etc/os-release; echo $ID$VERSION_ID)
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey \
    | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
curl -s -L https://nvidia.github.io/libnvidia-container/$distribution/libnvidia-container.list \
    | sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' \
    | sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list
sudo apt-get update
sudo apt-get install -y nvidia-container-toolkit
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker

# Verify:
docker run --rm --gpus all nvidia/cuda:12.8.0-base-ubuntu22.04 nvidia-smi
```

### 3. Load the image

```bash
# Decompress + register with Docker (single step)
gunzip -c cas-offinder-rust-1.0.0-image.tar.gz | docker load

# Verify
docker images cas-offinder-rust
# REPOSITORY          TAG     IMAGE ID       CREATED          SIZE
# cas-offinder-rust   1.0.0   <hash>         X minutes ago    3.65GB
```

## Usage

### Basic invocation

```bash
# Run from the directory containing your input file and genome
cd /path/to/your/workdir

docker run --rm \
    --gpus all \
    -v /path/to/genome_dir:/genome:ro \
    -v $(pwd):/workspace \
    cas-offinder-rust:1.0.0 \
    input.in G output.txt
```

Explanation:
- `--rm`: remove the container after it exits
- `--gpus all`: use all GPUs (or pin to one with `--gpus '"device=0"'`)
- `-v /path/to/genome_dir:/genome:ro`: mount the genome FASTA directory at `/genome` inside the container, read-only
- `-v $(pwd):/workspace`: mount the current directory (where input/output files live) at `/workspace`
- The first line of the input file must use the in-container path, e.g. `/genome/GRCh38.primary_assembly.genome.fa`

### Example input file

```
/genome/GRCh38.primary_assembly.genome.fa
NNNNNNNNNNNNNNNNNNNNNNNNNGG 1 1
CCGTGGTTCAACATTTGCTTAGCANNN 5 TP53_g1
GATGTTGGTAAGTGGGATATGGCANNN 5 BRCA1_g2
```

### Output files

When `docker run` finishes, three files are produced in `$(pwd)`:
- `output.txt` — the match list (TSV)
- `output_summary.txt` — per-(mm, db, rb) statistics
- `output_log.txt` — run metadata

### Simplify with a shell alias (optional)

Add to `~/.bashrc` or `~/.zshrc`:

```bash
casof() {
    docker run --rm --gpus all \
        -v "$(pwd)":/workspace \
        cas-offinder-rust:1.0.0 "$@"
}

# If you also want to auto-mount the genome directory:
casof() {
    docker run --rm --gpus all \
        -v /path/to/genome_dir:/genome:ro \
        -v "$(pwd)":/workspace \
        cas-offinder-rust:1.0.0 "$@"
}
```

Usage:
```bash
casof input.in G output.txt
```

## Common Errors

| Error | Cause | Fix |
|---|---|---|
| `could not select device driver "" with capabilities: [[gpu]]` | NVIDIA Container Toolkit not installed | Run STEP 2-3 above |
| `Unable to dynamically load the "cuda" shared library` | Missing `--gpus all` | Add `--gpus all` to the docker run command |
| `error while loading shared libraries: libnvrtc.so.12` | Image build issue | Rebuild on the build machine (check the Dockerfile) |
| Input file not found | Confused host path with container path | Use the in-container path on line 1 of the input file (`/genome/...`) |

## Upgrading the Image

When a new version arrives:

```bash
# Remove the old image (optional)
docker rmi cas-offinder-rust:1.0.0

# Load the new image
gunzip -c cas-offinder-rust-1.0.1-image.tar.gz | docker load
```

If the tags differ, the old and new images can coexist (`1.0.0`, `1.0.1`, `latest`, etc.).
