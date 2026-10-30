#!/usr/bin/env bash
# Runs the API on this Mac inside the toolchain image: it links OpenCV since M1-D5, which
# the Mac doesn't have. Settings come from .env as usual; these are set for the container:
# it listens on all its interfaces, and reaches scripts/dev-db.sh's database and
# scripts/dev-mail.sh's Mailpit through the host.
#
#   scripts/dev-api.sh                 # http://127.0.0.1:7773, Ctrl-C stops it
#   APP_PORT=7781 scripts/dev-api.sh   # another port on the Mac
#   scripts/dev-api.sh --release       # extra arguments go to cargo run
#
# DEV_SMTP_HOST overrides the SMTP host (default host.docker.internal, for Mailpit). These
# settings, when set in your shell (even empty), override .env too: STORAGE_*, EMAIL_FROM, PADDLE_*,
# RATE_LIMIT_*, JOB_WORKERS, RUST_LOG. E.g. keep photos in memory for one run:
#   STORAGE_ENDPOINT= STORAGE_BUCKET= STORAGE_ACCESS_KEY_ID= STORAGE_SECRET_ACCESS_KEY= scripts/dev-api.sh
set -euo pipefail

backend_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
port="${APP_PORT:-7773}"
tty_flags=()
if [[ -t 0 && -t 1 ]]; then
  tty_flags=(-it)
fi
overrides=()
while IFS= read -r name; do
  overrides+=(-e "${name}=${!name}")
done < <(compgen -e | grep -E '^(STORAGE_[A-Z_]+|EMAIL_FROM|PADDLE_[A-Z_]+|RATE_LIMIT_[A-Z_]+|JOB_WORKERS|RUST_LOG)$' || true)

exec docker run --rm --init ${tty_flags[@]+"${tty_flags[@]}"} \
  -p "127.0.0.1:${port}:7773" \
  -v "${backend_dir}:/app" \
  -v paccradle-cargo-registry:/usr/local/cargo/registry \
  -v paccradle-target:/target \
  -e CARGO_TARGET_DIR=/target \
  -e APP_HOST=0.0.0.0 \
  -e APP_PORT=7773 \
  -e DATABASE_URL=postgres://paccradle:paccradle@host.docker.internal:54329/paccradle \
  -e SMTP_HOST="${DEV_SMTP_HOST:-host.docker.internal}" \
  ${overrides[@]+"${overrides[@]}"} \
  -w /app \
  paccradle-toolchain cargo run -p paccradle-backend "$@"
