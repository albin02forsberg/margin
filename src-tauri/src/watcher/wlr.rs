//! The focused window on wlroots compositors (Sway, River, labwc, Hyprland) via the
//! wlr-foreign-toplevel-management protocol: each poll connects, lists the toplevels and
//! returns the activated one.

use std::collections::HashMap;
use std::hash::Hash;
use wayland_client::backend::ObjectId;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{event_created_child, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::{self as handle, ZwlrForeignToplevelHandleV1};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::{self as manager, ZwlrForeignToplevelManagerV1};

#[derive(Default, Debug)]
pub struct Top {
    app: String,
    title: String,
    active: bool,
}

/// What a toplevel handle reports (the protocol's `done` batching doesn't matter at 5 s polls).
pub enum Ev {
    Title(String),
    App(String),
    /// The raw `state` array: native-endian u32s, 2 = activated.
    State(Vec<u8>),
    Closed,
}

/// Apply toplevel K's event EV to TOPS.
pub fn apply<K: Hash + Eq>(tops: &mut HashMap<K, Top>, k: K, ev: Ev) {
    if let Ev::Closed = ev {
        tops.remove(&k);
        return;
    }
    let t = tops.entry(k).or_default();
    match ev {
        Ev::Title(s) => t.title = s,
        Ev::App(s) => t.app = s,
        Ev::State(s) => t.active = s.as_chunks::<4>().0.iter().any(|c| u32::from_ne_bytes(*c) == handle::State::Activated as u32),
        Ev::Closed => {}
    }
}

/// The activated toplevel's app id and title.
pub fn focused<K>(tops: &HashMap<K, Top>) -> Option<(String, String)> {
    tops.values().find(|t| t.active).map(|t| (t.app.clone(), t.title.clone()))
}

#[derive(Default)]
struct Tops(HashMap<ObjectId, Top>);

/// The focused window, or None when there's no Wayland session or the compositor lacks the protocol.
pub fn active() -> Option<(String, String)> {
    let conn = Connection::connect_to_env().ok()?;
    let (globals, mut queue) = registry_queue_init::<Tops>(&conn).ok()?;
    let _m: ZwlrForeignToplevelManagerV1 = globals.bind(&queue.handle(), 1..=3, ()).ok()?;
    let mut tops = Tops::default();
    // The manager announces every toplevel on bind, each followed by its title, app id and state.
    queue.roundtrip(&mut tops).ok()?;
    focused(&tops.0)
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for Tops {
    fn event(s: &mut Self, h: &ZwlrForeignToplevelHandleV1, ev: handle::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        let ev = match ev {
            handle::Event::Title { title } => Ev::Title(title),
            handle::Event::AppId { app_id } => Ev::App(app_id),
            handle::Event::State { state } => Ev::State(state),
            handle::Event::Closed => Ev::Closed,
            _ => return,
        };
        apply(&mut s.0, h.id(), ev);
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for Tops {
    fn event(_: &mut Self, _: &ZwlrForeignToplevelManagerV1, _: manager::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
    event_created_child!(Tops, ZwlrForeignToplevelManagerV1, [manager::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ())]);
}

impl Dispatch<WlRegistry, GlobalListContents> for Tops {
    fn event(_: &mut Self, _: &WlRegistry, _: <WlRegistry as Proxy>::Event, _: &GlobalListContents, _: &Connection, _: &QueueHandle<Self>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(s: &[u32]) -> Ev {
        Ev::State(s.iter().flat_map(|v| v.to_ne_bytes()).collect())
    }

    #[test]
    fn tracks_the_activated_toplevel() {
        let mut tops = HashMap::new();
        apply(&mut tops, 1, Ev::App("foot".into()));
        apply(&mut tops, 1, Ev::Title("~".into()));
        apply(&mut tops, 1, state(&[0, 2])); // maximized + activated
        apply(&mut tops, 2, Ev::App("firefox".into()));
        apply(&mut tops, 2, state(&[]));
        assert_eq!(focused(&tops), Some(("foot".into(), "~".into())));
        apply(&mut tops, 1, Ev::Title("vim notes.org".into()));
        assert_eq!(focused(&tops), Some(("foot".into(), "vim notes.org".into())));
        // Focus moves: the old one's state drops activated.
        apply(&mut tops, 1, state(&[0]));
        apply(&mut tops, 2, state(&[2]));
        assert_eq!(focused(&tops), Some(("firefox".into(), String::new())));
        apply(&mut tops, 2, Ev::Closed);
        assert_eq!(focused(&tops), None);
        assert_eq!(tops.len(), 1);
    }

    /// Needs a wlroots session: `cargo test -- --ignored wlr_active --nocapture`.
    #[test]
    #[ignore]
    fn wlr_active() {
        println!("{:?}", active().expect("no focused toplevel"));
    }
}
