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
INSTALL_DIR="$HOME/.local/bin"
INSTALL_BIN="$INSTALL_DIR/clipdeck"
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
for dep in curl python3; do
    command -v "$dep" &>/dev/null || die "$dep is required but not installed."
done

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

# ── Fetch latest release (parse JSON with python3, not fragile grep/cut) ─────
info "Checking latest release..."
API_URL="https://api.github.com/repos/${REPO}/releases/latest"
RELEASE_JSON=$(curl -fsSL "$API_URL") || die "Could not reach GitHub API"

VERSION=$(echo "$RELEASE_JSON" | python3 -c "
import sys, json
r = json.load(sys.stdin)
print(r['tag_name'])
") || die "Could not parse release tag from GitHub API response."

[[ -z "$VERSION" ]] && die "No release found. Have you published a release yet?"

DOWNLOAD_URL=$(echo "$RELEASE_JSON" | python3 -c "
import sys, json
r = json.load(sys.stdin)
assets = [a for a in r['assets'] if 'linux' in a['name'] and 'x86_64' in a['name']]
if not assets:
    sys.exit(1)
print(assets[0]['browser_download_url'])
") || die "Binary asset '${BINARY_NAME}' not found in release ${VERSION}."

SHA_URL=$(echo "$RELEASE_JSON" | python3 -c "
import sys, json
r = json.load(sys.stdin)
assets = [a for a in r['assets'] if a['name'].endswith('.sha256')]
print(assets[0]['browser_download_url'] if assets else '')
" 2>/dev/null || echo "")

info "Found ClipDeck ${VERSION}"

# ── Download binary ───────────────────────────────────────────────────────────
TMP_BIN=$(mktemp /tmp/clipdeck.XXXXXX)
info "Downloading ${BINARY_NAME}..."
curl -fsSL --progress-bar "$DOWNLOAD_URL" -o "$TMP_BIN"
chmod +x "$TMP_BIN"

# ── Verify SHA256 (if .sha256 asset exists in release) ───────────────────────
if [[ -n "$SHA_URL" ]]; then
    TMP_SHA=$(mktemp /tmp/clipdeck.sha256.XXXXXX)
    curl -fsSL "$SHA_URL" -o "$TMP_SHA"
    EXPECTED=$(awk '{print $1}' "$TMP_SHA")
    ACTUAL=$(sha256sum "$TMP_BIN" | awk '{print $1}')
    rm -f "$TMP_SHA"
    if [[ "$EXPECTED" != "$ACTUAL" ]]; then
        rm -f "$TMP_BIN"
        die "SHA256 mismatch — download may be corrupted.\n  Expected: $EXPECTED\n  Got:      $ACTUAL"
    fi
    success "SHA256 verified"
fi

# ── Install binary to ~/.local/bin (no sudo required) ────────────────────────
mkdir -p "$INSTALL_DIR"
mv "$TMP_BIN" "$INSTALL_BIN"
chmod +x "$INSTALL_BIN"
success "Binary installed → ${INSTALL_BIN}"

# ── Ensure ~/.local/bin is on PATH ────────────────────────────────────────────
# Note: when this script runs via "curl | bash", $PATH reflects the subshell
# environment, not the user's interactive shell — so we check the rc files
# directly rather than checking $PATH.
PATH_LINE='export PATH="$HOME/.local/bin:$PATH"'
for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
    if [[ -f "$rc" ]] && ! grep -qF '.local/bin' "$rc"; then
        printf '\n# Added by ClipDeck installer\n%s\n' "$PATH_LINE" >> "$rc"
        success "Added ~/.local/bin to PATH in $rc"
    fi
done
# Create the file if neither existed
if [[ ! -f "$HOME/.bashrc" && ! -f "$HOME/.zshrc" ]]; then
    printf '# Added by ClipDeck installer\n%s\n' "$PATH_LINE" >> "$HOME/.bashrc"
    success "Created ~/.bashrc with PATH entry"
fi

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
echo -e "  Press ${BOLD}Super+Alt+V${RESET} to open the clipboard manager"
echo -e "  Binary:   ${CYAN}${INSTALL_BIN}${RESET}"
echo -e "  Config:   ${CYAN}~/.config/clipdeck/settings.json${RESET}"
echo -e "  Database: ${CYAN}~/.local/share/clipdeck/history.db${RESET}"
echo -e "  Logs:     ${CYAN}journalctl --user -u clipdeck.service -f${RESET}"
echo -e "  Uninstall: ${CYAN}curl -fsSL https://raw.githubusercontent.com/${REPO}/production/uninstall.sh | bash${RESET}\n"
