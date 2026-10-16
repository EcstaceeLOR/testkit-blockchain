#!/usr/bin/env bash
set -euo pipefail

# Smoke test for CLI installation instructions in a clean temporary environment.
# Validates that `soroban-testkit-cli` builds and installs cleanly into an
# isolated root, runs without error, and exposes all expected subcommands.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

TMP_INSTALL_ROOT="$(mktemp -d)"
trap 'rm -rf "$TMP_INSTALL_ROOT"' EXIT

echo "==> Installing soroban-testkit-cli into temporary root: $TMP_INSTALL_ROOT"
CARGO_INSTALL_ROOT="$TMP_INSTALL_ROOT" cargo install --path "$WORKSPACE_ROOT/crates/cli" --locked --quiet

BIN="$TMP_INSTALL_ROOT/bin/soroban-testkit"

if [[ ! -x "$BIN" ]]; then
  echo "::error::Installed binary not found or not executable at $BIN"
  exit 1
fi

echo "==> Verifying soroban-testkit --help output"
HELP_OUTPUT="$("$BIN" --help)"

for cmd in coverage limits audit; do
  if ! echo "$HELP_OUTPUT" | grep -qw "$cmd"; then
    echo "::error::Missing expected subcommand '$cmd' in CLI help"
    exit 1
  fi
done

echo "==> CLI installation smoke test passed successfully."
