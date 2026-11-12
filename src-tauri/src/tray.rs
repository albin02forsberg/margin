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
    last: MenuItem<Wry>,
    pause: MenuItem<Wry>,
    resume: MenuItem<Wry>,
    switch: MenuItem<Wry>,
    stop: MenuItem<Wry>,
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let item = |id: &str, text: &str| MenuItem::with_id(app, id, text, true, None::<&str>);
    let status = MenuItem::with_id(app, "status", "", false, None::<&str>)?;
    let (start, pause, resume) = (item("start", "Start tracking…")?, item("pause", "Take a break")?, item("resume", "Resume")?);
    let last = MenuItem::with_id(app, "last", last_item(None, false).0, false, None::<&str>)?;
    let (switch, stop) = (item("switch", "Switch project…")?, item("stop", "Stop…")?);
    let sep = || PredefinedMenuItem::separator(app);
    let menu = Menu::with_items(
        app,
        &[&status, &sep()?, &start, &last, &pause, &resume, &switch, &stop, &sep()?, &item("capture", "Quick capture…")?, &item("today", "Open Today")?, &item("show", "Show Margin")?, &item("quit", "Quit")?],
    )?;
    let icon = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, ev| match ev.id().as_ref() {
            "show" => show(app),
            "quit" => crate::quit::ask(app),
            "capture" => crate::capture(app),
            id => {
                if !matches!(id, "pause" | "resume" | "last") {
                    show(app); // these ask for a project or a note
                }
                let _ = app.emit("tray", id);
            }
        })
        .build(app)?;
    app.manage(Tray { icon, status, start, last, pause, resume, switch, stop });
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

/// Label and enabled state of the "resume last session" item: needs a stopped session and nothing running.
fn last_item(last: Option<&(String, String)>, idle: bool) -> (String, bool) {
    match last {
        Some((p, t)) if t.is_empty() => (format!("Resume {p}"), idle),
        Some((p, t)) => (format!("Resume {p} / {t}"), idle),
        None => ("Resume last session".into(), false),
    }
}

/// Bring the tray in line with the time log; called on a timer and on file changes.
pub fn refresh(app: &AppHandle) {
    let Some(t) = app.try_state::<Tray>() else { return };
    let tc = app.state::<App>().tc();
    let (cur, on_break, today) = tc.status();
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
    let (text, on) = last_item(tc.last_session().as_ref(), cur.is_none() && on_break.is_none());
    let _ = (t.last.set_text(text), t.last.set_enabled(on));
    let _ = t.pause.set_enabled(cur.is_some());
    let _ = t.resume.set_enabled(on_break.is_some());
    let _ = t.switch.set_enabled(cur.is_some());
    let _ = t.stop.set_enabled(cur.is_some());
}

#[cfg(test)]
mod tests {
    use super::last_item;

    #[test]
    fn last_label() {
        let s = |p: &str, t: &str| (p.to_string(), t.to_string());
        assert_eq!(last_item(Some(&s("Acme", "Fix")), true), ("Resume Acme / Fix".into(), true));
        assert_eq!(last_item(Some(&s("Acme", "")), true), ("Resume Acme".into(), true));
        assert!(!last_item(Some(&s("Acme", "Fix")), false).1);
        assert!(!last_item(None, true).1);
    }
}
