#!/usr/bin/env bash
set -euo pipefail
echo "[verify] default feature set (includes auth-flat-fields)"
cargo test --quiet
echo "[verify] minimal unified auth (no flat fields)"
cargo test --no-default-features --features auth,cache,compression,database,search --quiet
echo "[verify] done"
