//! Shortcuts tab — lets users define and run named shell commands.
//!
//! Layout:
//!   ┌──────────────────────────────────────────────────────┐
//!   │ [name]  [command preview]  [▶ Run]  [✕]             │  ← ListBox rows
//!   │ …                                                    │
//!   ├──────────────────────────────────────────────────────┤
//!   │ ▾ Add new shortcut                                   │  ← Expander
//!   │   Name:    [_________________]                       │
//!   │   Command: [_________________]                       │
//!   │                           [Save]                     │
//!   ├──────────────────────────────────────────────────────┤
//!   │ Status: …                                            │  ← status label
//!   └──────────────────────────────────────────────────────┘

use std::sync::{Arc, Mutex};

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Box as GBox, Button, Entry, Label, ListBox, ListBoxRow, Orientation, Revealer, RevealerTransitionType,
    ScrolledWindow, SelectionMode,
};

use crate::core::{Shortcut, Storage};

/// Shortcuts tab widget.
pub struct ShortcutsTab {
    pub root: GBox,
    list_box: ListBox,
    status_label: Label,
    name_entry: Entry,
    cmd_entry: Entry,
    storage: Arc<Mutex<Storage>>,
}

impl ShortcutsTab {
    pub fn new(storage: Arc<Mutex<Storage>>) -> Self {
        let root = GBox::new(Orientation::Vertical, 0);
        root.set_vexpand(true);

        // ── Scrollable shortcut list ──────────────────────────────────────────
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .build();

        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::None);
        list_box.add_css_class("shortcut-list");
        scrolled.set_child(Some(&list_box));
        root.append(&scrolled);

        // ── "Add new shortcut" collapsible section ────────────────────────────
        let toggle_btn = Button::builder()
            .label("+ Add new shortcut")
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(0)
            .build();
        toggle_btn.add_css_class("add-shortcut-toggle");
        root.append(&toggle_btn);

        let form_box = GBox::new(Orientation::Vertical, 6);
        form_box.set_margin_start(12);
        form_box.set_margin_end(12);
        form_box.set_margin_top(8);
        form_box.set_margin_bottom(4);

        let name_row = GBox::new(Orientation::Horizontal, 8);
        let name_lbl = Label::new(Some("Name:"));
        name_lbl.set_width_chars(8);
        name_lbl.set_xalign(1.0);
        let name_entry = Entry::builder().placeholder_text("e.g. Update system").hexpand(true).build();
        name_row.append(&name_lbl);
        name_row.append(&name_entry);
        form_box.append(&name_row);

        let cmd_row = GBox::new(Orientation::Horizontal, 8);
        let cmd_lbl = Label::new(Some("Command:"));
        cmd_lbl.set_width_chars(8);
        cmd_lbl.set_xalign(1.0);
        let cmd_entry = Entry::builder().placeholder_text("e.g. sudo apt update && sudo apt upgrade -y").hexpand(true).build();
        cmd_row.append(&cmd_lbl);
        cmd_row.append(&cmd_entry);
        form_box.append(&cmd_row);

        let save_btn = Button::builder().label("Save").halign(gtk4::Align::End).build();
        save_btn.add_css_class("suggested-action");
        form_box.append(&save_btn);

        let revealer = Revealer::builder()
            .transition_type(RevealerTransitionType::SlideDown)
            .transition_duration(200)
            .reveal_child(false)
            .child(&form_box)
            .build();
        root.append(&revealer);

        // ── Status label ──────────────────────────────────────────────────────
        let status_label = Label::builder()
            .label("")
            .margin_start(12)
            .margin_end(12)
            .margin_top(6)
            .margin_bottom(8)
            .halign(gtk4::Align::Start)
            .wrap(true)
            .build();
        status_label.add_css_class("status-label");
        root.append(&status_label);

        // ── Wire toggle button ────────────────────────────────────────────────
        {
            let rev = revealer.clone();
            toggle_btn.connect_clicked(move |_| {
                rev.set_reveal_child(!rev.reveals_child());
            });
        }

        let tab = Self {
            root,
            list_box,
            status_label,
            name_entry,
            cmd_entry,
            storage: storage.clone(),
        };

        // Load initial data.
        tab.reload();

