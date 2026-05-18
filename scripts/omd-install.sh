#!/usr/bin/env bash
set -euo pipefail

VERSION="${OMD_VERSION:-latest}"
INSTALL_DIR="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.deepseek-omd"
SOURCE_DIR="${HOME}/.deepseek-omd/source"

REPO_URL="${OMD_REPO_URL:-https://github.com/emptylower/oh-my-deepseek.git}"

echo "=== OhMyDeepSeek Installer ==="
echo ""

# Check dependencies
command -v cargo >/dev/null 2>&1 || { echo "Error: Rust/Cargo required. Install from https://rustup.rs"; exit 1; }
command -v git >/dev/null 2>&1 || { echo "Error: Git required."; exit 1; }

# Clone or update source
if [ -d "$SOURCE_DIR" ]; then
    echo "Updating existing source..."
    cd "$SOURCE_DIR"
    git fetch origin
    git checkout main
    git pull origin main
else
    echo "Cloning DeepSeek-TUI..."
    git clone --branch main "$REPO_URL" "$SOURCE_DIR"
    cd "$SOURCE_DIR"
fi

# Build
echo "Building deepseek-omd..."
cargo build --release --bin deepseek-tui

# Install
mkdir -p "$INSTALL_DIR"
cp target/release/deepseek-tui "$INSTALL_DIR/deepseek-omd"
chmod +x "$INSTALL_DIR/deepseek-omd"

# Config
mkdir -p "$CONFIG_DIR"

echo ""
echo "=== Installation complete ==="
echo "Binary: $INSTALL_DIR/deepseek-omd"
echo ""
echo "Make sure $INSTALL_DIR is in your PATH."
echo "Run: deepseek-omd"
