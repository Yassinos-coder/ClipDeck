//! Global hotkey listener for X11.
//!
//! Uses `x11rb` to grab `Super + Alt + V` on the root window so that the
//! shortcut fires regardless of which application has focus.
//!
//! On Wayland without XWayland this module will fail gracefully and log a
//! warning — the user can still open ClipDeck by launching it from a
//! terminal or by mapping the keybinding in their compositor settings.

use async_channel::Sender;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, GrabMode, KeyPressEvent, ModMask};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;

use crate::core::clipboard::AppMessage;

// X11 keysym for the letter 'v' (lowercase)
const XK_V: u32 = 0x76;

/// Spawn the hotkey listener on a dedicated OS thread.
///
/// Sends `AppMessage::ToggleWindow` through `sender` whenever Super+Alt+V is
/// pressed.  Errors are logged but never propagated.
pub fn start_hotkey_listener(sender: Sender<AppMessage>) {
    std::thread::Builder::new()
        .name("hotkey-listener".into())
        .spawn(move || {
            if let Err(e) = run_listener(&sender) {
                log::warn!("Global hotkey listener stopped: {e}");
                log::warn!(
                    "Super+Alt+V hotkey is unavailable. \
                     You can map it manually in GNOME Settings → Keyboard → Custom Shortcuts \
                     with the command: clipdeck --show"
                );
            }
        })
        .expect("failed to spawn hotkey-listener thread");
}

fn run_listener(sender: &Sender<AppMessage>) -> anyhow::Result<()> {
    let (conn, screen_num) =
        RustConnection::connect(None).map_err(|e| anyhow::anyhow!("X11 connect: {e}"))?;

    let root = conn.setup().roots[screen_num].root;

    let keycode = keysym_to_keycode(&conn, XK_V)
        .ok_or_else(|| anyhow::anyhow!("keysym 0x{XK_V:x} not found in keyboard map"))?;

    log::debug!("Super+Alt+V → keycode {keycode}");

    // Grab with and without Num Lock (Mod2) / Caps Lock (Lock) variants so the
    // shortcut fires regardless of those modifier states.
    let super_key = ModMask::M4;
    let alt_key   = ModMask::M1;
    let num_lock  = ModMask::M2;
    let caps_lock = ModMask::LOCK;

    let base = super_key | alt_key;
    let variants = [
        base,
        base | num_lock,
        base | caps_lock,
        base | num_lock | caps_lock,
    ];

    for mods in variants {
        conn.grab_key(
            false,
            root,
            mods,
            keycode,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )?;
    }

    conn.flush()?;

    log::info!("Global hotkey Super+Alt+V registered on X11");

    loop {
        let event = conn.wait_for_event()?;
        if let Event::KeyPress(KeyPressEvent { .. }) = event {
            if sender.send_blocking(AppMessage::ToggleWindow).is_err() {
                break; // GTK loop exited
            }
        }
    }

    Ok(())
}

/// Translate an X11 keysym to a keycode by scanning the keyboard mapping.
fn keysym_to_keycode(conn: &impl Connection, keysym: u32) -> Option<u8> {
    let setup = conn.setup();
    let min_kc = setup.min_keycode;
    let count = setup.max_keycode - min_kc + 1;

    let mapping = conn
        .get_keyboard_mapping(min_kc, count)
        .ok()?
        .reply()
        .ok()?;

    let kspk = mapping.keysyms_per_keycode as usize;

    for (i, chunk) in mapping.keysyms.chunks(kspk).enumerate() {
        if chunk.iter().any(|&ks| ks == keysym) {
            return Some(min_kc + i as u8);
        }
    }

    None
}
