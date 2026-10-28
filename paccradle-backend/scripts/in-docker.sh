#!/usr/bin/env bash
# Runs a command inside the Rust + OpenCV toolchain image (Dockerfile, stage "toolchain").
# Use it while OpenCV isn't installed on the Mac, or to check the Linux build.
#
#   scripts/in-docker.sh cargo test --workspace
#   scripts/in-docker.sh cargo run -p vision --bin measure -- crates/vision/spike/photos/jar.jpg --paper a4
#
# Build the image first: docker build --target toolchain -t paccradle-toolchain .
# Build output lives in the "paccradle-target" Docker volume (fast; macOS bind mounts are slow).
set -euo pipefail

backend_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

exec docker run --rm \
  -v "${backend_dir}:/app" \
  -v paccradle-cargo-registry:/usr/local/cargo/registry \
  -v paccradle-target:/target \
  -e CARGO_TARGET_DIR=/target \
  -w /app \
  paccradle-toolchain "$@"
