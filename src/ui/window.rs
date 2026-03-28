//! Main popup window for ClipDeck.
//!
//! The window is created once and toggled visible/hidden on each Super+V press.
//! It is:
//!   • Decoration-free (frameless, dark)
//!   • Fixed size 640×520
//!   • Keyboard-driven (Ctrl+1–4 switch tabs, Esc closes)
//!   • Auto-hidden on focus loss

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use gtk4::prelude::*;
use gtk4::{
    gdk, Box as GBox, Button, CssProvider, EventControllerKey, Label, Orientation, Stack,
};

use crate::core::clipboard::write_to_clipboard;
use crate::core::{ClipboardItem, Storage};
use crate::system::commands::simulate_paste;
use crate::ui::tabs::clipboard_tab::ClipboardTab;
use crate::ui::tabs::emoji_tab::EmojiTab;
use crate::ui::tabs::shortcuts_tab::ShortcutsTab;
use crate::ui::tabs::tools_tab::ToolsTab;

const WINDOW_WIDTH: i32 = 640;
const WINDOW_HEIGHT: i32 = 520;

/// The main popup window.  Hold one instance for the lifetime of the process.
pub struct ClipDeckWindow {
    pub window: gtk4::Window,
    clipboard_tab: ClipboardTab,
    emoji_tab: EmojiTab,
    shortcuts_tab: ShortcutsTab,
    #[allow(dead_code)]
    tools_tab: ToolsTab,
    stack: Stack,
    storage: Arc<Mutex<Storage>>,
    /// Index of the currently active tab (0–3).
    active_tab: Arc<AtomicUsize>,
}

