#!/usr/bin/env bash
# ClipDeck uninstaller — preserves your clipboard database
set -euo pipefail

BOLD='\033[1m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
RESET='\033[0m'

echo -e "\n${BOLD}ClipDeck Uninstaller${RESET}\n"
echo -e "  Your clipboard database at ${CYAN}~/.local/share/clipdeck/${RESET} will NOT be deleted.\n"

# Stop and disable the service
if systemctl --user is-active clipdeck.service &>/dev/null; then
    systemctl --user disable --now clipdeck.service
    echo -e "  ✓ Service stopped and disabled"
fi

# Remove service file
SERVICE="$HOME/.config/systemd/user/clipdeck.service"
if [[ -f "$SERVICE" ]]; then
    rm "$SERVICE"
    echo -e "  ✓ Service file removed"
fi
systemctl --user daemon-reload 2>/dev/null || true

# Remove binary from ~/.local/bin (no sudo needed)
LOCAL_BIN="$HOME/.local/bin/clipdeck"
if [[ -f "$LOCAL_BIN" ]]; then
    rm "$LOCAL_BIN"
    echo -e "  ✓ Binary removed from ~/.local/bin"
fi

# Also clean up old /usr/local/bin install if it exists
if [[ -f /usr/local/bin/clipdeck ]]; then
    sudo rm /usr/local/bin/clipdeck
    echo -e "  ✓ Legacy binary removed from /usr/local/bin"
fi

# Remove config (ask first)
CONFIG_DIR="$HOME/.config/clipdeck"
if [[ -d "$CONFIG_DIR" ]]; then
    read -rp "  Remove config at ${CONFIG_DIR}? [y/N] " yn
    [[ "$yn" =~ ^[Yy]$ ]] && rm -rf "$CONFIG_DIR" && echo -e "  ✓ Config removed"
fi

echo -e "\n${GREEN}ClipDeck uninstalled.${RESET} Your clipboard history is preserved at ~/.local/share/clipdeck/\n"
