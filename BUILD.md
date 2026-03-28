# ClipDeck — Build & Run Guide

## Prerequisites (Pop!_OS / Ubuntu 22.04+)

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# System libraries
sudo apt update
sudo apt install -y \
    libgtk-4-dev \
    libadwaita-1-dev \
    libxcb1-dev \
    libx11-dev \
    libxcb-keysyms1-dev \
    pkg-config \
    build-essential \
    xdotool          # optional — enables auto-paste after selection
```

---

## Build

```bash
cd ~/Builds/ClipDeck

# Debug build (faster compile, verbose logging)
cargo build

# Release build (optimised, stripped binary)
cargo build --release
```

The binary lands at `target/release/clipdeck`.

---

## Run

```bash
# From the project directory (debug)
RUST_LOG=debug cargo run

# Or run the release binary directly
./target/release/clipdeck
```

Press **Super + V** (Windows key + V) to open the popup.
Press **Escape** or click outside to close it.

---

## Enable Autostart (systemd user service)

```bash
# 1. Copy the binary to a stable location
sudo cp target/release/clipdeck /usr/local/bin/clipdeck

# 2. Install the service file
mkdir -p ~/.config/systemd/user
cp systemd/clipdeck.service ~/.config/systemd/user/

# 3. Reload and enable
systemctl --user daemon-reload
systemctl --user enable --now clipdeck.service

# 4. Check status
systemctl --user status clipdeck.service
```

To stop / disable:

```bash
systemctl --user disable --now clipdeck.service
```

---

## Configuration

Settings are stored at `~/.config/clipdeck/settings.json` and created on
first launch with defaults.  Edit the file to customise:

| Key | Default | Description |
|-----|---------|-------------|
| `max_history` | 50 | Max clipboard entries retained |
| `poll_interval_ms` | 500 | How often to check the clipboard |
| `max_entry_size` | 1048576 | Skip entries larger than this (bytes) |
| `ignored_patterns` | see file | Substrings that mark content as sensitive |
| `github_repo` | `yourusername/clipdeck` | Repo slug for update checks |

---

## Database

SQLite database is at `~/.local/share/clipdeck/history.db`.
You can inspect it with any SQLite viewer:

```bash
sqlite3 ~/.local/share/clipdeck/history.db \
  "SELECT id, substr(content,1,60), datetime(timestamp,'unixepoch','localtime'), pinned FROM clipboard_history ORDER BY timestamp DESC LIMIT 20;"
```

---

## Logging

```bash
# Structured output
RUST_LOG=debug clipdeck 2>&1 | less

# Via systemd
journalctl --user -u clipdeck.service -f
```

---

## Wayland Note

Global hotkey registration uses the X11 `XGrabKey` API.  On a **pure
Wayland** session without XWayland, the Super+V shortcut will not register
automatically.

**Workaround (GNOME):**

1. Open *Settings → Keyboard → Custom Shortcuts*
2. Add a new shortcut:
   - **Name**: ClipDeck
   - **Command**: `/usr/local/bin/clipdeck --show`  *(flag planned for Phase 2)*
   - **Shortcut**: Super+V

The clipboard monitoring and UI themselves work on both X11 and Wayland.
