//! Emoji picker tab.
//!
//! Layout:
//!   ┌──────────────────────────────────────┐
//!   │ 🔍 Search emoji…                     │  ← SearchEntry
//!   ├──────────────────────────────────────┤
//!   │ 😀 😃 😄 😁 😆 😅 🤣 😂 🙂 🙃    │  ← FlowBox (10 per row)
//!   │ 😉 😊 😇 🥰 😍 🤩 😘 😗 😚 😙    │
//!   │ …                                    │
//!   └──────────────────────────────────────┘

use gtk4::prelude::*;
use gtk4::{
    Box as GBox, Button, FlowBox, FlowBoxChild, Orientation, ScrolledWindow, SearchEntry,
    SelectionMode,
};

use crate::core::clipboard::write_to_clipboard;
use crate::data::emoji::EMOJI_DATA;

/// The emoji picker tab.  `root` is placed directly into the tab stack.
pub struct EmojiTab {
    pub root: GBox,
    search: SearchEntry,
}

impl EmojiTab {
    /// Create the emoji tab.
    ///
    /// `close_window` is called after the user picks an emoji so the popup
    /// disappears immediately.
    pub fn new(close_window: impl Fn() + 'static + Clone) -> Self {
        let root = GBox::new(Orientation::Vertical, 0);
        root.set_vexpand(true);

        // ── Search bar ────────────────────────────────────────────────────────
        let search = SearchEntry::builder()
            .placeholder_text("Search emoji…")
            .margin_top(10)
            .margin_bottom(6)
            .margin_start(12)
            .margin_end(12)
            .build();
        root.append(&search);

        // ── Scrollable FlowBox ────────────────────────────────────────────────
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .build();

        let flow_box = FlowBox::builder()
            .selection_mode(SelectionMode::None)
            .max_children_per_line(10)
            .min_children_per_line(5)
            .homogeneous(true)
            .row_spacing(2)
            .column_spacing(2)
            .margin_start(8)
            .margin_end(8)
            .margin_top(4)
            .margin_bottom(8)
            .build();

        scrolled.set_child(Some(&flow_box));
        root.append(&scrolled);

        // Populate all emoji on first render.
        populate_flow_box(&flow_box, EMOJI_DATA, &close_window);

        // ── Search → filter ───────────────────────────────────────────────────
        {
            let flow_box_clone = flow_box.clone();
            let cw = close_window.clone();
            search.connect_search_changed(move |entry| {
                let query = entry.text().to_string().to_lowercase();

                // Clear existing children.
                while let Some(child) = flow_box_clone.first_child() {
                    flow_box_clone.remove(&child);
                }

                if query.is_empty() {
                    populate_flow_box(&flow_box_clone, EMOJI_DATA, &cw);
                } else {
                    let filtered: Vec<(&str, &str, &str)> = EMOJI_DATA
                        .iter()
                        .filter(|(_, name, tags)| {
                            name.to_lowercase().contains(&query)
                                || tags.to_lowercase().contains(&query)
                        })
                        .copied()
                        .collect();
                    populate_flow_box(&flow_box_clone, &filtered, &cw);
                }
            });
        }

        Self { root, search }
    }

    /// Move keyboard focus into the search entry.
    pub fn focus_search(&self) {
        self.search.grab_focus();
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Populate `flow_box` with one button per emoji entry.
fn populate_flow_box(
    flow_box: &FlowBox,
    entries: &[(&'static str, &'static str, &'static str)],
    close_window: &(impl Fn() + 'static + Clone),
) {
    for &(glyph, name, _tags) in entries {
        let btn = Button::builder().label(glyph).tooltip_text(name).build();
        btn.add_css_class("emoji-btn");

        let cw = close_window.clone();
        btn.connect_clicked(move |_| {
            match write_to_clipboard(glyph) {
                Ok(_) => log::debug!("Emoji copied: {glyph}"),
                Err(e) => log::error!("Failed to copy emoji: {e}"),
            }
            cw();
        });

        let child = FlowBoxChild::new();
        child.set_child(Some(&btn));
        flow_box.append(&child);
    }
}
