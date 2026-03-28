#!/usr/bin/env bash
# ╔══════════════════════════════════════════════════════════╗
# ║  ClipDeck — one-line installer for Pop!_OS / Ubuntu      ║
# ║  Usage:                                                  ║
# ║    curl -fsSL https://raw.githubusercontent.com/         ║
# ║      Yassinos-coder/ClipDeck/production/install.sh | bash║
# ╚══════════════════════════════════════════════════════════╝
set -euo pipefail

REPO="Yassinos-coder/ClipDeck"
BINARY_NAME="clipdeck-linux-x86_64"
INSTALL_BIN="/usr/local/bin/clipdeck"
SERVICE_NAME="clipdeck.service"
SERVICE_DIR="$HOME/.config/systemd/user"

# ── Colours ─────────────────────────────────────────────────────────────────
BOLD='\033[1m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
RESET='\033[0m'

info()    { echo -e "${CYAN}→${RESET} $*"; }
success() { echo -e "${GREEN}✓${RESET} $*"; }
warn()    { echo -e "${YELLOW}!${RESET} $*"; }
die()     { echo -e "${RED}✗ Error:${RESET} $*" >&2; exit 1; }

echo -e "\n${BOLD}ClipDeck Installer${RESET}\n"

# ── Check OS ─────────────────────────────────────────────────────────────────
if [[ "$OSTYPE" != "linux-gnu"* ]]; then
    die "ClipDeck requires Linux. Detected: $OSTYPE"
fi

# ── Check dependencies ────────────────────────────────────────────────────────
check_dep() {
    if ! command -v "$1" &>/dev/null; then
        warn "$1 not found. Install with: sudo apt install $2"
        return 1
    fi
}

MISSING=0
check_dep curl   curl  || MISSING=1
check_dep sudo   sudo  || MISSING=1
[[ $MISSING -eq 1 ]] && die "Please install the missing dependencies above."

# ── Detect GTK4 ──────────────────────────────────────────────────────────────
if ! pkg-config --exists gtk4 2>/dev/null; then
    info "GTK4 not detected — installing system libraries..."
    sudo apt-get update -qq
    sudo apt-get install -y \
        libgtk-4-1 \
        libadwaita-1-0 \
        libx11-6 \
        xdotool
fi

# ── Fetch latest release tag ──────────────────────────────────────────────────
info "Checking latest release..."
API_URL="https://api.github.com/repos/${REPO}/releases/latest"
RELEASE_JSON=$(curl -fsSL "$API_URL") || die "Could not reach GitHub API"

VERSION=$(echo "$RELEASE_JSON" | grep '"tag_name"' | head -1 | cut -d'"' -f4)
[[ -z "$VERSION" ]] && die "No release found. Have you published a release yet? See BUILD.md."

DOWNLOAD_URL=$(echo "$RELEASE_JSON" \
    | grep '"browser_download_url"' \
    | grep "$BINARY_NAME\"" \
    | head -1 \
    | cut -d'"' -f4)

[[ -z "$DOWNLOAD_URL" ]] && die "Binary asset '${BINARY_NAME}' not found in release ${VERSION}."

info "Found ClipDeck ${VERSION}"

# ── Download binary ───────────────────────────────────────────────────────────
TMP_BIN=$(mktemp /tmp/clipdeck.XXXXXX)
info "Downloading ${BINARY_NAME}..."
curl -fsSL --progress-bar "$DOWNLOAD_URL" -o "$TMP_BIN"

chmod +x "$TMP_BIN"

# ── Verify sha256 (if available) ──────────────────────────────────────────────
SHA_URL=$(echo "$RELEASE_JSON" \
    | grep '"browser_download_url"' \
    | grep "${BINARY_NAME}.sha256\"" \
    | head -1 \
    | cut -d'"' -f4)

if [[ -n "$SHA_URL" ]]; then
    TMP_SHA=$(mktemp /tmp/clipdeck.sha256.XXXXXX)
    curl -fsSL "$SHA_URL" -o "$TMP_SHA"
    EXPECTED=$(awk '{print $1}' "$TMP_SHA")
    ACTUAL=$(sha256sum "$TMP_BIN" | awk '{print $1}')
    if [[ "$EXPECTED" != "$ACTUAL" ]]; then
        rm -f "$TMP_BIN" "$TMP_SHA"
        die "SHA256 mismatch! Download may be corrupted.\n  Expected: $EXPECTED\n  Got:      $ACTUAL"
    fi
    success "SHA256 verified"
    rm -f "$TMP_SHA"
fi

# ── Install binary ────────────────────────────────────────────────────────────
info "Installing to ${INSTALL_BIN} (requires sudo)..."
sudo mv "$TMP_BIN" "$INSTALL_BIN"
sudo chmod +x "$INSTALL_BIN"
success "Binary installed → ${INSTALL_BIN}"

# ── Install systemd user service ──────────────────────────────────────────────
info "Setting up autostart service..."
mkdir -p "$SERVICE_DIR"

SERVICE_URL="https://raw.githubusercontent.com/${REPO}/production/systemd/${SERVICE_NAME}"
curl -fsSL "$SERVICE_URL" \
    | sed "s|ExecStart=.*|ExecStart=${INSTALL_BIN}|" \
    > "${SERVICE_DIR}/${SERVICE_NAME}"

systemctl --user daemon-reload
systemctl --user enable --now "$SERVICE_NAME" 2>/dev/null || true
success "Autostart service installed and enabled"

# ── Done ─────────────────────────────────────────────────────────────────────
echo -e "\n${BOLD}${GREEN}ClipDeck ${VERSION} installed successfully!${RESET}\n"
echo -e "  Press ${BOLD}Super + V${RESET} to open the clipboard manager"
echo -e "  Config:   ${CYAN}~/.config/clipdeck/settings.json${RESET}"
echo -e "  Database: ${CYAN}~/.local/share/clipdeck/history.db${RESET}"
echo -e "  Logs:     ${CYAN}journalctl --user -u clipdeck.service -f${RESET}"
echo -e "  Uninstall:${CYAN} curl -fsSL https://raw.githubusercontent.com/${REPO}/production/uninstall.sh | bash${RESET}\n"
