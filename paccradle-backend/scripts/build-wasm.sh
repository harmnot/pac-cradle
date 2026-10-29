#!/usr/bin/env bash
# Builds the insert_core preview for the browser (M1-A10) into the frontend, where the
# editor imports it by local path (DEC-03, nothing is published).
#
#   scripts/build-wasm.sh            # from paccradle-backend/, or `npm run build:wasm` in the frontend
#
# Needs the wasm32 target and the wasm-bindgen CLI at the version in Cargo.lock:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <version> --locked
# Output: paccradle-frontend-app/wasm/insert-core/ (generated, not committed).
set -euo pipefail

backend_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="${backend_dir}/../paccradle-frontend-app/wasm/insert-core"
target_dir="${CARGO_TARGET_DIR:-${backend_dir}/target}"

# The CLI writes the JS glue for one exact wasm-bindgen version: check it matches.
wanted="$(grep -A1 '^name = "wasm-bindgen"$' "${backend_dir}/Cargo.lock" | sed -n 's/^version = "\(.*\)"$/\1/p')"
found="$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)"
if [[ "${found}" != "${wanted}" ]]; then
  echo "wasm-bindgen CLI ${found:-not found}; this build needs ${wanted}:" >&2
  echo "  cargo install wasm-bindgen-cli --version ${wanted} --locked" >&2
  exit 1
fi

cargo build --manifest-path "${backend_dir}/Cargo.toml" -p insert_core_wasm \
  --target wasm32-unknown-unknown --profile wasm-release
# Without the name and producers sections the file is about half the size.
wasm-bindgen --target web --out-name insert_core --out-dir "${out_dir}" \
  --remove-name-section --remove-producers-section \
  "${target_dir}/wasm32-unknown-unknown/wasm-release/insert_core_wasm.wasm"

wasm="${out_dir}/insert_core_bg.wasm"
echo "Built ${wasm}: $(wc -c < "${wasm}" | tr -d ' ') bytes, $(gzip -9 -c "${wasm}" | wc -c | tr -d ' ') gzipped"
