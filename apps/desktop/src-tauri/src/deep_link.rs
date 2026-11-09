//! Hands a link code from the website to the window. A code waits in state until the window
//! asks for it, and an event tells an open window at once.

use crate::link;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

/// Event sent to the window with the code of a new link request.
pub const EVENT: &str = "silicium-link";

/// The last link request the window has not taken yet.
#[derive(Default)]
pub struct PendingLink(Mutex<Option<String>>);

/// Takes a link request raised by a URL (launch argument, deep link or second instance). Other
/// URLs are ignored.
pub fn offer(app: &AppHandle, raw: &str) {
    let Some(code) = link::code_from_url(raw) else {
        return;
    };
    if let Ok(mut pending) = app.state::<PendingLink>().0.lock() {
        *pending = Some(code.clone());
    }
    let _ = app.emit(EVENT, code);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// The waiting link request, if any. Taking it clears it.
#[tauri::command]
pub fn take_pending_link(state: State<'_, PendingLink>) -> Option<String> {
    state.0.lock().ok().and_then(|mut pending| pending.take())
}
