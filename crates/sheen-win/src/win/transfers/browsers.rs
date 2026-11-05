//! Browser downloads. Browsers write to a temporary `*.crdownload` (Chromium
//! family) or `*.part` (Firefox) file next to the final one and rename it when
//! done, so the file's size is the bytes received. The expected total comes
//! from the browser's own `History` database (Chromium); Firefox does not
//! record it until the download finishes, so its downloads show speed and
//! size without an ETA.

use super::sqlite::{self, Value};
use super::transfer_id;
use super::windows_list::Snapshot;
use sheen_core::{TransferKind, TransferSample, TransferUnit};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};
use windows::core::PCWSTR;
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadDirectoryChangesW, FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY,
    FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_INFORMATION, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_Downloads, SHGetKnownFolderPath, KF_FLAG_DEFAULT};

const PARTIAL: &[&str] = &["crdownload", "part", "partial", "download"];
const BROWSER_EXES: &[&str] = &[
    "chrome.exe",
    "msedge.exe",
    "brave.exe",
    "firefox.exe",
    "opera.exe",
    "vivaldi.exe",
    "browser.exe",
    "chromium.exe",
    "librewolf.exe",
    "waterfox.exe",
];
/// Chromium-family profile roots, relative to %LOCALAPPDATA% / %APPDATA%.
const CHROMIUM_ROOTS: &[(&str, bool, &str)] = &[
    ("Google\\Chrome\\User Data", true, "Chrome"),
    ("Google\\Chrome Beta\\User Data", true, "Chrome"),
    ("Microsoft\\Edge\\User Data", true, "Edge"),
    ("BraveSoftware\\Brave-Browser\\User Data", true, "Brave"),
    ("Vivaldi\\User Data", true, "Vivaldi"),
    ("Chromium\\User Data", true, "Chromium"),
    ("Yandex\\YandexBrowser\\User Data", true, "Yandex"),
    ("Opera Software\\Opera Stable", false, "Opera"),
    ("Opera Software\\Opera GX Stable", false, "Opera GX"),
];

pub fn downloads_dir() -> Option<PathBuf> {
    unsafe {
        let p = SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, None).ok()?;
        let path = p.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(p.0 as *const _));
        path
    }
}

fn is_partial(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| PARTIAL.iter().any(|p| p.eq_ignore_ascii_case(ext)))
}

/// Wake the poller whenever a file is created or renamed in Downloads. Size
/// changes are deliberately not watched: they fire on every write, and the
/// poller reads sizes on its own schedule.
pub fn spawn_downloads_watcher(wake: Sender<()>) {
    let Some(dir) = downloads_dir() else { return };
    std::thread::Builder::new()
        .name("sheen-downloads-watch".into())
        .spawn(move || unsafe {
            let wide: Vec<u16> = dir.as_os_str().encode_wide_nul();
            let Ok(handle) = CreateFileW(
                PCWSTR(wide.as_ptr()),
                FILE_LIST_DIRECTORY.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                None,
            ) else {
                return;
            };
            let mut buf = vec![0u32; 16 * 1024];
            loop {
                let mut bytes = 0u32;
                if ReadDirectoryChangesW(
                    handle,
                    buf.as_mut_ptr().cast(),
                    (buf.len() * 4) as u32,
                    false,
                    FILE_NOTIFY_CHANGE_FILE_NAME,
                    Some(&mut bytes),
                    None,
                    None,
                )
                .is_err()
                {
                    break;
                }
                if (bytes == 0 || any_partial(&buf, bytes as usize)) && wake.send(()).is_err() {
                    break;
                }
            }
            let _ = CloseHandle(handle);
        })
        .expect("spawn downloads watcher");
}

/// Walk a `FILE_NOTIFY_INFORMATION` chain looking for partial-download names.
unsafe fn any_partial(buf: &[u32], len: usize) -> bool {
    let base = buf.as_ptr() as *const u8;
    let mut offset = 0usize;
    while offset < len {
        let info = &*(base.add(offset) as *const FILE_NOTIFY_INFORMATION);
        let name =
            std::slice::from_raw_parts(info.FileName.as_ptr(), info.FileNameLength as usize / 2);
        if is_partial(&String::from_utf16_lossy(name)) {
            return true;
        }
        if info.NextEntryOffset == 0 {
            break;
        }
        offset += info.NextEntryOffset as usize;
    }
    false
}

trait EncodeWide {
    fn encode_wide_nul(&self) -> Vec<u16>;
}

impl EncodeWide for std::ffi::OsStr {
    fn encode_wide_nul(&self) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        self.encode_wide().chain(Some(0)).collect()
    }
}

