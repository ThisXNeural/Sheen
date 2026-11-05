//! Background transfers for the island: browser downloads, File Explorer
//! copies/moves (including phone and USB transfers), and classic progress
//! dialogs (IDM, 7-Zip, WinRAR, installers).
//!
//! There is no Windows API to read another app's progress, so each source is
//! read the way it exposes itself:
//!
//! | Source | How |
//! |---|---|
//! | Chromium browsers | `.crdownload` file size, plus the expected size from the browser's `History` database, read in place with Windows' own `winsqlite3.dll` |
//! | Firefox | `.part` file size (total unknown, so speed only) |
//! | File Explorer | the copy dialog's progress bar, via UI Automation |
//! | Win32 dialogs | `msctls_progress32` controls, via `PBM_GETPOS` |
//!
//! Cost: one thread that wakes every 1.5 s (1 s while something is active)
//! to enumerate top-level windows, which takes well under a millisecond. The
//! Downloads folder is watched with `ReadDirectoryChangesW`, so it costs
//! nothing until a file appears.

mod browsers;
mod dialogs;
mod explorer;
mod sqlite;
mod windows_list;

use crate::Signal;
use sheen_core::{SheenEvent, TransferSample};
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;

const IDLE_POLL: Duration = Duration::from_millis(1500);
const ACTIVE_POLL: Duration = Duration::from_millis(1000);

pub fn spawn_transfer_watcher(out: Sender<Signal>) {
    let (wake_tx, wake_rx) = channel::<()>();
    browsers::spawn_downloads_watcher(wake_tx);

    std::thread::Builder::new()
        .name("sheen-transfers".into())
        .spawn(move || {
            super::init_com();
            let mut explorer = explorer::ExplorerProvider::new();
            let mut dialogs = dialogs::DialogProvider::default();
            let mut browsers = browsers::BrowserProvider::default();
            let mut last_empty = true;

            loop {
                let windows = windows_list::snapshot();
                let mut samples: Vec<TransferSample> = Vec::new();
                samples.extend(explorer.poll(&windows));
                samples.extend(dialogs.poll(&windows));
                samples.extend(browsers.poll(&windows));

                // Report while anything is active, plus once more when the
                // list empties so the island can flash "done".
                if !(samples.is_empty() && last_empty) {
                    last_empty = samples.is_empty();
                    if out.send(Signal::Event(SheenEvent::Transfers(samples))).is_err() {
                        return;
                    }
                }

                let wait = if last_empty && !browsers.active() { IDLE_POLL } else { ACTIVE_POLL };
                let _ = wake_rx.recv_timeout(wait);
                while wake_rx.try_recv().is_ok() {}
            }
        })
        .expect("spawn transfer watcher");
}

/// Stable id for a transfer from any hashable identity.
fn transfer_id(parts: impl std::hash::Hash) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

/// "45% file.zip" / "45% complete" → "file.zip" / "complete".
fn strip_percent(title: &str) -> String {
    let t = title.trim();
    let rest = t.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ',');
    let rest = rest.strip_prefix('%').map(str::trim_start).unwrap_or(t);
    rest.to_string()
}

#[cfg(test)]
mod tests {
    use super::strip_percent;

    #[test]
    fn strips_leading_percentages() {
        assert_eq!(strip_percent("45% movie.mkv"), "movie.mkv");
        assert_eq!(strip_percent("12.5% complete"), "complete");
        assert_eq!(strip_percent("Extracting"), "Extracting");
        assert_eq!(strip_percent("2024 report.pdf"), "2024 report.pdf");
    }
}
