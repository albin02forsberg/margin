//! System tray: what's being tracked, and the timer actions, without the window.
//! Actions are run by the frontend (they may ask for a project or a note), so
//! the menu just shows the window when needed and forwards the action name.

use crate::timeclock::{format_hm, now};
use crate::App;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Emitter, Manager, Wry};

pub struct Tray {
    icon: TrayIcon<Wry>,
    status: MenuItem<Wry>,
    start: MenuItem<Wry>,
    pause: MenuItem<Wry>,
    resume: MenuItem<Wry>,
    switch: MenuItem<Wry>,
    stop: MenuItem<Wry>,
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let item = |id: &str, text: &str| MenuItem::with_id(app, id, text, true, None::<&str>);
    let status = MenuItem::with_id(app, "status", "", false, None::<&str>)?;
    let (start, pause, resume) = (item("start", "Start tracking…")?, item("pause", "Take a break")?, item("resume", "Resume")?);
    let (switch, stop) = (item("switch", "Switch project…")?, item("stop", "Stop…")?);
    let sep = || PredefinedMenuItem::separator(app);
    let menu = Menu::with_items(
        app,
        &[&status, &sep()?, &start, &pause, &resume, &switch, &stop, &sep()?, &item("show", "Show Margin")?, &item("quit", "Quit")?],
    )?;
    let icon = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "show" => show(app),
            "quit" => app.exit(0),
            id => {
                if id != "pause" && id != "resume" {
                    show(app); // these ask for a project or a note
                }
                let _ = app.emit("tray", id);
            }
        })
        .build(app)?;
    app.manage(Tray { icon, status, start, pause, resume, switch, stop });
    refresh(app);
    let h = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(30));
        refresh(&h);
    });
    Ok(())
}

pub fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = (w.show(), w.unminimize(), w.set_focus());
    }
}

/// Is a timer running (or paused for a break)?
pub fn tracking(app: &AppHandle) -> bool {
    let (cur, on_break, _) = app.state::<App>().tc().status();
    cur.is_some() || on_break.is_some()
}

/// Bring the tray in line with the time log; called on a timer and on file changes.
pub fn refresh(app: &AppHandle) {
    let Some(t) = app.try_state::<Tray>() else { return };
    let (cur, on_break, today) = app.state::<App>().tc().status();
    let label = match (&cur, &on_break) {
        (Some((since, project, _)), _) => Some(format!("{project} · {}", format_hm((now() - *since).num_seconds() as f64 / 3600.0))),
        (None, Some(project)) => Some(format!("{project} · on a break")),
        _ => None,
    };
    let status = format!("{} — today {}", label.as_deref().unwrap_or("Not tracking"), format_hm(today));
    let _ = t.status.set_text(&status);
    let _ = t.icon.set_tooltip(Some(&status));
    let _ = t.icon.set_title(label.as_deref()); // shown next to the icon on macOS and Linux
    let _ = t.start.set_enabled(cur.is_none() && on_break.is_none());
    let _ = t.pause.set_enabled(cur.is_some());
    let _ = t.resume.set_enabled(on_break.is_some());
    let _ = t.switch.set_enabled(cur.is_some());
    let _ = t.stop.set_enabled(cur.is_some());
}
