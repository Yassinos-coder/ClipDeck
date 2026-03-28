use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_channel::Sender;

use crate::config::Settings;
use crate::core::{ClipboardItem, Storage};

/// Message types sent from background threads to the GTK main loop.
#[derive(Debug, Clone)]
pub enum AppMessage {
    /// A new (or refreshed) clipboard item was captured.
    NewClipboardItem(ClipboardItem),
    /// Toggle window visibility (from global hotkey).
    ToggleWindow,
    /// A newer version is available and download has started automatically.
    NewVersionAvailable(String),
    /// Auto-update progress text shown in the window status bar.
    /// Empty string hides the bar.
    UpdateStatus(String),
}

/// Continuously polls the system clipboard and writes new entries to storage.
pub struct ClipboardMonitor;

impl ClipboardMonitor {
    /// Spawn the monitor on a background thread.
    ///
    /// The caller supplies an `async_channel::Sender<AppMessage>` so that discovered
    /// items can be forwarded to the GTK main loop safely.
    pub fn start(
        settings: Arc<Settings>,
        storage: Arc<Mutex<Storage>>,
        sender: Sender<AppMessage>,
    ) {
        std::thread::Builder::new()
            .name("clipboard-monitor".into())
            .spawn(move || run_monitor(settings, storage, sender))
            .expect("failed to spawn clipboard-monitor thread");
    }
}

fn run_monitor(
    settings: Arc<Settings>,
    storage: Arc<Mutex<Storage>>,
    sender: Sender<AppMessage>,
) {
    let mut clipboard = match arboard::Clipboard::new() {
        Ok(cb) => cb,
        Err(e) => {
            log::error!("Clipboard init failed: {e}");
            return;
        }
    };

    let interval = Duration::from_millis(settings.poll_interval_ms);
    let mut last_content = String::new();

    log::info!(
        "Clipboard monitor started (poll every {}ms)",
        settings.poll_interval_ms
    );

    loop {
        std::thread::sleep(interval);

        let text = match clipboard.get_text() {
            Ok(t) => t,
            Err(_) => continue, // clipboard empty or app didn't respond
        };

        // Skip empty, unchanged, over-sized, or sensitive content.
        if text.is_empty() || text == last_content {
            continue;
        }
        if text.len() > settings.max_entry_size {
            log::debug!("Skipping oversized clipboard entry ({} bytes)", text.len());
            continue;
        }
        if settings.is_sensitive(&text) {
            log::debug!("Skipping sensitive clipboard entry");
            continue;
        }

        last_content = text.clone();

        let item = ClipboardItem {
            id: 0,
            content: text,
            timestamp: chrono::Utc::now().timestamp(),
            pinned: false,
        };

        let stored = {
            let mut db = match storage.lock() {
                Ok(g) => g,
                Err(e) => {
                    log::error!("Storage lock poisoned: {e}");
                    continue;
                }
            };

            match db.insert_item(&item) {
                Ok(id) => {
                    // Evict old entries while we hold the lock.
                    let _ = db.evict_old(settings.max_history);
                    ClipboardItem { id, ..item }
                }
                Err(e) => {
                    log::error!("Failed to store clipboard item: {e}");
                    continue;
                }
            }
        };

        if sender.send_blocking(AppMessage::NewClipboardItem(stored)).is_err() {
            // GTK main loop has exited — time to stop.
            log::info!("GTK channel closed, stopping clipboard monitor");
            break;
        }
    }
}

/// Write `text` to the system clipboard from any thread.
pub fn write_to_clipboard(text: &str) -> anyhow::Result<()> {
    let mut cb = arboard::Clipboard::new()?;
    cb.set_text(text)?;
    Ok(())
}
