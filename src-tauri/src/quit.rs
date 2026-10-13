//! Quit handshake (#281): a real quit (tray Quit, closing the window when it doesn't hide to the tray,
//! any other `app.exit`) first asks the frontend, which saves and asks about unsaved edits that clash
//! with the disk, then answers with `quit_reply`. An update's restart passes straight through.

use std::sync::atomic::{AtomicU32, AtomicU8, Ordering::SeqCst};
use tauri::{AppHandle, Emitter, RESTART_EXIT_CODE};

const IDLE: u8 = 0;
/// `quit-requested` sent, no answer yet.
const ASKED: u8 = 1;
/// The frontend is asking the user (no timeout).
const BUSY: u8 = 2;
/// Confirmed: the next exit request goes through.
const GO: u8 = 3;

static STATE: AtomicU8 = AtomicU8::new(IDLE);
/// Which request a timeout belongs to, so an old one can't end a later request.
static SEQ: AtomicU32 = AtomicU32::new(0);

pub fn confirmed() -> bool {
    STATE.load(SeqCst) == GO
}

/// May an exit request with CODE go ahead? A confirmed quit, an update's restart (Tauri ignores
/// `prevent_exit` for it anyway) and the last window already gone (no webview left to ask) can.
pub fn exit_allowed(code: Option<i32>, confirmed: bool) -> bool {
    confirmed || code.is_none() || code == Some(RESTART_EXIT_CODE)
}

/// Ask the frontend before quitting; exits anyway if it doesn't acknowledge within 10 s, so quit can't hang.
pub fn ask(app: &AppHandle) {
    match STATE.load(SeqCst) {
        GO => return app.exit(0),
        ASKED | BUSY => return, // already asking
        _ => {}
    }
    let seq = SEQ.fetch_add(1, SeqCst) + 1;
    STATE.store(ASKED, SeqCst);
    let _ = app.emit("quit-requested", ());
    let h = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(10));
        if SEQ.load(SeqCst) == seq && STATE.compare_exchange(ASKED, GO, SeqCst, SeqCst).is_ok() {
            h.exit(0)
        }
    });
}

/// The frontend's answer to `quit-requested`: "wait" (it is saving and checking), "show" (it needs the
/// user: bring the window back from the tray), "go" (quit now) or "cancel".
#[tauri::command]
pub fn quit_reply(app: AppHandle, answer: &str) {
    match answer {
        "show" => crate::tray::show(&app),
        "wait" => {
            let _ = STATE.compare_exchange(ASKED, BUSY, SeqCst, SeqCst);
        }
        "go" => {
            STATE.store(GO, SeqCst);
            app.exit(0)
        }
        _ => STATE.store(IDLE, SeqCst),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_allowed_only_when_confirmed_restarting_or_windowless() {
        assert!(!exit_allowed(Some(0), false), "tray Quit / app.exit asks first");
        assert!(exit_allowed(Some(0), true), "confirmed quit goes through");
        assert!(exit_allowed(Some(RESTART_EXIT_CODE), false), "update restart is never held up");
        assert!(exit_allowed(None, false), "last window already destroyed: nothing left to ask");
    }
}
