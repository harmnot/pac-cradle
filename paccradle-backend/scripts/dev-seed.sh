#!/usr/bin/env bash
# Development seed data (examples/dev_seed.rs), run in the toolchain image like every API build.
# The same four accounts every time, with one password; the list and what each one has is in
# .claude/skills/dev-seed/SKILL.md at the repo root.
#
#   scripts/dev-seed.sh               # reset the four accounts (needs scripts/dev-db.sh)
#   scripts/dev-seed.sh admin-token   # sign the admin in to the back office, print the tokens
#
# It refuses to run unless APP_ENV=development and the database is on this machine. Settings
# come from .env, as for scripts/dev-api.sh; ADMIN_WEB_URL defaults to http://localhost:3001.
set -euo pipefail

backend_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec "${backend_dir}/scripts/in-docker.sh" cargo run --quiet --example dev_seed -- "$@"
