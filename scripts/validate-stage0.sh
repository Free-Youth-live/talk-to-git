#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked -p ttg-core

cargo run --locked -p ttg-fixtures -- --force --commits "${FIXTURE_COMMITS:-50000}"
cargo run --locked -p ttg-core --bin core_probe -- --repo fixtures/generated/standard --status-json >/dev/null
cargo run --locked -p ttg-core --bin core_probe -- --repo fixtures/generated/history --log 50000
cargo run --locked -p ttg-core --bin core_probe -- --repo fixtures/generated/large-diff --diff >/dev/null

staged_before="$(git -C fixtures/generated/standard diff --cached | wc -l)"
cargo run --locked -p ttg-core --bin core_probe -- --repo fixtures/generated/standard --stage-hunk 0 "@@ -1,5 +1,5 @@"
staged_after="$(git -C fixtures/generated/standard diff --cached | wc -l)"
if [[ "$staged_after" -le "$staged_before" ]]; then
  echo "expected the staged Hunk to add to the staged diff" >&2
  exit 1
fi
cargo run --locked -p ttg-fixtures -- --force --commits "${FIXTURE_COMMITS:-50000}"

cargo run --locked -p ttg-gui -- --repo fixtures/generated/standard --ai-status >/dev/null

echo "Stage 0 local validation passed"
