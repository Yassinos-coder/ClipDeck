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

// ── Stylesheet — "Signal" (Developer-First Light Theme) ──────────────────────
//
//  Philosophy: precision tool for power users.
//  • Fira Sans + Fira Mono — native to Pop!_OS / Ubuntu developer stack
//  • Left-bar selection indicator — VS Code lineage, immediately legible
//  • Underline-only tab active state — editorial, not pill-shaped
//  • Vivid #2563eb accent — bolder than generic blue, confident
//  • Clipboard content always rendered in monospace — code is at home

const APP_CSS: &str = r#"
/* ═══════════════════════════════════════════════════════════
   ClipDeck — "Signal" Design System
   Palette:
     white    #ffffff   surface
     off-white#fafafa   header / tab bar
     border   #eaeaea   dividers
     ink      #111827   primary text
     muted    #6b7280   secondary text
     ghost    #9ca3af   timestamps, hints
     accent   #2563eb   interactive blue
     accent+  #1d4ed8   pressed blue
     hover-bg #f3f4f6   subtle hover
     sel-bg   #eff6ff   selected background
     sel-bar  #2563eb   left selection indicator
     green    #15803d   run / success
     red      #dc2626   delete / error
     amber-bg #fffbeb   update bar
     amber-fg #92400e   update bar text
   ═══════════════════════════════════════════════════════════ */

/* ── Window shell ── */
.clipdeck-window {
    background-color: #ffffff;
    border-radius: 12px;
    border: 1px solid #d1d5db;
    box-shadow:
        0 0 0 1px rgba(0,0,0,0.04),
        0 4px 6px rgba(0,0,0,0.05),
        0 12px 40px rgba(0,0,0,0.14),
        0 24px 64px rgba(0,0,0,0.08);
}

/* ── Header ── */
.clipdeck-header {
    background-color: #fafafa;
    border-bottom: 1px solid #eaeaea;
    border-radius: 12px 12px 0 0;
}

.clipdeck-title {
    font-size: 15px;
    font-weight: 700;
    color: #111827;
    font-family: "Fira Sans", "Ubuntu", "Cantarell", sans-serif;
    letter-spacing: -0.2px;
}

.clipdeck-version {
    font-size: 10px;
    font-weight: 500;
    color: #9ca3af;
    font-family: "Fira Mono", "Ubuntu Mono", "DejaVu Sans Mono", monospace;
    letter-spacing: 0.3px;
}

/* ── Tab separator ── */
.tab-separator {
    background-color: #eaeaea;
    min-height: 1px;
}

/* ── Tab row ── */
.tab-row {
    background-color: #fafafa;
}

/* ── Tab buttons — underline style ── */
.tab-btn {
    font-size: 12px;
    font-weight: 500;
    color: #6b7280;
    padding: 6px 14px;
    border-radius: 0;
    border: none;
    border-bottom: 2px solid transparent;
    background: transparent;
    min-height: 34px;
    font-family: "Fira Sans", "Ubuntu", "Cantarell", sans-serif;
    transition: color 120ms, border-color 120ms;
}

.tab-btn:hover {
    color: #374151;
    background-color: rgba(0,0,0,0.03);
}

/* Active tab: vivid accent underline — the one memorable detail */
.tab-btn.active {
    color: #2563eb;
    font-weight: 600;
    border-bottom: 2px solid #2563eb;
    background-color: transparent;
}

/* ── Search entry ── */
entry.search {
    background-color: #f9fafb;
    color: #111827;
    border: 1.5px solid #e5e7eb;
    border-radius: 8px;
    padding: 8px 14px;
    font-size: 13px;
    font-family: "Fira Sans", "Ubuntu", "Cantarell", sans-serif;
    caret-color: #2563eb;
}

entry.search:focus {
    background-color: #ffffff;
    border-color: #2563eb;
    box-shadow: 0 0 0 3px rgba(37,99,235,0.12);
}

/* ── Clipboard list ── */
.clipboard-list {
    background-color: transparent;
}

.clipboard-list row {
    background-color: transparent;
    border-radius: 0;
    margin: 0;
    border: none;
    border-left: 3px solid transparent;
    transition: background-color 80ms, border-color 80ms;
}

.clipboard-list row:hover {
    background-color: #f3f4f6;
    border-left: 3px solid transparent;
}

/* The VS Code-lineage left bar — the signature interaction */
.clipboard-list row:selected {
    background-color: #eff6ff;
    border-left: 3px solid #2563eb;
}

.clipboard-list row:selected .clipboard-item-label {
    color: #1e40af;
}

/* Clipboard content in monospace — code, commands, URLs look native */
.clipboard-item-label {
    font-size: 12.5px;
    color: #1f2937;
    font-family: "Fira Mono", "Source Code Pro", "Ubuntu Mono",
                 "DejaVu Sans Mono", monospace;
    letter-spacing: -0.1px;
}

.clipboard-item-time {
    font-size: 10.5px;
    color: #9ca3af;
    font-family: "Fira Sans", "Ubuntu", "Cantarell", sans-serif;
    font-weight: 400;
}

