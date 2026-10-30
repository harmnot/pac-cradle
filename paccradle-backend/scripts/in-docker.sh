#!/usr/bin/env bash
# Runs a command inside the Rust + OpenCV toolchain image (Dockerfile, stage "toolchain").
# The API links OpenCV since M1-D5, so on a Mac without OpenCV every API build, test and
# `cargo sqlx prepare` runs here:
#
#   scripts/in-docker.sh cargo test --workspace
#   scripts/in-docker.sh cargo sqlx prepare -- --all-targets
#   scripts/in-docker.sh cargo run -p vision --bin measure -- crates/vision/spike/photos/jar.jpg --paper a4
#
# Build the image first: docker build --target toolchain -t paccradle-toolchain .
# Build output lives in the "paccradle-target" Docker volume (fast; macOS bind mounts are slow).
# DATABASE_URL points at scripts/dev-db.sh's database through the host (set DOCKER_DATABASE_URL
# to use another); SQLX_OFFLINE passes through when set.
set -euo pipefail

backend_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
database_url="${DOCKER_DATABASE_URL:-postgres://paccradle:paccradle@host.docker.internal:54329/paccradle}"

exec docker run --rm \
  -v "${backend_dir}:/app" \
  -v paccradle-cargo-registry:/usr/local/cargo/registry \
  -v paccradle-target:/target \
  -e CARGO_TARGET_DIR=/target \
  -e DATABASE_URL="${database_url}" \
  ${SQLX_OFFLINE:+-e SQLX_OFFLINE="${SQLX_OFFLINE}"} \
  -w /app \
  paccradle-toolchain "$@"
