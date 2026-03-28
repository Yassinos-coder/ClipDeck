//! Main popup window for ClipDeck.
//!
//! Visual style: Windows 11 / Fluent Design — white background, clean cards,
//! blue (#0078d4) accent colour.
//!
//! Layout:
//!   ┌─────────────────────────────────────────────────────┐
//!   │  ClipDeck            Clipboard · Emoji · …     v0.1 │  ← header
//!   ├─────────────────────────────────────────────────────┤
//!   │  [📋 Clipboard]  [😀 Emoji]  [⚡ Shortcuts]  [🔧 …] │  ← tab bar
//!   ├─────────────────────────────────────────────────────┤
//!   │  🔍  Search clipboard…                              │
//!   │  ┌──────────────────────────────────────────────┐   │
//!   │  │  Hello World                      just now   │   │  ← clipboard tab
//!   │  └──────────────────────────────────────────────┘   │
//!   ├─────────────────────────────────────────────────────┤
//!   │  ⬇ Downloading v0.2.0… 42%                         │  ← update bar
//!   └─────────────────────────────────────────────────────┘

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use gtk4::prelude::*;
use gtk4::{
    gdk, Box as GBox, Button, CssProvider, EventControllerKey, Label, Orientation,
    Revealer, RevealerTransitionType, Stack,
};

use crate::core::clipboard::write_to_clipboard;
use crate::core::{ClipboardItem, Storage};
use crate::system::commands::simulate_paste;
use crate::ui::tabs::clipboard_tab::ClipboardTab;
use crate::ui::tabs::emoji_tab::EmojiTab;
use crate::ui::tabs::shortcuts_tab::ShortcutsTab;
use crate::ui::tabs::tools_tab::ToolsTab;

const WINDOW_WIDTH: i32 = 620;
const WINDOW_HEIGHT: i32 = 520;

/// The main popup window.
pub struct ClipDeckWindow {
    pub window: gtk4::Window,
    clipboard_tab: ClipboardTab,
    emoji_tab: EmojiTab,
    shortcuts_tab: ShortcutsTab,
    #[allow(dead_code)]
    tools_tab: ToolsTab,
    stack: Stack,
    storage: Arc<Mutex<Storage>>,
    active_tab: Arc<AtomicUsize>,
    /// Slide-in bar at the bottom that shows auto-update progress.
    update_revealer: Revealer,
    update_label: Label,
}

