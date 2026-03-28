// ClipDeck — fast, native clipboard manager for Linux (GTK4 + Libadwaita)

mod config;
mod core;
mod data;
mod services;
mod system;
mod ui;

use std::sync::{Arc, Mutex};

use gtk4::prelude::*;
use libadwaita::prelude::*;

use config::Settings;
use core::clipboard::{AppMessage, ClipboardMonitor};
use core::Storage;
use system::hotkeys::start_hotkey_listener;
use ui::ClipDeckWindow;

fn main() {
    // ── Logging ───────────────────────────────────────────────────────────────
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!(
        "ClipDeck {} starting",
        env!("CARGO_PKG_VERSION")
    );

    // ── Settings ──────────────────────────────────────────────────────────────
    let settings = Arc::new(Settings::load());
    log::debug!("Settings loaded: {:?}", settings);

    // ── Storage ───────────────────────────────────────────────────────────────
    let storage = match Storage::open() {
        Ok(db) => Arc::new(Mutex::new(db)),
        Err(e) => {
            eprintln!("FATAL: Could not open database: {e}");
            std::process::exit(1);
        }
    };

    log::info!("Database at {:?}", Storage::db_path());

    // ── GTK / Libadwaita application ──────────────────────────────────────────
    let app = libadwaita::Application::builder()
        .application_id("dev.clipdeck")
        .flags(gtk4::gio::ApplicationFlags::FLAGS_NONE)
        .build();

    // Clone for move into activate closure.
    let settings_clone = settings.clone();
    let storage_clone = storage.clone();

    app.connect_activate(move |app| {
        build_ui(app, settings_clone.clone(), storage_clone.clone());
    });

    app.run();
}

fn build_ui(
    app: &libadwaita::Application,  // libadwaita::Application is IsA<gtk4::Application>
    settings: Arc<Settings>,
    storage: Arc<Mutex<Storage>>,
) {
    // ── Create popup window (hidden initially) ────────────────────────────────
    let deck = Arc::new(ClipDeckWindow::new(app, storage.clone()));

    // ── Cross-thread channel (background → GTK main loop) ─────────────────────
    let (sender, receiver) =
        gtk4::glib::MainContext::channel::<AppMessage>(gtk4::glib::Priority::DEFAULT);

    // ── Attach receiver to GTK main loop ─────────────────────────────────────
    {
        let deck_clone = deck.clone();
        receiver.attach(None, move |msg| {
            handle_message(msg, &deck_clone);
            gtk4::glib::ControlFlow::Continue
        });
    }

    // ── Spawn clipboard monitor ───────────────────────────────────────────────
    ClipboardMonitor::start(settings.clone(), storage.clone(), sender.clone());

    // ── Spawn X11 hotkey listener ─────────────────────────────────────────────
    start_hotkey_listener(sender.clone());

    // ── Async: version check (fire-and-forget) ────────────────────────────────
    {
        let repo = settings.github_repo.clone();
        let version = settings.version.clone();
        let sender_vc = sender.clone();

        // Spawn a Tokio runtime just for the one-shot HTTP call.
        std::thread::Builder::new()
            .name("version-check".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();

                rt.block_on(async move {
                    match services::updater::check_for_update(&repo, &version).await {
                        Ok(Some(new_ver)) => {
                            log::info!("New version available: {new_ver}");
                            let _ = sender_vc
                                .send(AppMessage::NewVersionAvailable(new_ver));
                        }
                        Ok(None) => log::info!("ClipDeck is up to date"),
                        Err(e) => log::debug!("Version check failed: {e}"),
                    }
                });
            })
            .expect("version-check thread");
    }

    log::info!("ClipDeck ready — press Super+V to open");
}

// ── Message dispatcher ────────────────────────────────────────────────────────

fn handle_message(msg: AppMessage, deck: &ClipDeckWindow) {
    match msg {
        AppMessage::NewClipboardItem(item) => {
            log::debug!("New clipboard item: {}", item.preview());
            // If the window is open, prepend the item live.
            if deck.window.is_visible() {
                deck.prepend_item(item);
            }
            // If hidden we just let the next show() reload from DB.
        }

        AppMessage::ToggleWindow => {
            log::debug!("ToggleWindow received");
            deck.toggle();
        }

        AppMessage::NewVersionAvailable(ver) => {
            log::info!("Update available: {ver}");
            // TODO Phase 2: show an in-app banner
        }
    }
}
