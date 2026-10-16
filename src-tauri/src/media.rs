//! Whether a microphone or camera is in use right now (opt-in: `activity_media_signal`), the
//! surest sign of a call. Only that yes/no is ever read: no audio or video is opened, and no app
//! is named. macOS only so far; elsewhere it's unknown (None).

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
    #[cfg(not(target_os = "macos"))]
    {
        None
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

    /// Needs a Mac: `cargo test -- --ignored media_probe --nocapture` (start a call to see true).
    #[test]
    #[ignore]
    fn media_probe() {
        println!("{:?}", in_use());
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
