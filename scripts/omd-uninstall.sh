#!/usr/bin/env bash
set -euo pipefail

INSTALL_DIR="${HOME}/.local/bin"
CONFIG_DIR="${HOME}/.deepseek-omd"

echo "=== OhMyDeepSeek Uninstaller ==="
echo ""

# Remove binary
if [ -f "$INSTALL_DIR/deepseek-omd" ]; then
    rm "$INSTALL_DIR/deepseek-omd"
    echo "Removed: $INSTALL_DIR/deepseek-omd"
fi

# Remove config and source
if [ -d "$CONFIG_DIR" ]; then
    read -p "Remove config and source ($CONFIG_DIR)? [y/N] " confirm
    if [[ "$confirm" =~ ^[Yy]$ ]]; then
        rm -rf "$CONFIG_DIR"
        echo "Removed: $CONFIG_DIR"
    fi
fi

# Do NOT touch ~/.deepseek/ (original TUI config)
echo ""
echo "=== Uninstall complete ==="
echo "Note: ~/.deepseek/ (original TUI config) was NOT modified."
