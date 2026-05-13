# cas-offinder-rust v1.0.0 — Linux x86_64 + CUDA 12.8
#
# Base: Ubuntu 22.04 (glibc 2.35, satisfies binary's GLIBC_2.34 requirement).
# Layer 1: CUDA 12.8 runtime (libcudart, libcublas etc.)
# Layer 2: cuda-nvrtc-12-8 (kernel JIT compilation at startup)
# Layer 3: pre-built cas-offinder-cli binary
#
# Final image size: ~1.7 GB (vs ~5 GB if using devel base).
FROM nvidia/cuda:12.8.0-runtime-ubuntu22.04

# Install nvrtc only (kernel JIT). Other CUDA dev tooling not needed since the
# binary is pre-built. Clean apt caches afterwards to keep the image lean.
RUN apt-get update && apt-get install -y --no-install-recommends \
        cuda-nvrtc-12-8 \
    && rm -rf /var/lib/apt/lists/*

# Install the pre-built binary.
WORKDIR /app
COPY cas-offinder-cli /app/cas-offinder-cli
RUN chmod +x /app/cas-offinder-cli

# Make CUDA libraries discoverable to the dynamic loader (for libnvrtc).
ENV LD_LIBRARY_PATH=/usr/local/cuda/lib64:${LD_LIBRARY_PATH}

# Default working directory is /workspace — users mount their input/output
# files here:   docker run -v $(pwd):/workspace ...
WORKDIR /workspace

ENTRYPOINT ["/app/cas-offinder-cli"]
