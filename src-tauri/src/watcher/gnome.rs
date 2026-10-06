//! The focused window on GNOME Wayland, which has no API for it: a small Shell extension
//! (`packaging/gnome-extension/`) answers `org.margin.ActiveWindow.Get` on the session bus.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zbus::blocking::Connection;
use zbus::Message;

pub const UUID: &str = "active-window@margin.albin.dev";
const FILES: [(&str, &str); 2] = [
    ("metadata.json", include_str!("../../../packaging/gnome-extension/metadata.json")),
    ("extension.js", include_str!("../../../packaging/gnome-extension/extension.js")),
];

/// Kept between polls: a new bus connection every 5 s would mean a new zbus thread each time.
static BUS: Mutex<Option<Connection>> = Mutex::new(None);

/// A GNOME session (`XDG_CURRENT_DESKTOP` lists GNOME, as with "ubuntu:GNOME").
pub fn session() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|d| d.split(':').any(|s| s.eq_ignore_ascii_case("gnome")))
}

/// The extension's reply, None when it isn't running.
fn get() -> Option<Message> {
    let mut bus = BUS.lock().unwrap();
    if bus.is_none() {
        *bus = Connection::session().ok();
    }
    let n = "org.margin.ActiveWindow";
    bus.as_ref()?.call_method(Some(n), "/org/margin/ActiveWindow", Some(n), "Get", &()).ok()
}

/// The focused window's class and title from a `Get` reply; None when nothing is focused.
fn parse(m: &Message) -> Option<(String, String)> {
    let (app, title): (String, String) = m.body().deserialize().ok()?;
    (!app.is_empty() || !title.is_empty()).then_some((app, title))
}

pub fn active() -> Option<(String, String)> {
    parse(&get()?)
}

/// Where the extension goes under the XDG data dir DATA (`~/.local/share`).
fn dir(data: &Path) -> PathBuf {
    data.join("gnome-shell/extensions").join(UUID)
}

/// Copy the extension into DATA's extensions folder (GNOME picks it up at the next login).
pub fn install(data: &Path) -> std::io::Result<()> {
    let d = dir(data);
    std::fs::create_dir_all(&d)?;
    FILES.iter().try_for_each(|(name, text)| std::fs::write(d.join(name), text))
}

/// "running", "installed" (log in again and enable it) or "missing".
pub fn status(data: &Path) -> &'static str {
    if get().is_some() {
        "running"
    } else if dir(data).join("extension.js").exists() {
        "installed"
    } else {
        "missing"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reply_parsing() {
        let reply = |a: &str, t: &str| Message::method_call("/org/margin/ActiveWindow", "Get").unwrap().build(&(a, t)).unwrap();
        assert_eq!(parse(&reply("org.gnome.Nautilus", "Home")), Some(("org.gnome.Nautilus".into(), "Home".into())));
        assert_eq!(parse(&reply("", "")), None, "nothing focused");
        let wrong = Message::method_call("/", "Get").unwrap().build(&("only one",)).unwrap();
        assert_eq!(parse(&wrong), None);
    }

    #[test]
    fn install_copies_the_extension() {
        let data = std::env::temp_dir().join(format!("margin-gnome-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&data);
        install(&data).unwrap();
        let d = data.join("gnome-shell/extensions/active-window@margin.albin.dev");
        let meta = std::fs::read_to_string(d.join("metadata.json")).unwrap();
        assert!(meta.contains(&format!("\"uuid\": \"{UUID}\"")), "uuid matches the folder");
        assert!(std::fs::read_to_string(d.join("extension.js")).unwrap().contains("org.margin.ActiveWindow"));
        install(&data).unwrap(); // reinstalling overwrites
        std::fs::remove_dir_all(&data).unwrap();
    }
}