impl ClipDeckWindow {
    /// Create and configure the window.  The window starts hidden.
    pub fn new(
        app: &impl gtk4::prelude::IsA<gtk4::Application>,
        storage: Arc<Mutex<Storage>>,
    ) -> Self {
        // ── Load CSS ──────────────────────────────────────────────────────────
        load_css();

        // ── Window ────────────────────────────────────────────────────────────
        let window = gtk4::Window::builder()
            .application(app)
            .title("ClipDeck")
            .default_width(WINDOW_WIDTH)
            .default_height(WINDOW_HEIGHT)
            .decorated(false)
            .resizable(false)
            .build();

        window.add_css_class("clipdeck-window");

        // ── Root layout ───────────────────────────────────────────────────────
        let root = GBox::new(Orientation::Vertical, 0);

        // Header bar
        let header = build_header();
        root.append(&header);

        // ── Tab bar ───────────────────────────────────────────────────────────
        let tab_row = GBox::new(Orientation::Horizontal, 0);
        tab_row.add_css_class("tab-row");
        tab_row.set_spacing(4);
        tab_row.set_margin_start(8);
        tab_row.set_margin_end(8);
        tab_row.set_margin_top(4);
        tab_row.set_margin_bottom(4);

        let tab_clipboard = make_tab_btn("Clipboard");
        let tab_emoji     = make_tab_btn("Emoji");
        let tab_shortcuts = make_tab_btn("Shortcuts");
        let tab_tools     = make_tab_btn("Tools");

        tab_row.append(&tab_clipboard);
        tab_row.append(&tab_emoji);
        tab_row.append(&tab_shortcuts);
        tab_row.append(&tab_tools);
        root.append(&tab_row);

        // ── Tab content stack ─────────────────────────────────────────────────
        let stack = Stack::new();
        stack.set_vexpand(true);
        stack.set_transition_type(gtk4::StackTransitionType::SlideLeftRight);
        stack.set_transition_duration(150);

        // Build tabs
        let clipboard_tab = ClipboardTab::new();

        // close_window callback for emoji: hide the GTK window.
        // We use a clone of the window handle captured in a plain Fn closure.
        // This will be wired after `me` is constructed; for now build the tab
        // with a no-op that gets replaced via the real window once we have it.
        // Actually we just capture the window clone directly here.
        let win_for_emoji = window.clone();
        let emoji_tab = EmojiTab::new(move || {
            win_for_emoji.hide();
        });

        let shortcuts_tab = ShortcutsTab::new(storage.clone());
        let tools_tab = ToolsTab::new(storage.clone());

        stack.add_named(&clipboard_tab.root, Some("clipboard"));
        stack.add_named(&emoji_tab.root,    Some("emoji"));
        stack.add_named(&shortcuts_tab.root, Some("shortcuts"));
        stack.add_named(&tools_tab.root,    Some("tools"));

        root.append(&stack);
        window.set_child(Some(&root));

        let active_tab = Arc::new(AtomicUsize::new(0));

        // ── Wire tab buttons ──────────────────────────────────────────────────
        let tab_names = ["clipboard", "emoji", "shortcuts", "tools"];
        let tab_btns  = [
            tab_clipboard.clone(),
            tab_emoji.clone(),
            tab_shortcuts.clone(),
            tab_tools.clone(),
        ];

        // Mark first tab active immediately.
        tab_clipboard.add_css_class("active");

        for (i, btn) in tab_btns.iter().enumerate() {
            let stack_c = stack.clone();
            let active_c = active_tab.clone();
            let btns_c: [Button; 4] = [
                tab_clipboard.clone(),
                tab_emoji.clone(),
                tab_shortcuts.clone(),
                tab_tools.clone(),
            ];
            let name = tab_names[i];
            btn.connect_clicked(move |_| {
                stack_c.set_visible_child_name(name);
                active_c.store(i, Ordering::Relaxed);
                for (j, b) in btns_c.iter().enumerate() {
                    if j == i {
                        b.add_css_class("active");
                    } else {
                        b.remove_css_class("active");
                    }
                }
            });
        }

        // ── Keyboard shortcuts ────────────────────────────────────────────────
        let key_ctrl = EventControllerKey::new();
        {
            let win_clone      = window.clone();
            let stack_clone    = stack.clone();
            let active_clone = active_tab.clone();
            let btns_kb: [Button; 4] = [
                tab_clipboard.clone(),
                tab_emoji.clone(),
                tab_shortcuts.clone(),
                tab_tools.clone(),
            ];

            key_ctrl.connect_key_pressed(move |_, keyval, _keycode, mods| {
                // Esc → hide
                if keyval == gdk::Key::Escape {
                    win_clone.hide();
                    return gtk4::glib::Propagation::Stop;
                }

                // Ctrl+1 … Ctrl+4 → switch tabs
                if mods.contains(gdk::ModifierType::CONTROL_MASK) {
                    let idx: Option<usize> = match keyval {
                        gdk::Key::_1 => Some(0),
                        gdk::Key::_2 => Some(1),
                        gdk::Key::_3 => Some(2),
                        gdk::Key::_4 => Some(3),
                        _ => None,
                    };
                    if let Some(i) = idx {
                        stack_clone.set_visible_child_name(tab_names[i]);
                        active_clone.store(i, Ordering::Relaxed);
                        for (j, b) in btns_kb.iter().enumerate() {
                            if j == i {
                                b.add_css_class("active");
                            } else {
                                b.remove_css_class("active");
                            }
                        }
                        return gtk4::glib::Propagation::Stop;
                    }
                }

                gtk4::glib::Propagation::Proceed
            });
        }
        window.add_controller(key_ctrl);

        // ── Auto-hide on focus loss ───────────────────────────────────────────
        {
            let win_clone = window.clone();
            window.connect_is_active_notify(move |w| {
                if !w.is_active() {
                    win_clone.hide();
                }
            });
        }

        let me = Self {
            window,
            clipboard_tab,
            emoji_tab,
            shortcuts_tab,
            tools_tab,
            stack,
            storage: storage.clone(),
            active_tab,
        };

        // Wire clipboard item-activated callback.
        {
            let win = me.window.clone();
            me.clipboard_tab.connect_item_activated(move |item| {
                on_item_selected(&item, &win);
            });
        }

        me
    }

    // ── Public interface ──────────────────────────────────────────────────────

    /// Toggle the window between visible and hidden.
    pub fn toggle(&self) {
        if self.window.is_visible() {
            self.window.hide();
        } else {
            self.show();
        }
    }

    /// Show the window, refresh clipboard history, and focus the active tab.
    pub fn show(&self) {
        self.refresh_history();

        // Focus the search entry in the active tab.
        match self.active_tab.load(Ordering::Relaxed) {
            0 => self.clipboard_tab.focus_search(),
            1 => self.emoji_tab.focus_search(),
            _ => {}
        }

        self.window.present();
    }

