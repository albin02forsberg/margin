//! Whether a microphone or camera is in use right now (opt-in: `activity_media_signal`), the
//! surest sign of a call. Only that yes/no is ever read: no audio or video is opened, and no app
//! is named. macOS and Linux; elsewhere it's unknown (None).

use chrono::{Duration, NaiveDate, NaiveDateTime};
use std::path::Path;

/// A run of use this long is a virtual camera or a stuck device, not a call.
const MAX_HOURS: i64 = 8;

/// The media log's intervals on DAY in DIR (`<data>/activity/media`), joined across the
/// watcher's 5-min record splits; runs of 8 h or more are dropped.
pub fn intervals(dir: &Path, day: NaiveDate) -> Vec<(NaiveDateTime, NaiveDateTime)> {
    merged(crate::watcher::spans(dir, day).into_iter().map(|s| (s.start, s.end)).collect())
}

/// SPANS sorted and joined where under 15 s (3 polls) apart, minus runs of 8 h or more.
fn merged(mut spans: Vec<(NaiveDateTime, NaiveDateTime)>) -> Vec<(NaiveDateTime, NaiveDateTime)> {
    spans.sort();
    let mut out: Vec<(NaiveDateTime, NaiveDateTime)> = vec![];
    for (s, e) in spans {
        match out.last_mut() {
            Some(l) if s - l.1 <= Duration::seconds(15) => l.1 = l.1.max(e),
            _ => out.push((s, e)),
        }
    }
    out.retain(|(s, e)| *e - *s < Duration::hours(MAX_HOURS));
    out
}

