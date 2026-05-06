# Installation Guide (Linux x86_64)

## Requirements

| Component | Version | Notes |
|---|---|---|
| Linux x86_64 | glibc 2.17+ | Most distros from 2014+ |
| NVIDIA GPU | Compute capability 5.0+ | RTX 4090 / 5090 tested; older cards probably also work |
| NVIDIA driver | 525+ (for CUDA 12.x) | Check with `nvidia-smi` |
| CUDA Toolkit | 12.8 (or compatible) | For `libnvrtc.so` (runtime kernel JIT) |

## Step 1: NVIDIA driver

Most servers/workstations already have this. Verify:

```bash
nvidia-smi
```

If you see your GPU listed, the driver is fine. Otherwise install from your distro's package manager (`nvidia-driver-525` or newer) or from https://www.nvidia.com/drivers.

## Step 2: CUDA Toolkit (just `libnvrtc`)

The binary uses dynamic loading, so you only need the runtime libraries — not the full nvcc compiler.

### Option A: Conda / Mamba (easiest, recommended)

```bash
# install micromamba/mamba/conda first if you don't have it
micromamba install -c nvidia cuda-toolkit=12.8

# Find the prefix where it installed
echo $MAMBA_ROOT_PREFIX  # or wherever your env lives
```

### Option B: Official installer

Download from https://developer.nvidia.com/cuda-12-8-0-download-archive and follow the runfile installer.

### Option C: Distro package

```bash
# Ubuntu / Debian
sudo apt install cuda-nvrtc-12-8 cuda-cudart-12-8

# Fedora / RHEL
sudo dnf install cuda-nvrtc-12-8 cuda-cudart-12-8
```

## Step 3: Set environment variables

The binary needs `libnvrtc.so` and `libcuda.so.1` on the runtime library path.

```bash
# Add these to your ~/.bashrc, ~/.zshrc, or run before each session:
export CUDA_PATH=/path/to/cuda                      # e.g., /usr/local/cuda or your micromamba env
export LD_LIBRARY_PATH=$CUDA_PATH/lib:$LD_LIBRARY_PATH
# (some installers use $CUDA_PATH/lib64 instead — adjust if needed)
```

For micromamba, `$CUDA_PATH` is typically your env prefix:

```bash
export CUDA_PATH=$(dirname $(dirname $(which nvcc 2>/dev/null)))    # if nvcc is installed
# or simply:
export CUDA_PATH=$HOME/micromamba/envs/your_env_name
export LD_LIBRARY_PATH=$CUDA_PATH/lib:$LD_LIBRARY_PATH
```

## Step 4: Test

```bash
./cas-offinder-cli examples/ngg_example.in G test_output.txt
```

You should see something like:

```
Total 1 device(s) found.
Loading input file...
Reading /path/to/genome.fa...
...
Completed in 8.5s
```

And three output files: `test_output.txt`, `test_output_summary.txt`, `test_output_log.txt`.

## Troubleshooting

**`error while loading shared libraries: libnvrtc.so.12`**
→ `LD_LIBRARY_PATH` doesn't include the directory containing `libnvrtc.so`.
Find it with: `find / -name 'libnvrtc.so*' 2>/dev/null` and add that directory.

**`Total 0 device(s) found.`**
→ The CUDA driver isn't loaded, or the GPU isn't visible to this process. Try `nvidia-smi` to verify the driver works, and check `CUDA_VISIBLE_DEVICES` isn't set to an empty value.

**CPU-only fallback**
→ Use `C` instead of `G`: `./cas-offinder-cli input.in C output.txt`. Slower but doesn't need a GPU.

## Optional: install systemwide

```bash
sudo cp cas-offinder-cli /usr/local/bin/
sudo chmod +x /usr/local/bin/cas-offinder-cli
```

Then run from anywhere with just `cas-offinder-cli ...`.
