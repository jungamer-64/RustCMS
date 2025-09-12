#!/usr/bin/env bash
set -euo pipefail
# 厳密 clippy 実行
cargo clippy --workspace --message-format=json --all-targets --all-features -- \
  -W clippy::all \
  -W clippy::pedantic \
  -W clippy::nursery \
  -W clippy::cargo \
  -W clippy::correctness \
  -W clippy::complexity \
  -W clippy::perf \
  -W clippy::suspicious \
  -D warnings
