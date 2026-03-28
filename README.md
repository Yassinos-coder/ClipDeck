<img width="100%" src="https://capsule-render.vercel.app/api?type=waving&color=0891b2&height=140&section=header&text=ClipDeck&fontSize=60&fontColor=ffffff&animation=fadeIn&fontAlignY=38&desc=Fast%2C%20native%20clipboard%20manager%20%26%20launcher%20for%20Linux&descAlignY=58&descAlign=50" alt="ClipDeck Header" />

<p align="center">
  <img src="https://img.shields.io/badge/Rust-stable-orange?style=for-the-badge&logo=rust&logoColor=white&labelColor=1c1917" alt="Rust" />
  <img src="https://img.shields.io/badge/GTK4-blue?style=for-the-badge&logo=gnome&logoColor=white&labelColor=1c1917" alt="GTK4" />
  <img src="https://img.shields.io/badge/Libadwaita-0.7-purple?style=for-the-badge&logo=gnome&logoColor=white&labelColor=1c1917" alt="Libadwaita" />
  <img src="https://img.shields.io/badge/SQLite-bundled-lightblue?style=for-the-badge&logo=sqlite&logoColor=white&labelColor=1c1917" alt="SQLite" />
  <img src="https://img.shields.io/badge/Platform-Pop!__OS%20%2F%20Ubuntu-48B9C7?style=for-the-badge&logo=linux&logoColor=white&labelColor=1c1917" alt="Platform" />
  <img src="https://img.shields.io/badge/License-MIT-green?style=for-the-badge&labelColor=1c1917" alt="MIT License" />
</p>

<p align="center">
  <strong>ClipDeck</strong> is a keyboard-driven clipboard manager and productivity launcher for Linux (Pop!_OS / GNOME / X11), inspired by Windows Win+V and Raycast.<br/>
  Press <kbd>Super</kbd>+<kbd>V</kbd> to open it from anywhere. Zero electron, zero JS — pure Rust and GTK4.
</p>

---

## Features

| Tab | What it does |
|-----|-------------|
| **Clipboard** | Monitors your clipboard continuously, stores the last 50 entries in SQLite, fuzzy-search through history, press Enter to paste |
| **Emoji** | 750+ emoji with instant fuzzy search — click to copy and the window closes automatically |
| **Shortcuts** | Save shell commands you run often (e.g. `git status`, `code .`) and execute them with one click |
| **Tools** | Flush DNS cache, clear clipboard history, open config directory, install/uninstall autostart |

### Core behaviour
- **Global hotkey** — Super+V registered via X11 XGrabKey (works on X11; graceful fallback on Wayland)
- **Lightweight** — native Rust binary, no runtime, minimal memory footprint
- **Dark mode** — Catppuccin Mocha palette via Libadwaita + custom CSS
- **Sensitive content filter** — entries containing "password", "token", "secret", etc. are silently skipped
- **SQLite storage** — clipboard history and shortcuts persisted to `~/.local/share/clipdeck/history.db`
- **Auto-update check** — checks GitHub releases on startup and logs if a newer version is available
- **Systemd user service** — optional autostart via `~/.config/systemd/user/clipdeck.service`

---

## Screenshot

```
┌──────────────────────────────────────────────────────┐
│  ClipDeck                                       v0.1 │
├──────────────────────────────────────────────────────┤
│  [📋 Clipboard]  [😀 Emoji]  [⚡ Shortcuts]  [🔧 Tools] │
├──────────────────────────────────────────────────────┤
│  🔍 Search clipboard…                               │
│  ┌────────────────────────────────────────────────┐  │
│  │ Hello World                         just now   │  │
│  │ git commit -m "fix: correct typo"  2 min ago   │  │
│  │ https://github.com/yassinos-coder   5 min ago   │  │
│  │ SELECT * FROM users LIMIT 10;      12 min ago  │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

---

## Installation

### Prerequisites (Pop!_OS / Ubuntu 22.04+)

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# System libraries
sudo apt update
sudo apt install -y \
    libgtk-4-dev \
    libadwaita-1-dev \
    libx11-dev \
    pkg-config \
    build-essential \
    xdotool        # optional — enables auto-paste after selection
```

### Build

```bash
git clone https://github.com/yassinos-coder/clipdeck.git
cd clipdeck

# Release build (optimised, stripped)
cargo build --release
```

Binary lands at `target/release/clipdeck`.

### Run

```bash
# Direct
./target/release/clipdeck

# Or install system-wide
sudo cp target/release/clipdeck /usr/local/bin/clipdeck
clipdeck
```

Press **Super+V** to open the popup. Press **Esc** or click outside to close.

### Autostart with systemd

```bash
sudo cp target/release/clipdeck /usr/local/bin/clipdeck

mkdir -p ~/.config/systemd/user
cp systemd/clipdeck.service ~/.config/systemd/user/

systemctl --user daemon-reload
systemctl --user enable --now clipdeck.service

# Check status
systemctl --user status clipdeck.service
```

