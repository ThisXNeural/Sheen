use std::os::windows::process::CommandExt;
use std::path::Path;

/// Open an Explorer window with `path` selected.
pub fn reveal(path: &Path) {
    let _ = std::process::Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .spawn();
}

/// Open a file with its default app.
pub fn open(path: &Path) {
    let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
}
