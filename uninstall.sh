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
[[ -f "$SERVICE" ]] && rm "$SERVICE" && echo -e "  ✓ Service file removed"
systemctl --user daemon-reload 2>/dev/null || true

# Remove binary
if [[ -f /usr/local/bin/clipdeck ]]; then
    sudo rm /usr/local/bin/clipdeck
    echo -e "  ✓ Binary removed from /usr/local/bin"
fi

# Remove config (ask first)
CONFIG_DIR="$HOME/.config/clipdeck"
if [[ -d "$CONFIG_DIR" ]]; then
    read -rp "  Remove config at ${CONFIG_DIR}? [y/N] " yn
    [[ "$yn" =~ ^[Yy]$ ]] && rm -rf "$CONFIG_DIR" && echo -e "  ✓ Config removed"
fi

echo -e "\n${GREEN}ClipDeck uninstalled.${RESET} Your clipboard history is preserved at ~/.local/share/clipdeck/\n"