    /// Append a freshly-captured item to the top of the clipboard list without
    /// reloading the full history.
    pub fn prepend_item(&self, item: ClipboardItem) {
        self.clipboard_tab.prepend_item(item);
    }

    // ── Internals ─────────────────────────────────────────────────────────────

    fn refresh_history(&self) {
        let history = self
            .storage
            .lock()
            .ok()
            .and_then(|db| db.get_history(50).ok())
            .unwrap_or_default();

        self.clipboard_tab.load_history(history);
        self.shortcuts_tab.reload();
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn on_item_selected(item: &ClipboardItem, window: &gtk4::Window) {
    match write_to_clipboard(&item.content) {
        Ok(_) => log::info!("Copied to clipboard: {}", item.preview()),
        Err(e) => log::error!("Failed to write clipboard: {e}"),
    }

    window.hide();

    // Best-effort: simulate Ctrl+V in the previously focused window.
    if let Err(e) = simulate_paste() {
        log::debug!("Auto-paste unavailable: {e}");
    }
}

fn make_tab_btn(label: &str) -> Button {
    let btn = Button::builder().label(label).build();
    btn.add_css_class("tab-btn");
    btn
}

fn build_header() -> GBox {
    let header = GBox::new(Orientation::Horizontal, 0);
    header.add_css_class("clipdeck-header");
    header.set_margin_start(16);
    header.set_margin_end(8);
    header.set_margin_top(12);
    header.set_margin_bottom(4);

    let title = Label::new(Some("ClipDeck"));
    title.add_css_class("clipdeck-title");
    title.set_hexpand(true);
    title.set_halign(gtk4::Align::Start);

    let version = Label::new(Some(concat!("v", env!("CARGO_PKG_VERSION"))));
    version.add_css_class("clipdeck-version");

    header.append(&title);
    header.append(&version);

    header
}

fn load_css() {
    let css = CssProvider::new();
    css.load_from_data(APP_CSS);

    gtk4::style_context_add_provider_for_display(
        &gdk::Display::default().expect("no GDK display"),
        &css,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

// ── Stylesheet ────────────────────────────────────────────────────────────────

const APP_CSS: &str = r#"
/* ── Window chrome ── */
.clipdeck-window {
    background-color: #1e1e2e;
    border-radius: 14px;
    border: 1px solid rgba(255,255,255,0.08);
    box-shadow: 0 24px 64px rgba(0,0,0,0.6);
}

/* ── Header ── */
.clipdeck-header {
    padding-bottom: 4px;
}

.clipdeck-title {
    font-family: "Inter", "Cantarell", sans-serif;
    font-size: 15px;
    font-weight: 700;
    color: #cdd6f4;
    letter-spacing: 0.5px;
}

.clipdeck-version {
    font-size: 11px;
    color: #585b70;
}

/* ── Tab row ── */
.tab-row {
    padding: 4px 8px;
    border-bottom: 1px solid rgba(255,255,255,0.06);
    background-color: #181825;
}

/* ── Tab buttons ── */
.tab-btn {
    font-size: 12px;
    font-weight: 600;
    color: #6c7086;
    padding: 4px 12px;
    border-radius: 6px;
    border: none;
    background: transparent;
    min-height: 28px;
    transition: background-color 100ms, color 100ms;
}

.tab-btn:hover {
    color: #cdd6f4;
    background-color: rgba(255,255,255,0.05);
}

.tab-btn.active {
    color: #89b4fa;
    background-color: rgba(137,180,250,0.15);
}

/* ── Search entry ── */
entry.search {
    background-color: #313244;
    color: #cdd6f4;
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 8px;
    padding: 8px 12px;
    font-size: 14px;
    caret-color: #89b4fa;
}

entry.search:focus {
    border-color: #89b4fa;
    box-shadow: 0 0 0 2px rgba(137,180,250,0.2);
}

/* ── Clipboard list ── */
.clipboard-list {
    background-color: transparent;
}

.clipboard-list row {
    background-color: transparent;
    border-radius: 8px;
    margin: 2px 8px;
    transition: background-color 120ms;
}

.clipboard-list row:hover {
    background-color: rgba(255,255,255,0.04);
}

.clipboard-list row:selected {
    background-color: rgba(137,180,250,0.18);
}

.clipboard-list row:selected .clipboard-item-label {
    color: #cdd6f4;
}

/* ── Clipboard item labels ── */
.clipboard-item-label {
    font-size: 13px;
    color: #bac2de;
    font-family: "JetBrains Mono", "Fira Code", monospace;
}

.clipboard-item-time {
    font-size: 11px;
    color: #585b70;
}

/* ── Emoji picker ── */
.emoji-btn {
    font-size: 22px;
    min-width: 40px;
    min-height: 40px;
    padding: 4px;
    border-radius: 6px;
    border: none;
    background: transparent;
    transition: background-color 80ms;
}

.emoji-btn:hover {
    background-color: rgba(137,180,250,0.15);
}

.emoji-btn:active {
    background-color: rgba(137,180,250,0.3);
}

/* ── Shortcuts tab ── */
.shortcut-list {
    background-color: transparent;
}

.shortcut-list row {
    background-color: transparent;
    border-radius: 8px;
    margin: 2px 8px;
}

.shortcut-list row:hover {
    background-color: rgba(255,255,255,0.04);
}

.shortcut-row {
    border-bottom: 1px solid rgba(255,255,255,0.04);
}

.shortcut-name {
    font-size: 13px;
    font-weight: 600;
    color: #cdd6f4;
}

.shortcut-cmd {
    font-size: 12px;
    color: #6c7086;
    font-family: "JetBrains Mono", "Fira Code", monospace;
}

.shortcut-run-btn {
    font-size: 12px;
    color: #a6e3a1;
    background-color: rgba(166,227,161,0.12);
    border: 1px solid rgba(166,227,161,0.2);
    border-radius: 6px;
    padding: 3px 10px;
    min-height: 24px;
}

.shortcut-run-btn:hover {
    background-color: rgba(166,227,161,0.22);
}

.shortcut-del-btn {
    font-size: 12px;
    color: #f38ba8;
    background-color: rgba(243,139,168,0.10);
    border: 1px solid rgba(243,139,168,0.2);
    border-radius: 6px;
    padding: 3px 8px;
    min-height: 24px;
}

.shortcut-del-btn:hover {
    background-color: rgba(243,139,168,0.22);
}

.add-shortcut-toggle {
    font-size: 12px;
    font-weight: 600;
    color: #89b4fa;
    background: transparent;
    border: 1px dashed rgba(137,180,250,0.3);
    border-radius: 6px;
    padding: 4px 12px;
    transition: background-color 100ms;
}

.add-shortcut-toggle:hover {
    background-color: rgba(137,180,250,0.08);
}

/* ── Tools tab ── */
.tools-title {
    font-size: 14px;
    font-weight: 700;
    color: #cdd6f4;
}

.tool-btn {
    font-size: 13px;
    font-weight: 600;
    color: #cdd6f4;
    background-color: #313244;
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 8px;
    padding: 12px 16px;
    min-height: 52px;
    transition: background-color 120ms;
}

.tool-btn:hover {
    background-color: #45475a;
}

.tool-btn:active {
    background-color: rgba(137,180,250,0.2);
}

/* ── Shared status label ── */
.status-label {
    font-size: 12px;
    color: #a6adc8;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    opacity: 0.85;
}

/* ── Dim label (empty state) ── */
.dim-label {
    font-size: 13px;
    color: #45475a;
}

/* ── General entry styling ── */
entry {
    background-color: #313244;
    color: #cdd6f4;
    border: 1px solid rgba(255,255,255,0.08);
    border-radius: 6px;
    padding: 6px 10px;
    caret-color: #89b4fa;
}

entry:focus {
    border-color: #89b4fa;
}

/* ── Suggested action button (save) ── */
button.suggested-action {
    background-color: #89b4fa;
    color: #1e1e2e;
    border-radius: 6px;
    font-weight: 700;
    padding: 4px 16px;
    border: none;
}

button.suggested-action:hover {
    background-color: #b4befe;
}
"#;
