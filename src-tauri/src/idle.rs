//! Idle detection: coming back after `idle_threshold_minutes` away from a running
//! timer shows the window and asks (in the frontend) what to do with that time.

use crate::timeclock::now;
use crate::{tray, App};
use chrono::{Duration, NaiveDateTime};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use user_idle::UserIdle;

/// One poll. AWAY is when the current idle stretch began, once it passed THRESHOLD;
/// LAST is the previous poll, so a longer gap (the machine slept) counts as away too.
/// Returns the new AWAY and, when the user just came back, (idle start, return time).
pub fn step(away: Option<NaiveDateTime>, last: NaiveDateTime, idle: Duration, threshold: Duration, running: bool, now: NaiveDateTime) -> (Option<NaiveDateTime>, Option<(NaiveDateTime, NaiveDateTime)>) {
    if !running {
        return (None, None);
    }
    let input = now - idle; // last keyboard/mouse input
    let away = away.or((now - last >= threshold).then_some(last)).or((idle >= threshold).then_some(input));
    match away {
        Some(a) if idle < threshold => (None, Some((a, input))),
        a => (a, None),
    }
}

/// How long since the last input, if this desktop can tell: X11, macOS and Windows
/// through user-idle, Wayland through ext-idle-notify (threshold fixed at startup).
pub(crate) fn source(threshold: Duration) -> Option<Box<dyn Fn() -> Duration + Send>> {
    let user_idle = || Box::new(|| Duration::seconds(UserIdle::get_time().map_or(0, |i| i.as_seconds() as i64))) as Box<dyn Fn() -> Duration + Send>;
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return crate::wayland_idle::start(threshold).map(|f| Box::new(f) as _).map_err(|e| eprintln!("idle time unavailable ({e}); only catching sleep")).ok();
    } else if std::env::var_os("DISPLAY").is_none() {
        return None; // user-idle's X11 backend crashes without a display
    }
    let _ = threshold;
    UserIdle::get_time().map_err(|e| eprintln!("idle time unavailable ({e:?}); only catching sleep")).ok().map(|_| user_idle())
}

/// Poll every 5 s. Without an idle source only sleep and suspend are caught, from the gap between polls.
pub fn start(app: &AppHandle) {
    let mins = app.state::<App>().cfg().idle_threshold_minutes;
    let source = if mins > 0 { source(Duration::minutes(mins)) } else { None };
    let app = app.clone();
    std::thread::spawn(move || {
        let (mut away, mut last) = (None, now());
        loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let s = app.state::<App>();
            let mins = s.cfg().idle_threshold_minutes;
            let idle = source.as_ref().map_or(Duration::zero(), |f| f());
            let t = now();
            let running = mins > 0 && s.tc().current().is_some();
            let (a, back) = step(away, last, idle, Duration::minutes(mins), running, t);
            (away, last) = (a, t);
            if let Some((since, back)) = back {
                tray::show(&app);
                let _ = app.emit("idle", json!({ "since": since, "back": back }));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_steps() {
        let t = |m: i64| NaiveDateTime::parse_from_str("2026-10-01 09:00", "%Y-%m-%d %H:%M").unwrap() + Duration::minutes(m);
        let (m, th) = (Duration::minutes, Duration::minutes(10));
        // Idle 5 min: not away yet. Idle 12 min: away since 9:00.
        assert_eq!(step(None, t(4), m(5), th, true, t(5)), (None, None));
        assert_eq!(step(None, t(11), m(12), th, true, t(12)), (Some(t(0)), None));
        assert_eq!(step(Some(t(0)), t(29), m(30), th, true, t(30)), (Some(t(0)), None));
        // Back at 9:31 (input 1 min ago): prompt for 9:00–9:31.
        assert_eq!(step(Some(t(0)), t(30), m(1), th, true, t(32)), (None, Some((t(0), t(31)))));
        // Timer stopped or on a break: forget it.
        assert_eq!(step(Some(t(0)), t(30), m(1), th, false, t(32)), (None, None));
        // Woke from sleep: the gap since the last poll is the time away.
        assert_eq!(step(None, t(0), m(0), th, true, t(60)), (None, Some((t(0), t(60)))));
        assert_eq!(step(None, t(0), m(0), th, true, t(1)), (None, None));
    }
}
