//! Idle time under Wayland via the ext-idle-notify-v1 protocol (KWin, wlroots,
//! Mutter 46+): the compositor says when input has stopped for THRESHOLD.

use crate::timeclock::now;
use chrono::{Duration, NaiveDateTime};
use std::sync::{Arc, Mutex};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_registry::WlRegistry, wl_seat::WlSeat};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::{Event, ExtIdleNotificationV1};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1;

/// When the last input happened, while idle past the threshold.
type Since = Arc<Mutex<Option<NaiveDateTime>>>;

struct State {
    since: Since,
    threshold: Duration,
}

/// Start listening; the returned function gives the current idle time (zero until past THRESHOLD).
pub fn start(threshold: Duration) -> Result<impl Fn() -> Duration + Send, String> {
    let conn = Connection::connect_to_env().map_err(|e| e.to_string())?;
    let (globals, mut queue) = registry_queue_init::<State>(&conn).map_err(|e| e.to_string())?;
    let qh = queue.handle();
    let seat: WlSeat = globals.bind(&qh, 1..=1, ()).map_err(|e| e.to_string())?;
    let notifier: ExtIdleNotifierV1 = globals.bind(&qh, 1..=2, ()).map_err(|e| format!("compositor lacks ext-idle-notify: {e}"))?;
    let ms = threshold.num_milliseconds().clamp(1000, u32::MAX as i64) as u32;
    // v2 can ignore idle inhibitors (a playing video keeps the screen on, but nobody is working).
    if notifier.version() >= 2 {
        notifier.get_input_idle_notification(ms, &seat, &qh, ());
    } else {
        notifier.get_idle_notification(ms, &seat, &qh, ());
    }
    let since = Since::default();
    let mut state = State { since: since.clone(), threshold };
    std::thread::spawn(move || while queue.blocking_dispatch(&mut state).is_ok() {});
    Ok(move || since.lock().unwrap().map_or(Duration::zero(), |s| now() - s))
}

impl Dispatch<ExtIdleNotificationV1, ()> for State {
    fn event(s: &mut Self, _: &ExtIdleNotificationV1, ev: Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        *s.since.lock().unwrap() = match ev {
            Event::Idled => Some(now() - s.threshold),
            Event::Resumed => None,
            _ => return,
        };
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(_: &mut Self, _: &WlRegistry, _: <WlRegistry as Proxy>::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<WlSeat, ()> for State {
    fn event(_: &mut Self, _: &WlSeat, _: <WlSeat as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
impl Dispatch<ExtIdleNotifierV1, ()> for State {
    fn event(_: &mut Self, _: &ExtIdleNotifierV1, _: <ExtIdleNotifierV1 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

#[cfg(test)]
mod tests {
    /// Needs a Wayland session and 2 s without input: `cargo test -- --ignored wayland`.
    #[test]
    #[ignore]
    fn wayland_reports_idle() {
        let idle = super::start(chrono::Duration::seconds(1)).unwrap();
        let t = std::time::Instant::now();
        while idle().is_zero() {
            assert!(t.elapsed().as_secs() < 30, "never idle");
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        assert!(idle() >= chrono::Duration::seconds(1));
    }
}
