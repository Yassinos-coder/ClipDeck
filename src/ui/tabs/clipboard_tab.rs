//! Clipboard history tab — the main Phase-1 UI component.
//!
//! Layout:
//!   ┌─────────────────────────────┐
//!   │ 🔍 Search clipboard…        │  ← SearchEntry
//!   ├─────────────────────────────┤
//!   │ ▶ Hello World               │  ← ListBox rows
//!   │   git commit -m "fix bug"   │
//!   │   https://example.com       │
//!   └─────────────────────────────┘

use std::cell::RefCell;
use std::rc::Rc;

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use gtk4::prelude::*;
use gtk4::{
    Box as GBox, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, SearchEntry,
    SelectionMode,
};

use crate::core::ClipboardItem;

/// Internal state shared between closures.
struct TabState {
    /// All items in insertion order (newest first).
    all_items: Vec<ClipboardItem>,
    /// The subset currently rendered in `list_box`, in display order.
    visible_items: Vec<ClipboardItem>,
}

impl TabState {
    fn new() -> Self {
        Self {
            all_items: Vec::new(),
            visible_items: Vec::new(),
        }
    }
}

/// Holds widget handles so the caller can drive updates from the outside.
pub struct ClipboardTab {
    /// The root container to place in the main window.
    pub root: GBox,
    state: Rc<RefCell<TabState>>,
    list_box: ListBox,
    search: SearchEntry,
}

impl ClipboardTab {
    pub fn new() -> Self {
        let root = GBox::new(Orientation::Vertical, 0);
        root.set_vexpand(true);

        // ── Search bar ────────────────────────────────────────────────────────
        let search = SearchEntry::builder()
            .placeholder_text("Search clipboard…")
            .margin_top(12)
            .margin_bottom(8)
            .margin_start(12)
            .margin_end(12)
            .build();

        root.append(&search);

        // ── Scrollable list ───────────────────────────────────────────────────
        let scrolled = ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .build();

        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::Single);
        list_box.add_css_class("clipboard-list");

        scrolled.set_child(Some(&list_box));
        root.append(&scrolled);

        let state = Rc::new(RefCell::new(TabState::new()));

        // ── Wire search → filter ──────────────────────────────────────────────
        {
            let list_box_clone = list_box.clone();
            let state_clone = state.clone();
            search.connect_search_changed(move |entry| {
                let query = entry.text().to_string();
                let mut s = state_clone.borrow_mut();
                let all_items = s.all_items.clone();
                rebuild_list(&list_box_clone, &all_items, &query, &mut s.visible_items);
            });
        }

        Self {
            root,
            state,
            list_box,
            search,
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Load (or reload) the full history, replacing any existing rows.
    pub fn load_history(&self, history: Vec<ClipboardItem>) {
        let mut s = self.state.borrow_mut();
        s.all_items = history;
        let query = self.search.text().to_string();
        let all_items = s.all_items.clone();
        rebuild_list(&self.list_box, &all_items, &query, &mut s.visible_items);
    }

    /// Prepend a freshly captured item to the top of the list.
    pub fn prepend_item(&self, item: ClipboardItem) {
        let mut s = self.state.borrow_mut();
        // Remove duplicate (same content) if it exists.
        s.all_items.retain(|i| i.content != item.content);
        s.all_items.insert(0, item);
        let query = self.search.text().to_string();
        let all_items = s.all_items.clone();
        rebuild_list(&self.list_box, &all_items, &query, &mut s.visible_items);
    }

    /// Connect a callback fired when the user activates a row (Enter / double-click).
    pub fn connect_item_activated<F>(&self, callback: F)
    where
        F: Fn(ClipboardItem) + 'static,
    {
        let state = self.state.clone();
        self.list_box.connect_row_activated(move |_, row| {
            let idx = row.index() as usize;
            if let Some(item) = state.borrow().visible_items.get(idx) {
                callback(item.clone());
            }
        });
    }

    /// Move keyboard focus into the search entry.
    pub fn focus_search(&self) {
        self.search.grab_focus();
    }

    /// Clear search text and reset the list to full history.
    pub fn reset_search(&self) {
        self.search.set_text("");
        let mut s = self.state.borrow_mut();
        let all_items = s.all_items.clone();
        rebuild_list(&self.list_box, &all_items, "", &mut s.visible_items);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Repopulate `list_box` with items that match `query` (all items if empty),
/// and update `visible_items` to reflect the new display order so that
/// `row.index()` can be used to look up the correct item.
fn rebuild_list(
    list_box: &ListBox,
    all_items: &[ClipboardItem],
    query: &str,
    visible_items: &mut Vec<ClipboardItem>,
) {
    // Clear existing rows.
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
    visible_items.clear();

    let to_show: Vec<&ClipboardItem> = if query.is_empty() {
        all_items.iter().collect()
    } else {
        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(&ClipboardItem, i64)> = all_items
            .iter()
            .filter_map(|item| {
                matcher
                    .fuzzy_match(&item.content, query)
                    .map(|score| (item, score))
            })
            .collect();
        // Best matches first.
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().map(|(item, _)| item).collect()
    };

    for item in to_show {
        list_box.append(&make_row(item));
        visible_items.push(item.clone());
    }

    // Auto-select the first visible row.
    if let Some(row) = list_box.row_at_index(0) {
        list_box.select_row(Some(&row));
    }
}

/// Build a single `ListBoxRow` for a clipboard item.
fn make_row(item: &ClipboardItem) -> ListBoxRow {
    let row_box = GBox::new(Orientation::Horizontal, 8);
    row_box.set_margin_start(12);
    row_box.set_margin_end(12);
    row_box.set_margin_top(8);
    row_box.set_margin_bottom(8);

    // Pin indicator
    if item.pinned {
        let pin = Label::new(Some("📌"));
        row_box.append(&pin);
    }

    // Content preview
    let label = Label::new(Some(&item.preview()));
    label.set_hexpand(true);
    label.set_halign(gtk4::Align::Start);
    label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    label.add_css_class("clipboard-item-label");

    // Relative timestamp
    let ts = Label::new(Some(&item.relative_time()));
    ts.add_css_class("clipboard-item-time");
    ts.set_valign(gtk4::Align::Center);

    row_box.append(&label);
    row_box.append(&ts);

    let row = ListBoxRow::new();
    row.set_child(Some(&row_box));
    row
}
