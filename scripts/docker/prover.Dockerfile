# Multi-stage Dockerfile for tesseract-prover.
#
# Builds from source in a hermetic container — no dependency on the host's
# toolchain / pre-built binary / pre-downloaded artifacts. The only host
# requirement at build time is an SSH agent with access to the
# polytope-labs/sp1-beefy private repo.
#
# Build:
#   docker build \
#     --ssh default \
#     -f scripts/docker/prover.Dockerfile \
#     --build-arg HYPERBRIDGE_REF=main \
#     -t polytopelabs/prover:latest .
#
# Run (requires nvidia-container-toolkit on host + NVIDIA driver that can
# drive your GPU; e.g. 580+ for Blackwell RTX 50xx):
#   docker run --rm --gpus all --network host \
#     -v ./config.toml:/app/config.toml:ro \
#     -v sp1-cache:/root/.sp1 \
#     polytopelabs/prover:latest
#
# The /root/.sp1 volume caches the ~236 MB sp1-gpu-server binary and
# ~8 GB of Groth16 circuit artifacts that sp1-sdk downloads on first run.

# syntax=docker/dockerfile:1.7

############################################################
# Stage 1 — build environment
############################################################
FROM nvidia/cuda:12.8.0-devel-ubuntu24.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive

# Drop NVIDIA cuda apt repo (we already have CUDA libs in the base image;
# the cuda repo is often slow from outside US-west and we don't need it).
RUN rm -f /etc/apt/sources.list.d/cuda*.list /etc/apt/sources.list.d/nvidia-*.list \
 && apt-get update -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30 \
 && apt-get install -y --no-install-recommends \
        build-essential pkg-config libssl-dev protobuf-compiler libprotobuf-dev clang cmake \
        git ca-certificates curl openssh-client \
        golang-go \
 && rm -rf /var/lib/apt/lists/*

# Rust toolchain (1.91 matches rust-toolchain.toml in hyperbridge).
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/root/.foundry/bin:/usr/local/cargo/bin:/usr/local/go/bin:${PATH}
RUN curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain 1.91.0 --profile minimal

# Node.js 20 + pnpm (needed for @openzeppelin/contracts npm deps that the
# EVM solidity sources import). Foundry (forge) for the solidity build.
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
 && apt-get update && apt-get install -y --no-install-recommends nodejs \
 && rm -rf /var/lib/apt/lists/* \
 && npm install -g pnpm@10
RUN curl -L https://foundry.paradigm.xyz | bash \
 && /root/.foundry/bin/foundryup

# GitHub SSH known hosts for the SSH mount below.
RUN mkdir -p -m 700 /root/.ssh \
 && ssh-keyscan -t ed25519,rsa github.com >> /root/.ssh/known_hosts

ARG HYPERBRIDGE_REF=main

WORKDIR /build
RUN --mount=type=ssh \
    git clone --filter=blob:none --no-checkout git@github.com:polytope-labs/hyperbridge.git . \
 && git checkout ${HYPERBRIDGE_REF} \
 && git submodule update --init --recursive \
        evm/lib/solidity-stringutils \
        evm/lib/sp1-contracts

# Build EVM ABIs (emit out/*.sol/*.json that ismp-solidity-abi includes).
RUN cd evm && pnpm install --prefer-offline --reporter=silent && forge build

# Build tesseract-prover with default feature set (includes SP1 local CUDA).
# BuildKit caches for cargo registry + target dir to avoid redownloading
# crates + allow incremental rebuilds. We cp the final binary out of the
# cached target/ to a stable path inside the image layer.
RUN --mount=type=ssh \
    --mount=type=cache,target=/usr/local/cargo/registry,id=beefy-cargo-registry \
    --mount=type=cache,target=/usr/local/cargo/git,id=beefy-cargo-git \
    --mount=type=cache,target=/build/target,id=beefy-cargo-target \
    cargo build --release \
        --manifest-path tesseract/prover/Cargo.toml \
        --bin tesseract-prover \
 && cp /build/target/release/tesseract-prover /usr/local/bin/tesseract-prover \
 && strip /usr/local/bin/tesseract-prover

############################################################
# Stage 2 — runtime image
############################################################
# -base variant gives us libcudart.so.12 without the cuBLAS/cuFFT/cuDNN bulk
# (~200MB vs ~2.5GB for -runtime). sp1-gpu-server only links against libcudart.
FROM nvidia/cuda:12.8.0-base-ubuntu24.04

RUN rm -f /etc/apt/sources.list.d/cuda*.list /etc/apt/sources.list.d/nvidia-*.list \
 && apt-get update -o Acquire::http::Timeout=30 -o Acquire::https::Timeout=30 \
 && apt-get install -y --no-install-recommends ca-certificates libssl3 \
 && update-ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/bin/tesseract-prover /app/tesseract-prover

# sp1-sdk downloads sp1-gpu-server (~236 MB) and groth16 circuits (~8 GB)
# to $HOME/.sp1 on first run. Mount a named volume here to persist across
# container recreates.
VOLUME ["/root/.sp1"]

WORKDIR /app
ENTRYPOINT ["/app/tesseract-prover"]
CMD ["--config", "/app/config.toml"]