impl ClipDeckWindow {
    pub fn new(
        app: &impl gtk4::prelude::IsA<gtk4::Application>,
        storage: Arc<Mutex<Storage>>,
    ) -> Self {
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

        // ── Root ──────────────────────────────────────────────────────────────
        let root = GBox::new(Orientation::Vertical, 0);

        // Header
        root.append(&build_header());

        // Tab bar
        let tab_clipboard = make_tab_btn("📋  Clipboard");
        let tab_emoji     = make_tab_btn("😀  Emoji");
        let tab_shortcuts = make_tab_btn("⚡  Shortcuts");
        let tab_tools     = make_tab_btn("🔧  Tools");
        tab_clipboard.add_css_class("active");

        let tab_row = GBox::new(Orientation::Horizontal, 2);
        tab_row.add_css_class("tab-row");
        tab_row.set_margin_start(8);
        tab_row.set_margin_end(8);
        tab_row.set_margin_top(4);
        tab_row.set_margin_bottom(4);
        for btn in [&tab_clipboard, &tab_emoji, &tab_shortcuts, &tab_tools] {
            tab_row.append(btn);
        }
        root.append(&tab_row);

        // Separator
        let sep = gtk4::Separator::new(Orientation::Horizontal);
        sep.add_css_class("tab-separator");
        root.append(&sep);

        // ── Content stack ─────────────────────────────────────────────────────
        let stack = Stack::new();
        stack.set_vexpand(true);
        stack.set_transition_type(gtk4::StackTransitionType::SlideLeftRight);
        stack.set_transition_duration(120);

        let clipboard_tab = ClipboardTab::new();

        let win_for_emoji = window.clone();
        let emoji_tab = EmojiTab::new(move || win_for_emoji.hide());

        let shortcuts_tab = ShortcutsTab::new(storage.clone());
        let tools_tab     = ToolsTab::new(storage.clone());

        stack.add_named(&clipboard_tab.root,  Some("clipboard"));
        stack.add_named(&emoji_tab.root,      Some("emoji"));
        stack.add_named(&shortcuts_tab.root,  Some("shortcuts"));
        stack.add_named(&tools_tab.root,      Some("tools"));
        root.append(&stack);

        // ── Update status bar (hidden by default) ─────────────────────────────
        let update_label = Label::new(Some(""));
        update_label.add_css_class("update-bar-label");
        update_label.set_halign(gtk4::Align::Start);
        update_label.set_margin_start(14);
        update_label.set_margin_end(14);
        update_label.set_margin_top(8);
        update_label.set_margin_bottom(8);

        let update_box = GBox::new(Orientation::Horizontal, 0);
        update_box.add_css_class("update-bar");
        update_box.append(&update_label);

        let update_revealer = Revealer::builder()
            .transition_type(RevealerTransitionType::SlideDown)
            .transition_duration(200)
            .reveal_child(false)
            .child(&update_box)
            .build();
        root.append(&update_revealer);

        window.set_child(Some(&root));

        let active_tab = Arc::new(AtomicUsize::new(0));

        // ── Wire tab buttons ──────────────────────────────────────────────────
        const TAB_NAMES: [&str; 4] = ["clipboard", "emoji", "shortcuts", "tools"];
        let all_btns = [
            tab_clipboard.clone(),
            tab_emoji.clone(),
            tab_shortcuts.clone(),
            tab_tools.clone(),
        ];

        for (i, btn) in all_btns.iter().enumerate() {
            let stack_c  = stack.clone();
            let active_c = active_tab.clone();
            let btns_c   = all_btns.clone();
            btn.connect_clicked(move |_| {
                stack_c.set_visible_child_name(TAB_NAMES[i]);
                active_c.store(i, Ordering::Relaxed);
                for (j, b) in btns_c.iter().enumerate() {
                    if j == i { b.add_css_class("active"); } else { b.remove_css_class("active"); }
                }
            });
        }

        // ── Keyboard navigation ───────────────────────────────────────────────
        let key_ctrl = EventControllerKey::new();
        {
            let win_c    = window.clone();
            let stack_c  = stack.clone();
            let active_c = active_tab.clone();
            let btns_c   = all_btns.clone();
            key_ctrl.connect_key_pressed(move |_, keyval, _, mods| {
                if keyval == gdk::Key::Escape {
                    win_c.hide();
                    return gtk4::glib::Propagation::Stop;
                }
                if mods.contains(gdk::ModifierType::CONTROL_MASK) {
                    let idx = match keyval {
                        gdk::Key::_1 => Some(0usize),
                        gdk::Key::_2 => Some(1),
                        gdk::Key::_3 => Some(2),
                        gdk::Key::_4 => Some(3),
                        _ => None,
                    };
                    if let Some(i) = idx {
                        stack_c.set_visible_child_name(TAB_NAMES[i]);
                        active_c.store(i, Ordering::Relaxed);
                        for (j, b) in btns_c.iter().enumerate() {
                            if j == i { b.add_css_class("active"); } else { b.remove_css_class("active"); }
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
            let win_c = window.clone();
            window.connect_is_active_notify(move |w| {
                if !w.is_active() { win_c.hide(); }
            });
        }

        let me = Self {
            window,
            clipboard_tab,
            emoji_tab,
            shortcuts_tab,
            tools_tab,
            stack,
            storage,
            active_tab,
            update_revealer,
            update_label,
        };

        // Wire clipboard item-activated.
        {
            let win = me.window.clone();
            me.clipboard_tab.connect_item_activated(move |item| {
                on_item_selected(&item, &win);
            });
        }

        me
    }

    // ── Public interface ──────────────────────────────────────────────────────

    pub fn toggle(&self) {
        if self.window.is_visible() { self.window.hide(); } else { self.show(); }
    }

    pub fn show(&self) {
        self.refresh_history();
        match self.active_tab.load(Ordering::Relaxed) {
            0 => self.clipboard_tab.focus_search(),
            1 => self.emoji_tab.focus_search(),
            _ => {}
        }
        self.window.present();
    }

    pub fn prepend_item(&self, item: ClipboardItem) {
        self.clipboard_tab.prepend_item(item);
    }

    /// Show the update progress bar with `msg`.
    pub fn show_update_status(&self, msg: &str) {
        self.update_label.set_text(msg);
        self.update_revealer.set_reveal_child(true);
    }

    /// Hide the update progress bar.
    pub fn hide_update_status(&self) {
        self.update_revealer.set_reveal_child(false);
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
        Ok(_)  => log::info!("Copied: {}", item.preview()),
        Err(e) => log::error!("Clipboard write failed: {e}"),
    }
    window.hide();
    if let Err(e) = simulate_paste() {
        log::debug!("Auto-paste unavailable: {e}");
    }
}

fn make_tab_btn(label: &str) -> Button {
    let b = Button::builder().label(label).build();
    b.add_css_class("tab-btn");
    b
}

fn build_header() -> GBox {
    let h = GBox::new(Orientation::Horizontal, 0);
    h.add_css_class("clipdeck-header");
    h.set_margin_start(16);
    h.set_margin_end(12);
    h.set_margin_top(14);
    h.set_margin_bottom(10);

    let title = Label::new(Some("Clipboard"));
    title.add_css_class("clipdeck-title");
    title.set_hexpand(true);
    title.set_halign(gtk4::Align::Start);

    let ver = Label::new(Some(concat!("v", env!("CARGO_PKG_VERSION"))));
    ver.add_css_class("clipdeck-version");

    h.append(&title);
    h.append(&ver);
    h
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

// ── Stylesheet — Windows 11 / Fluent Design ───────────────────────────────────

const APP_CSS: &str = r#"
/* ── Window ── */
.clipdeck-window {
    background-color: #ffffff;
    border-radius: 10px;
    border: 1px solid #d0d0d0;
    box-shadow: 0 8px 32px rgba(0,0,0,0.18), 0 2px 8px rgba(0,0,0,0.08);
}

/* ── Header ── */
.clipdeck-header {
    background-color: #f9f9f9;
    border-bottom: 1px solid #ebebeb;
    border-radius: 10px 10px 0 0;
}

.clipdeck-title {
    font-size: 14px;
    font-weight: 600;
    color: #1a1a1a;
    font-family: "Segoe UI", "Inter", "Cantarell", sans-serif;
}

.clipdeck-version {
    font-size: 11px;
    color: #9e9e9e;
}

/* ── Tab separator ── */
.tab-separator {
    background-color: #ebebeb;
    min-height: 1px;
}

/* ── Tab row ── */
.tab-row {
    background-color: #f9f9f9;
}

/* ── Tab buttons ── */
.tab-btn {
    font-size: 12px;
    font-weight: 500;
    color: #616161;
    padding: 4px 14px;
    border-radius: 6px;
    border: none;
    background: transparent;
    min-height: 30px;
    font-family: "Segoe UI", "Inter", sans-serif;
}

.tab-btn:hover {
    background-color: rgba(0,0,0,0.05);
    color: #1a1a1a;
}

.tab-btn.active {
    color: #0078d4;
    font-weight: 600;
    background-color: rgba(0,120,212,0.08);
}

/* ── Search entry ── */
entry.search {
    background-color: #f3f3f3;
    color: #1a1a1a;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    padding: 7px 12px;
    font-size: 13px;
    caret-color: #0078d4;
    font-family: "Segoe UI", "Inter", sans-serif;
}

entry.search:focus {
    background-color: #ffffff;
    border-color: #0078d4;
    box-shadow: 0 0 0 2px rgba(0,120,212,0.15);
}

/* ── Clipboard list ── */
.clipboard-list {
    background-color: transparent;
}

.clipboard-list row {
    background-color: #ffffff;
    border-radius: 6px;
    margin: 2px 8px;
    border: 1px solid transparent;
}

.clipboard-list row:hover {
    background-color: #f5f5f5;
    border-color: #e8e8e8;
}

.clipboard-list row:selected {
    background-color: #dde8f8;
    border-color: #0078d4;
}

.clipboard-list row:selected .clipboard-item-label {
    color: #003d82;
}

.clipboard-item-label {
    font-size: 13px;
    color: #1a1a1a;
    font-family: "Segoe UI", "Inter", sans-serif;
}

.clipboard-item-time {
    font-size: 11px;
    color: #9e9e9e;
}

/* ── Emoji picker ── */
.emoji-btn {
    font-size: 22px;
    min-width: 44px;
    min-height: 44px;
    padding: 4px;
    border-radius: 6px;
    border: none;
    background: transparent;
}

.emoji-btn:hover {
    background-color: rgba(0,120,212,0.08);
}

.emoji-btn:active {
    background-color: rgba(0,120,212,0.18);
}

/* ── Shortcuts tab ── */
.shortcut-list {
    background-color: transparent;
}

.shortcut-list row {
    background-color: #ffffff;
    border-radius: 6px;
    margin: 2px 8px;
    border: 1px solid transparent;
}

.shortcut-list row:hover {
    background-color: #f5f5f5;
    border-color: #e8e8e8;
}

.shortcut-name {
    font-size: 13px;
    font-weight: 600;
    color: #1a1a1a;
    font-family: "Segoe UI", "Inter", sans-serif;
}

.shortcut-cmd {
    font-size: 11px;
    color: #757575;
    font-family: "Cascadia Code", "JetBrains Mono", monospace;
}

.shortcut-run-btn {
    font-size: 12px;
    color: #107c10;
    background-color: rgba(16,124,16,0.08);
    border: 1px solid rgba(16,124,16,0.25);
    border-radius: 4px;
    padding: 2px 10px;
    min-height: 26px;
}

.shortcut-run-btn:hover {
    background-color: rgba(16,124,16,0.15);
}

.shortcut-del-btn {
    font-size: 12px;
    color: #c50f1f;
    background-color: rgba(197,15,31,0.06);
    border: 1px solid rgba(197,15,31,0.2);
    border-radius: 4px;
    padding: 2px 8px;
    min-height: 26px;
}

.shortcut-del-btn:hover {
    background-color: rgba(197,15,31,0.14);
}

.add-shortcut-toggle {
    font-size: 12px;
    font-weight: 500;
    color: #0078d4;
    background: transparent;
    border: 1px dashed rgba(0,120,212,0.4);
    border-radius: 6px;
    padding: 5px 14px;
}

.add-shortcut-toggle:hover {
    background-color: rgba(0,120,212,0.06);
}

/* ── Tools tab ── */
.tools-title {
    font-size: 13px;
    font-weight: 600;
    color: #616161;
    font-family: "Segoe UI", "Inter", sans-serif;
}

.tool-btn {
    font-size: 13px;
    font-weight: 500;
    color: #1a1a1a;
    background-color: #f5f5f5;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    padding: 12px 16px;
    min-height: 56px;
    font-family: "Segoe UI", "Inter", sans-serif;
}

.tool-btn:hover {
    background-color: #e8f0fe;
    border-color: #0078d4;
    color: #0078d4;
}

.tool-btn:active {
    background-color: #dde8f8;
}

/* ── Shared status label ── */
.status-label {
    font-size: 12px;
    color: #616161;
    font-family: "Cascadia Code", "JetBrains Mono", monospace;
}

/* ── Update progress bar ── */
.update-bar {
    background-color: #fffbe6;
    border-top: 1px solid #f0e68c;
    border-radius: 0 0 10px 10px;
}

.update-bar-label {
    font-size: 12px;
    color: #6b5900;
    font-family: "Segoe UI", "Inter", sans-serif;
    font-weight: 500;
}

/* ── Dim / empty state ── */
.dim-label {
    font-size: 13px;
    color: #bdbdbd;
    font-family: "Segoe UI", "Inter", sans-serif;
}

/* ── General entry ── */
entry {
    background-color: #f5f5f5;
    color: #1a1a1a;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    padding: 6px 10px;
    caret-color: #0078d4;
    font-family: "Segoe UI", "Inter", sans-serif;
}

entry:focus {
    background-color: #ffffff;
    border-color: #0078d4;
    box-shadow: 0 0 0 2px rgba(0,120,212,0.15);
}

/* ── Suggested-action button (Save) ── */
button.suggested-action {
    background-color: #0078d4;
    color: #ffffff;
    border-radius: 6px;
    font-weight: 600;
    padding: 4px 18px;
    border: none;
}

button.suggested-action:hover {
    background-color: #106ebe;
}

button.suggested-action:active {
    background-color: #005a9e;
}
"#;