---

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Super + V` | Toggle ClipDeck open/closed |
| `Esc` | Close window |
| `↑ / ↓` | Navigate clipboard list |
| `Enter` | Copy selected item and close |
| `Ctrl+1` | Switch to Clipboard tab |
| `Ctrl+2` | Switch to Emoji tab |
| `Ctrl+3` | Switch to Shortcuts tab |
| `Ctrl+4` | Switch to Tools tab |

---

## Configuration

Settings file: `~/.config/clipdeck/settings.json` (auto-created on first run)

```json
{
  "max_history": 50,
  "poll_interval_ms": 500,
  "max_entry_size": 1048576,
  "ignored_patterns": ["password", "passwd", "token", "secret", "api_key"],
  "github_repo": "yassinos-coder/clipdeck",
  "version": "0.1.0"
}
```

---

## Architecture

```
src/
├── main.rs               Entry point, GTK app, thread wiring
├── config/               Settings (JSON, ~/.config/clipdeck/)
├── core/
│   ├── clipboard.rs      arboard poller + AppMessage channel
│   ├── storage.rs        SQLite CRUD (clipboard + shortcuts)
│   ├── history.rs        ClipboardItem model
│   └── shortcuts.rs      Shortcut model + sh -c execution
├── data/
│   └── emoji.rs          Static emoji dataset (750+ entries)
├── ui/
│   ├── window.rs         GTK4 popup, Stack, tab bar, CSS
│   └── tabs/
│       ├── clipboard_tab.rs  Fuzzy search + ListBox
│       ├── emoji_tab.rs      FlowBox emoji grid
│       ├── shortcuts_tab.rs  Shortcut CRUD + run output
│       └── tools_tab.rs      System tool buttons
├── system/
│   ├── hotkeys.rs        x11rb XGrabKey (Super+V)
│   ├── autostart.rs      systemd service generator
│   └── commands.rs       flush_dns, simulate_paste, etc.
└── services/
    └── updater.rs        GitHub releases version check
```

**Cross-thread communication:** All background threads (clipboard poller, hotkey listener, version checker) send `AppMessage` values to the GTK main loop via `glib::MainContext::channel`. No unsafe code outside the `x11rb` X11 bindings.

---

## Wayland Note

The Super+V global hotkey uses the X11 `XGrabKey` API. On a **pure Wayland** session without XWayland, the hotkey registration will fail gracefully with a log warning.

**Workaround (GNOME Wayland):**

1. Open *Settings → Keyboard → Custom Shortcuts*
2. Name: `ClipDeck`, Command: `/usr/local/bin/clipdeck`, Shortcut: `Super+V`

The clipboard monitor and all UI features work on both X11 and Wayland.

---

## Tech Stack

<p align="left">
<img src="https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white" height="28" alt="Rust" />
<img src="https://img.shields.io/badge/GTK4-4A86CF?style=flat-square&logo=gnome&logoColor=white" height="28" alt="GTK4" />
<img src="https://img.shields.io/badge/SQLite-003B57?style=flat-square&logo=sqlite&logoColor=white" height="28" alt="SQLite" />
<img src="https://img.shields.io/badge/Tokio-async-black?style=flat-square&logo=rust&logoColor=white" height="28" alt="Tokio" />
<img src="https://img.shields.io/badge/Linux-FCC624?style=flat-square&logo=linux&logoColor=black" height="28" alt="Linux" />
</p>

- **[gtk4-rs](https://gtk-rs.org/)** — GTK4 bindings
- **[libadwaita](https://world.pages.gitlab.gnome.org/Rust/libadwaita-rs/)** — Adwaita design system
- **[arboard](https://github.com/1Password/arboard)** — Cross-platform clipboard access
- **[x11rb](https://github.com/psychon/x11rb)** — Pure Rust X11 bindings (global hotkey)
- **[rusqlite](https://github.com/rusqlite/rusqlite)** — SQLite (bundled, no system dep)
- **[fuzzy-matcher](https://github.com/lotabout/fuzzy-matcher)** — Skim fuzzy search algorithm
- **[reqwest](https://github.com/seanmonstar/reqwest)** — HTTP client for version checks

---

## Contributing

Pull requests welcome. Please:
- Keep the layered architecture (`core` must not import from `ui`)
- Use `anyhow` for errors, `log` macros for logging
- Test on X11 (primary) and XWayland

---

## Author

Built by **Yassine Castro** — Full-Stack Developer based in Casablanca, Morocco.

<p align="left">
  <a href="https://www.github.com/yassinos-coder" target="_blank" rel="noreferrer"><img src="https://img.shields.io/badge/GitHub-yassinos--coder-0891b2?style=for-the-badge&logo=github&logoColor=white&labelColor=1c1917" alt="GitHub" /></a>
  <a href="https://www.linkedin.com/in/yassine-castro-6a6ba7237" target="_blank" rel="noreferrer"><img src="https://img.shields.io/badge/LinkedIn-Yassine%20Castro-0891b2?style=for-the-badge&logo=linkedin&logoColor=white&labelColor=1c1917" alt="LinkedIn" /></a>
  <a href="https://www.x.com/YassineCastro1" target="_blank" rel="noreferrer"><img src="https://img.shields.io/badge/Twitter-@YassineCastro1-0891b2?style=for-the-badge&logo=x&logoColor=white&labelColor=1c1917" alt="Twitter" /></a>
  <a href="http://yassinoscoder.codes" target="_blank" rel="noreferrer"><img src="https://img.shields.io/badge/Portfolio-yassinoscoder.codes-0891b2?style=for-the-badge&logo=safari&logoColor=white&labelColor=1c1917" alt="Portfolio" /></a>
</p>

---

## Support

If ClipDeck saves you time, consider supporting the project!

<a href="https://paypal.me/the1290srider" target="_blank" rel="noreferrer">
  <img src="https://img.shields.io/badge/Donate%20via%20PayPal-00457C?style=for-the-badge&logo=paypal&logoColor=white" alt="Donate via PayPal" />
</a>

---

## License

MIT — see [LICENSE](LICENSE) for details.

<img width="100%" src="https://capsule-render.vercel.app/api?type=waving&color=0891b2&height=100&section=footer" alt="Footer" />
# ClipDeck