/// What the browser told us about a download.
#[derive(Clone)]
struct Known {
    app: String,
    name: String,
    total: Option<u64>,
}

#[derive(Default)]
pub struct BrowserProvider {
    downloads: Option<PathBuf>,
    known: HashMap<PathBuf, Known>,
    histories: Vec<(PathBuf, String)>,
    histories_at: Option<Instant>,
    last_lookup: Option<Instant>,
    active: bool,
}

impl BrowserProvider {
    pub fn active(&self) -> bool {
        self.active
    }

    pub fn poll(&mut self, snap: &Snapshot) -> Vec<TransferSample> {
        if self.downloads.is_none() {
            self.downloads = downloads_dir();
        }
        let Some(dir) = self.downloads.clone() else { return Vec::new() };
        let partials: Vec<(PathBuf, u64)> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| is_partial(&e.file_name().to_string_lossy()))
            .filter_map(|e| Some((e.path(), e.metadata().ok()?.len())))
            .collect();
        self.active = !partials.is_empty();
        self.known.retain(|p, _| partials.iter().any(|(q, _)| q == p));
        if partials.is_empty() {
            return Vec::new();
        }

        // Look up totals for downloads we have not identified yet (at most
        // every 2 s, since it touches the browsers' databases).
        let unknown =
            partials.iter().any(|(p, _)| !self.known.get(p).is_some_and(|k| k.total.is_some()));
        if unknown && self.last_lookup.is_none_or(|t| t.elapsed() > Duration::from_secs(2)) {
            self.last_lookup = Some(Instant::now());
            self.lookup_chromium();
        }

        let in_view = BROWSER_EXES.contains(&snap.foreground_exe.as_str());
        partials
            .into_iter()
            .map(|(path, size)| {
                let known = self.known.get(&path).cloned().unwrap_or_else(|| guess(&path));
                TransferSample {
                    id: transfer_id(("download", &path)),
                    kind: TransferKind::Download,
                    app: known.app,
                    name: known.name,
                    done: size,
                    total: known.total.filter(|t| *t >= size),
                    unit: TransferUnit::Bytes,
                    source_in_view: in_view,
                }
            })
            .collect()
    }

    fn lookup_chromium(&mut self) {
        if self.histories_at.is_none_or(|t| t.elapsed() > Duration::from_secs(60)) {
            self.histories = find_histories();
            self.histories_at = Some(Instant::now());
        }
        for (db, app) in &self.histories {
            let Some(rows) = sqlite::query(
                db,
                "SELECT current_path, target_path, total_bytes FROM downloads WHERE state = 0",
                &[true, true, false],
            ) else {
                continue;
            };
            for row in rows {
                let [Value::Text(current), Value::Text(target), Value::Int(total)] = &row[..]
                else {
                    continue;
                };
                let name = Path::new(target)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.known.insert(
                    PathBuf::from(current),
                    Known { app: app.clone(), name, total: (*total > 0).then_some(*total as u64) },
                );
            }
        }
    }
}

/// Best effort before (or without) the browser's own record.
fn guess(path: &Path) -> Known {
    let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let (stem, ext) = file.rsplit_once('.').unwrap_or((&file, ""));
    let app = if ext.eq_ignore_ascii_case("part") { "Firefox" } else { "Browser" };
    Known { app: app.into(), name: stem.to_string(), total: None }
}

fn find_histories() -> Vec<(PathBuf, String)> {
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let roaming = std::env::var_os("APPDATA").map(PathBuf::from);
    let mut out = Vec::new();
    for (rel, is_local, app) in CHROMIUM_ROOTS {
        let Some(base) = (if *is_local { &local } else { &roaming }) else { continue };
        let root = base.join(rel);
        if root.join("History").is_file() {
            out.push((root.join("History"), app.to_string()));
        }
        for entry in std::fs::read_dir(&root).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "Default" || name.starts_with("Profile ") {
                let h = entry.path().join("History");
                if h.is_file() {
                    out.push((h, app.to_string()));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_partial_files() {
        assert!(is_partial("Unconfirmed 12345.crdownload"));
        assert!(is_partial("movie.mkv.part"));
        assert!(!is_partial("movie.mkv"));
        assert!(!is_partial("part"));
    }

    #[test]
    fn guesses_names_from_temp_files() {
        let k = guess(Path::new(r"C:\Downloads\movie.mkv.part"));
        assert_eq!((k.app.as_str(), k.name.as_str()), ("Firefox", "movie.mkv"));
        let k = guess(Path::new(r"C:\Downloads\setup.exe.crdownload"));
        assert_eq!((k.app.as_str(), k.name.as_str()), ("Browser", "setup.exe"));
    }
}
