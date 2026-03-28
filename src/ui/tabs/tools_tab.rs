//! System tools tab.
//!
//! Layout:
//!   ┌──────────────────────────────────────────┐
//!   │  System Tools                            │  ← title
//!   ├──────────────────────────────────────────┤
//!   │  [Flush DNS Cache]  [Clear Clip History] │  ← 2×2 grid
//!   │  [Open Config Dir]  [Install Autostart]  │
//!   ├──────────────────────────────────────────┤
//!   │  Status: …                               │  ← status label
//!   └──────────────────────────────────────────┘

use std::sync::{Arc, Mutex};

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Box as GBox, Button, Grid, Label, Orientation};

use crate::core::Storage;
use crate::system::{autostart, commands};

/// System tools tab widget.
pub struct ToolsTab {
    pub root: GBox,
}

impl ToolsTab {
    pub fn new(storage: Arc<Mutex<Storage>>) -> Self {
        let root = GBox::new(Orientation::Vertical, 0);
        root.set_vexpand(true);

        // ── Title ─────────────────────────────────────────────────────────────
        let title = Label::new(Some("System Tools"));
        title.add_css_class("tools-title");
        title.set_halign(gtk4::Align::Start);
        title.set_margin_start(16);
        title.set_margin_top(16);
        title.set_margin_bottom(12);
        root.append(&title);

        // ── Status label (declared early so buttons can reference it) ─────────
        let status_label = Label::builder()
            .label("")
            .halign(gtk4::Align::Start)
            .wrap(true)
            .margin_start(16)
            .margin_end(16)
            .margin_top(12)
            .margin_bottom(16)
            .build();
        status_label.add_css_class("status-label");

        // ── 2×2 Grid of tool buttons ──────────────────────────────────────────
        let grid = Grid::builder()
            .row_spacing(12)
            .column_spacing(12)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(8)
            .build();

        // --- Tool 1: Flush DNS Cache ---
        let flush_btn = make_tool_btn("Flush DNS Cache", "Clears the systemd-resolved DNS cache");
        {
            let status = status_label.clone();
            flush_btn.connect_clicked(move |_| {
                let status2 = status.clone();
                status2.set_label("Flushing DNS cache…");

                std::thread::spawn(move || {
                    let msg = commands::flush_dns()
                        .unwrap_or_else(|e| format!("Error: {e}"));
                    glib::idle_add_local_once(move || {
                        status2.set_label(&msg);
                    });
                });
            });
        }
        grid.attach(&flush_btn, 0, 0, 1, 1);

        // --- Tool 2: Clear Clipboard History ---
        let clear_btn = make_tool_btn("Clear Clip History", "Deletes all non-pinned clipboard entries");
        {
            let status = status_label.clone();
            let storage_clear = storage.clone();
            clear_btn.connect_clicked(move |_| {
                let result = {
                    let mut db = storage_clear.lock().unwrap();
                    db.clear_history()
                };
                match result {
                    Ok(n) => status.set_label(&format!("Cleared {n} clipboard entries.")),
                    Err(e) => status.set_label(&format!("Error: {e}")),
                }
            });
        }
        grid.attach(&clear_btn, 1, 0, 1, 1);

        // --- Tool 3: Open Config Dir ---
        let config_btn = make_tool_btn("Open Config Dir", "Opens ~/.config/clipdeck in the file manager");
        {
            let status = status_label.clone();
            config_btn.connect_clicked(move |_| {
                let config_path = dirs::config_dir()
                    .map(|p| p.join("clipdeck").to_string_lossy().to_string())
                    .unwrap_or_else(|| "~/.config/clipdeck".to_string());

                let result = std::process::Command::new("xdg-open")
                    .arg(&config_path)
                    .spawn();

                match result {
                    Ok(_) => status.set_label(&format!("Opened: {config_path}")),
                    Err(e) => status.set_label(&format!("Error opening config dir: {e}")),
                }
            });
        }
        grid.attach(&config_btn, 0, 1, 1, 1);

        // --- Tool 4: Install Autostart ---
        let autostart_btn = make_tool_btn("Install Autostart", "Creates a systemd user service for ClipDeck");
        {
            let status = status_label.clone();
            autostart_btn.connect_clicked(move |_| {
                if autostart::is_installed() {
                    status.set_label("Autostart already installed.");
                    return;
                }

                // Find the current executable path.
                let binary = std::env::current_exe()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "clipdeck".to_string());

                let status2 = status.clone();
                std::thread::spawn(move || {
                    let msg = autostart::install_service(&binary)
                        .map(|_| "Autostart installed. Run: systemctl --user enable --now clipdeck.service".to_string())
                        .unwrap_or_else(|e| format!("Error: {e}"));
                    glib::idle_add_local_once(move || {
                        status2.set_label(&msg);
                    });
                });
            });
        }
        grid.attach(&autostart_btn, 1, 1, 1, 1);

        root.append(&grid);
        root.append(&status_label);

        // ── Support section ───────────────────────────────────────────────────
        let sep = gtk4::Separator::new(Orientation::Horizontal);
        sep.set_margin_start(16);
        sep.set_margin_end(16);
        sep.set_margin_top(8);
        root.append(&sep);

        let support_lbl = Label::builder()
            .label("If ClipDeck saves you time, consider supporting the project!")
            .halign(gtk4::Align::Start)
            .wrap(true)
            .margin_start(16)
            .margin_end(16)
            .margin_top(12)
            .margin_bottom(8)
            .build();
        support_lbl.add_css_class("support-label");
        root.append(&support_lbl);

        let donate_btn = Button::builder()
            .label("Donate via PayPal")
            .halign(gtk4::Align::Start)
            .margin_start(16)
            .margin_end(16)
            .margin_bottom(16)
            .build();
        donate_btn.add_css_class("donate-btn");
        donate_btn.connect_clicked(|_| {
            let _ = std::process::Command::new("xdg-open")
                .arg("https://paypal.me/the1290srider")
                .spawn();
        });
        root.append(&donate_btn);

        Self { root }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_tool_btn(label: &str, tooltip: &str) -> Button {
    let btn = Button::builder()
        .label(label)
        .tooltip_text(tooltip)
        .hexpand(true)
        .build();
    btn.add_css_class("tool-btn");
    btn
}