/// Some(true) when any app is recording from a microphone or using a camera; None when this OS can't tell.
pub fn in_use() -> Option<bool> {
    #[cfg(target_os = "macos")]
    {
        mac::in_use()
    }
    #[cfg(target_os = "linux")]
    {
        Some(linux::alsa_capture(Path::new("/proc/asound")) || linux::camera(Path::new("/proc"), Path::new("/dev")) || linux::pactl())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

#[cfg(any(target_os = "linux", test))]
mod linux {
    use std::path::Path;

    /// Whether any process under PROC (`/proc`) has a `/dev/video*` open. Other users' fds are
    /// unreadable and skipped, which is fine: a call app runs as the user. Skips the walk when
    /// DEV (`/dev`) has no `video*` node (`v4l-subdev*` doesn't count); an unreadable DEV still walks.
    pub fn camera(proc: &Path, dev: &Path) -> bool {
        let none = match std::fs::read_dir(dev) {
            Ok(d) => !d.flatten().any(|n| n.file_name().to_string_lossy().starts_with("video")),
            Err(e) => e.kind() == std::io::ErrorKind::NotFound,
        };
        if none {
            return false;
        }
        let Ok(pids) = std::fs::read_dir(proc) else { return false };
        pids.flatten().filter(|p| p.file_name().to_string_lossy().bytes().all(|b| b.is_ascii_digit())).any(|p| {
            std::fs::read_dir(p.path().join("fd")).into_iter().flatten().flatten().any(|fd| std::fs::read_link(fd.path()).is_ok_and(|l| l.to_string_lossy().starts_with("/dev/video")))
        })
    }

    /// Whether an ALSA capture substream under ASOUND (`/proc/asound`) is running:
    /// `card*/pcm*c/sub*/status` starts with `state: RUNNING`. PipeWire/PulseAudio suspend an
    /// unused source after a few seconds, so this is false between calls.
    pub fn alsa_capture(asound: &Path) -> bool {
        let dirs = |d: &Path, pre: &str, suf: &str| -> Vec<std::path::PathBuf> {
            std::fs::read_dir(d).into_iter().flatten().flatten().filter(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                n.starts_with(pre) && n.ends_with(suf)
            }).map(|e| e.path()).collect()
        };
        dirs(asound, "card", "").iter().flat_map(|c| dirs(c, "pcm", "c")).flat_map(|p| dirs(&p, "sub", "")).any(|s| std::fs::read_to_string(s.join("status")).is_ok_and(|t| t.starts_with("state: RUNNING")))
    }

    /// Whether `pactl list source-outputs short` lists a recording stream (PulseAudio or
    /// PipeWire; catches Bluetooth mics, which aren't ALSA). Missing pactl or a sound server
    /// that doesn't answer within 2 s counts as no.
    #[cfg(target_os = "linux")]
    pub fn pactl() -> bool {
        use std::process::{Command, Stdio};
        let mut c = Command::new("pactl");
        c.args(["list", "source-outputs", "short"]).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
        for (k, v) in crate::backup::git_env_changes() {
            match v {
                Some(v) => c.env(k, v),
                None => c.env_remove(k),
            };
        }
        let Ok(mut child) = c.spawn() else { return false };
        for _ in 0..20 {
            if let Ok(Some(_)) = child.try_wait() {
                let mut out = String::new();
                let _ = std::io::Read::read_to_string(&mut child.stdout.take().unwrap(), &mut out);
                return recording(&out);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let _ = child.kill();
        let _ = child.wait();
        false
    }

    /// pactl's short listing OUT has a stream line.
    pub fn recording(out: &str) -> bool {
        out.lines().any(|l| !l.trim().is_empty())
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::ffi::c_void;

    /// A property address: selector, scope, element (same layout as the C struct).
    type Addr = [u32; 3];
    const fn cc(s: &[u8; 4]) -> u32 {
        u32::from_be_bytes(*s)
    }
    const SYSTEM: u32 = 1; // kAudioObjectSystemObject, kCMIOObjectSystemObject
    const GLOBAL: u32 = cc(b"glob");

    #[link(name = "CoreAudio", kind = "framework")]
    extern "C" {
        fn AudioObjectGetPropertyDataSize(id: u32, a: *const Addr, qs: u32, q: *const c_void, size: *mut u32) -> i32;
        fn AudioObjectGetPropertyData(id: u32, a: *const Addr, qs: u32, q: *const c_void, size: *mut u32, data: *mut c_void) -> i32;
    }
    #[link(name = "CoreMediaIO", kind = "framework")]
    extern "C" {
        fn CMIOObjectGetPropertyDataSize(id: u32, a: *const Addr, qs: u32, q: *const c_void, size: *mut u32) -> i32;
        fn CMIOObjectGetPropertyData(id: u32, a: *const Addr, qs: u32, q: *const c_void, size: u32, used: *mut u32, data: *mut c_void) -> i32;
    }

    /// Whether any object in the system object's id list LIST (processes, devices) has the u32
    /// property ON set; None when the list can't be read. CMIO picks CoreMediaIO over CoreAudio.
    fn any(list: u32, on: u32, cmio: bool) -> Option<bool> {
        let (la, oa): (Addr, Addr) = ([list, GLOBAL, 0], [on, GLOBAL, 0]);
        let mut size = 0u32;
        // SAFETY: plain C calls with valid pointers; buffers are sized from the size query.
        unsafe {
            let st = if cmio { CMIOObjectGetPropertyDataSize(SYSTEM, &la, 0, std::ptr::null(), &mut size) } else { AudioObjectGetPropertyDataSize(SYSTEM, &la, 0, std::ptr::null(), &mut size) };
            if st != 0 {
                return None;
            }
            let mut ids = vec![0u32; size as usize / 4];
            let mut used = size;
            let st = if cmio { CMIOObjectGetPropertyData(SYSTEM, &la, 0, std::ptr::null(), size, &mut used, ids.as_mut_ptr().cast()) } else { AudioObjectGetPropertyData(SYSTEM, &la, 0, std::ptr::null(), &mut used, ids.as_mut_ptr().cast()) };
            if st != 0 {
                return None;
            }
            ids.truncate(used as usize / 4);
            Some(ids.into_iter().any(|id| {
                let (mut v, mut n) = (0u32, 4u32);
                let st = if cmio { CMIOObjectGetPropertyData(id, &oa, 0, std::ptr::null(), 4, &mut n, (&mut v as *mut u32).cast()) } else { AudioObjectGetPropertyData(id, &oa, 0, std::ptr::null(), &mut n, (&mut v as *mut u32).cast()) };
                st == 0 && v != 0
            }))
        }
    }

    /// Mic: any process recording (kAudioProcessPropertyIsRunningInput, macOS 14+), not the
    /// default input device running, which a headset playing music also is. Camera: any video
    /// device running somewhere (CoreMediaIO). Neither needs a permission.
    pub fn in_use() -> Option<bool> {
        let mic = any(cc(b"prs#"), cc(b"piri"), false);
        let cam = any(cc(b"dev#"), cc(b"gone"), true);
        match (mic, cam) {
            (None, None) => None,
            (m, c) => Some(m == Some(true) || c == Some(true)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(&format!("2026-10-01 {s}"), "%Y-%m-%d %H:%M:%S").unwrap()
    }

    /// Needs a Mac or Linux: `cargo test -- --ignored media_probe --nocapture` (start a call to see true).
    #[test]
    #[ignore]
    fn media_probe() {
        println!("{:?}", in_use());
    }

    #[cfg(unix)]
    #[test]
    fn linux_probe() {
        let root = std::env::temp_dir().join(format!("margin-media-{}", crate::notes::gen_id()));
        let w = |p: &str, t: &str| {
            let p = root.join(p);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, t).unwrap();
        };
        let ln = |to: &str, p: &str| {
            std::fs::create_dir_all(root.join(p).parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(to, root.join(p)).unwrap();
        };
        let (proc, asound) = (root.join("proc"), root.join("asound"));
        ln("/dev/null", "proc/12/fd/0");
        ln("socket:[123]", "proc/12/fd/3");
        ln("/dev/video0", "proc/self/fd/9"); // not a pid
        w("proc/cpuinfo", "");
        w("asound/card0/pcm0p/sub0/status", "state: RUNNING\n"); // playback
        w("asound/card0/pcm0c/sub0/status", "closed\n");
        w("asound/card0/pcm0c/info", "");
        let dev = root.join("dev");
        w("dev/v4l-subdev0", "");
        assert!(!linux::camera(&proc, &dev) && !linux::alsa_capture(&asound));
        ln("/dev/video2", "proc/40/fd/7");
        assert!(!linux::camera(&proc, &dev), "no video node: skip the walk");
        w("dev/video2", "");
        w("asound/card1/pcm0c/sub1/status", "state: RUNNING\nowner_pid   : 40\n");
        assert!(linux::camera(&proc, &dev) && linux::alsa_capture(&asound));
        assert!(!linux::camera(&root.join("nope"), &dev) && !linux::alsa_capture(&root.join("nope")));
        assert!(linux::recording("42\t57\t89\tPipeWire\tfloat32le 1ch 48000Hz\n") && !linux::recording("") && !linux::recording("\n"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn media_intervals() {
        // Split every 5 min by the watcher, out of order; a later separate use; a camera on all day.
        let got = merged(vec![(t("09:05:00"), t("09:10:00")), (t("09:00:00"), t("09:05:00")), (t("09:10:05"), t("09:12:00")), (t("10:00:00"), t("10:30:00"))]);
        assert_eq!(got, [(t("09:00:00"), t("09:12:00")), (t("10:00:00"), t("10:30:00"))]);
        let day: Vec<_> = (0..100).map(|i| (t("08:00:00") + Duration::minutes(5 * i), t("08:05:00") + Duration::minutes(5 * i))).collect();
        assert!(merged(day).is_empty(), "8 h+ of use is a virtual camera");
    }
}