        // ── Wire save button ──────────────────────────────────────────────────
        {
            let storage_save = storage.clone();
            let name_entry_s = tab.name_entry.clone();
            let cmd_entry_s = tab.cmd_entry.clone();
            let list_box_s = tab.list_box.clone();
            let status_s = tab.status_label.clone();
            let revealer_s = revealer.clone();
            let storage_reload = storage.clone();

            save_btn.connect_clicked(move |_| {
                let name = name_entry_s.text().to_string();
                let cmd = cmd_entry_s.text().to_string();

                if name.trim().is_empty() || cmd.trim().is_empty() {
                    status_s.set_label("Name and command cannot be empty.");
                    return;
                }

                let result = {
                    let mut db = storage_save.lock().unwrap();
                    db.insert_shortcut(&name, &cmd)
                };

                match result {
                    Ok(_) => {
                        name_entry_s.set_text("");
                        cmd_entry_s.set_text("");
                        revealer_s.set_reveal_child(false);
                        status_s.set_label("Shortcut saved.");
                        reload_list(&list_box_s, &storage_reload, &status_s);
                    }
                    Err(e) => {
                        status_s.set_label(&format!("Error: {e}"));
                    }
                }
            });
        }

        tab
    }

    /// Reload the shortcut list from the database.
    pub fn reload(&self) {
        reload_list(&self.list_box, &self.storage, &self.status_label);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn reload_list(list_box: &ListBox, storage: &Arc<Mutex<Storage>>, status: &Label) {
    // Clear existing rows.
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let shortcuts = {
        let db = storage.lock().unwrap();
        db.get_shortcuts().unwrap_or_default()
    };

    if shortcuts.is_empty() {
        let empty = Label::builder()
            .label("No shortcuts yet. Add one below.")
            .margin_top(24)
            .margin_bottom(24)
            .build();
        empty.add_css_class("dim-label");
        let row = ListBoxRow::new();
        row.set_child(Some(&empty));
        row.set_activatable(false);
        list_box.append(&row);
        return;
    }

    for shortcut in shortcuts {
        let row = make_shortcut_row(&shortcut, storage, status, list_box);
        list_box.append(&row);
    }
}

fn make_shortcut_row(
    shortcut: &Shortcut,
    storage: &Arc<Mutex<Storage>>,
    status: &Label,
    list_box: &ListBox,
) -> ListBoxRow {
    let row_box = GBox::new(Orientation::Horizontal, 8);
    row_box.set_margin_start(12);
    row_box.set_margin_end(12);
    row_box.set_margin_top(6);
    row_box.set_margin_bottom(6);
    row_box.add_css_class("shortcut-row");

    // Name label
    let name_lbl = Label::new(Some(&shortcut.name));
    name_lbl.add_css_class("shortcut-name");
    name_lbl.set_width_chars(14);
    name_lbl.set_xalign(0.0);
    name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    row_box.append(&name_lbl);

    // Command preview
    let cmd_preview: String = if shortcut.command.chars().count() > 40 {
        format!("{}…", shortcut.command.chars().take(39).collect::<String>())
    } else {
        shortcut.command.clone()
    };
    let cmd_lbl = Label::new(Some(&cmd_preview));
    cmd_lbl.add_css_class("shortcut-cmd");
    cmd_lbl.set_hexpand(true);
    cmd_lbl.set_xalign(0.0);
    cmd_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    row_box.append(&cmd_lbl);

    // Run button
    let run_btn = Button::builder().label("▶ Run").build();
    run_btn.add_css_class("shortcut-run-btn");

    {
        let sc = shortcut.clone();
        let status_clone = status.clone();
        run_btn.connect_clicked(move |_| {
            let sc2 = sc.clone();
            let status2 = status_clone.clone();
            status2.set_label("Running…");

            std::thread::spawn(move || {
                let result = sc2.execute().unwrap_or_else(|e| format!("Error: {e}"));
                glib::idle_add_local_once(move || {
                    status2.set_label(&result);
                });
            });
        });
    }

    row_box.append(&run_btn);

    // Delete button
    let del_btn = Button::builder().label("✕").build();
    del_btn.add_css_class("shortcut-del-btn");

    {
        let id = shortcut.id;
        let storage_del = storage.clone();
        let status_del = status.clone();
        let list_box_del = list_box.clone();
        let storage_reload = storage.clone();
        del_btn.connect_clicked(move |_| {
            let result = {
                let mut db = storage_del.lock().unwrap();
                db.delete_shortcut(id)
            };
            match result {
                Ok(_) => {
                    status_del.set_label("Shortcut deleted.");
                    reload_list(&list_box_del, &storage_reload, &status_del);
                }
                Err(e) => {
                    status_del.set_label(&format!("Delete error: {e}"));
                }
            }
        });
    }

    row_box.append(&del_btn);

    let row = ListBoxRow::new();
    row.set_child(Some(&row_box));
    row.set_activatable(false);
    row
}