/* ── Emoji picker ── */
.emoji-btn {
    font-size: 22px;
    min-width: 46px;
    min-height: 46px;
    padding: 6px;
    border-radius: 8px;
    border: none;
    background: transparent;
    transition: background-color 80ms;
}

.emoji-btn:hover {
    background-color: #eff6ff;
}

.emoji-btn:active {
    background-color: #dbeafe;
}

/* ── Shortcuts tab ── */
.shortcut-list {
    background-color: transparent;
}

.shortcut-list row {
    background-color: transparent;
    border-radius: 0;
    margin: 0;
    border: none;
    border-left: 3px solid transparent;
}

.shortcut-list row:hover {
    background-color: #f3f4f6;
}

.shortcut-name {
    font-size: 13px;
    font-weight: 600;
    color: #111827;
    font-family: "Fira Sans", "Ubuntu", "Cantarell", sans-serif;
}

.shortcut-cmd {
    font-size: 11.5px;
    color: #6b7280;
    font-family: "Fira Mono", "Ubuntu Mono", "DejaVu Sans Mono", monospace;
    letter-spacing: -0.1px;
}

.shortcut-run-btn {
    font-size: 11px;
    font-weight: 600;
    color: #15803d;
    background-color: #f0fdf4;
    border: 1px solid #bbf7d0;
    border-radius: 4px;
    padding: 2px 10px;
    min-height: 26px;
    font-family: "Fira Sans", "Ubuntu", sans-serif;
}

.shortcut-run-btn:hover {
    background-color: #dcfce7;
    border-color: #86efac;
}

.shortcut-del-btn {
    font-size: 11px;
    font-weight: 600;
    color: #dc2626;
    background-color: #fff5f5;
    border: 1px solid #fecaca;
    border-radius: 4px;
    padding: 2px 8px;
    min-height: 26px;
    font-family: "Fira Sans", "Ubuntu", sans-serif;
}

.shortcut-del-btn:hover {
    background-color: #fee2e2;
    border-color: #fca5a5;
}

.add-shortcut-toggle {
    font-size: 12px;
    font-weight: 500;
    color: #2563eb;
    background: transparent;
    border: 1.5px dashed #bfdbfe;
    border-radius: 8px;
    padding: 6px 16px;
    font-family: "Fira Sans", "Ubuntu", sans-serif;
}

.add-shortcut-toggle:hover {
    background-color: #eff6ff;
    border-color: #93c5fd;
}

/* ── Tools tab ── */
.tools-title {
    font-size: 11px;
    font-weight: 600;
    color: #9ca3af;
    font-family: "Fira Sans", "Ubuntu", "Cantarell", sans-serif;
    letter-spacing: 0.6px;
}

.tool-btn {
    font-size: 13px;
    font-weight: 500;
    color: #374151;
    background-color: #f9fafb;
    border: 1.5px solid #e5e7eb;
    border-radius: 10px;
    padding: 14px 16px;
    min-height: 58px;
    font-family: "Fira Sans", "Ubuntu", "Cantarell", sans-serif;
    transition: all 120ms;
}

.tool-btn:hover {
    background-color: #eff6ff;
    border-color: #93c5fd;
    color: #1d4ed8;
    box-shadow: 0 1px 4px rgba(37,99,235,0.10);
}

.tool-btn:active {
    background-color: #dbeafe;
    border-color: #2563eb;
}

/* ── Shared status / output label ── */
.status-label {
    font-size: 11.5px;
    color: #6b7280;
    font-family: "Fira Mono", "Ubuntu Mono", "DejaVu Sans Mono", monospace;
    letter-spacing: -0.1px;
}

/* ── Update progress bar — amber stripe at bottom ── */
.update-bar {
    background-color: #fffbeb;
    border-top: 1px solid #fde68a;
    border-radius: 0 0 12px 12px;
}

.update-bar-label {
    font-size: 12px;
    color: #92400e;
    font-family: "Fira Sans", "Ubuntu", sans-serif;
    font-weight: 500;
}

/* ── Empty / dim state ── */
.dim-label {
    font-size: 13px;
    color: #d1d5db;
    font-family: "Fira Sans", "Ubuntu", "Cantarell", sans-serif;
}

/* ── Generic entry (add-shortcut form fields) ── */
entry {
    background-color: #f9fafb;
    color: #111827;
    border: 1.5px solid #e5e7eb;
    border-radius: 6px;
    padding: 7px 11px;
    caret-color: #2563eb;
    font-family: "Fira Sans", "Ubuntu", "Cantarell", sans-serif;
    font-size: 13px;
}

entry:focus {
    background-color: #ffffff;
    border-color: #2563eb;
    box-shadow: 0 0 0 3px rgba(37,99,235,0.12);
}

/* ── Save / confirm buttons ── */
button.suggested-action {
    background-color: #2563eb;
    color: #ffffff;
    border-radius: 6px;
    font-weight: 600;
    font-size: 13px;
    padding: 5px 20px;
    border: none;
    font-family: "Fira Sans", "Ubuntu", sans-serif;
}

button.suggested-action:hover {
    background-color: #1d4ed8;
}

button.suggested-action:active {
    background-color: #1e40af;
}
"#;
