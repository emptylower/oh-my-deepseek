#!/usr/bin/env bash
set -euo pipefail

# Build deepseek-omd from the patched DeepSeek-TUI source
#
# Usage:
#   ./scripts/omd-build.sh [--release]
#
# Output:
#   target/release/deepseek-omd (with --release)
#   target/debug/deepseek-omd (without --release)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TUI_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> Building deepseek-omd from: $TUI_ROOT"
echo "==> Base version: $(cat "$TUI_ROOT/.omd-base-version" 2>/dev/null || echo 'unknown')"

# Parse args
PROFILE="debug"
CARGO_FLAGS=""
if [[ "${1:-}" == "--release" ]]; then
    PROFILE="release"
    CARGO_FLAGS="--release"
fi

# Verify we're in the right repo
if [[ ! -f "$TUI_ROOT/crates/omd/Cargo.toml" ]]; then
    echo "ERROR: crates/omd/ not found. Are you in the patched DeepSeek-TUI directory?"
    exit 1
fi

# Build
echo "==> cargo build $CARGO_FLAGS"
cd "$TUI_ROOT"
cargo build --bin deepseek-tui $CARGO_FLAGS

# Copy binary with omd name
SRC="$TUI_ROOT/target/$PROFILE/deepseek-tui"
DST="$TUI_ROOT/target/$PROFILE/deepseek-omd"

if [[ ! -f "$SRC" ]]; then
    echo "ERROR: Build succeeded but binary not found at $SRC"
    exit 1
fi

cp "$SRC" "$DST"
echo "==> Built: $DST"
echo "==> Size: $(du -h "$DST" | cut -f1)"

# Optional: install to ~/.local/bin
if [[ "${2:-}" == "--install" ]] || [[ "${1:-}" == "--install" ]]; then
    INSTALL_DIR="${HOME}/.local/bin"
    mkdir -p "$INSTALL_DIR"
    cp "$DST" "$INSTALL_DIR/deepseek-omd"
    echo "==> Installed to $INSTALL_DIR/deepseek-omd"
    echo "    Make sure $INSTALL_DIR is in your PATH"
fi
