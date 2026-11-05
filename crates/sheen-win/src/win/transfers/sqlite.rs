//! Minimal read-only access to SQLite through `winsqlite3.dll`, which ships
//! with Windows 10 and 11. This avoids bundling a SQLite build for the one
//! query Sheen needs.

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::Path;

#[link(name = "winsqlite3", kind = "raw-dylib")]
extern "system" {
    fn sqlite3_open_v2(
        filename: *const c_char,
        db: *mut *mut c_void,
        flags: c_int,
        vfs: *const c_char,
    ) -> c_int;
    fn sqlite3_prepare_v2(
        db: *mut c_void,
        sql: *const c_char,
        n: c_int,
        stmt: *mut *mut c_void,
        tail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_step(stmt: *mut c_void) -> c_int;
    fn sqlite3_column_text(stmt: *mut c_void, col: c_int) -> *const c_char;
    fn sqlite3_column_int64(stmt: *mut c_void, col: c_int) -> i64;
    fn sqlite3_finalize(stmt: *mut c_void) -> c_int;
    fn sqlite3_close(db: *mut c_void) -> c_int;
    fn sqlite3_busy_timeout(db: *mut c_void, ms: c_int) -> c_int;
}

const SQLITE_OK: c_int = 0;
const SQLITE_ROW: c_int = 100;
const SQLITE_OPEN_READONLY: c_int = 0x1;
const SQLITE_OPEN_URI: c_int = 0x40;

pub enum Value {
    Text(String),
    Int(i64),
}

/// Run `sql` against the database at `path` without taking any lock
/// (`immutable=1`). This is safe for a database another process is writing:
/// at worst a query sees a torn page and fails, and the caller retries later.
pub fn query(path: &Path, sql: &str, columns: &[bool]) -> Option<Vec<Vec<Value>>> {
    let uri = CString::new(file_uri(path)).ok()?;
    let sql = CString::new(sql).ok()?;
    unsafe {
        let mut db = std::ptr::null_mut();
        if sqlite3_open_v2(
            uri.as_ptr(),
            &mut db,
            SQLITE_OPEN_READONLY | SQLITE_OPEN_URI,
            std::ptr::null(),
        ) != SQLITE_OK
        {
            sqlite3_close(db);
            return None;
        }
        sqlite3_busy_timeout(db, 50);
        let mut stmt = std::ptr::null_mut();
        if sqlite3_prepare_v2(db, sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) != SQLITE_OK {
            sqlite3_close(db);
            return None;
        }
        let mut rows = Vec::new();
        while sqlite3_step(stmt) == SQLITE_ROW {
            let row = columns
                .iter()
                .enumerate()
                .map(|(i, is_text)| {
                    if *is_text {
                        let p = sqlite3_column_text(stmt, i as c_int);
                        Value::Text(if p.is_null() {
                            String::new()
                        } else {
                            CStr::from_ptr(p).to_string_lossy().into_owned()
                        })
                    } else {
                        Value::Int(sqlite3_column_int64(stmt, i as c_int))
                    }
                })
                .collect();
            rows.push(row);
        }
        sqlite3_finalize(stmt);
        sqlite3_close(db);
        Some(rows)
    }
}

/// `C:\Users\A B\History` → `file:///C:/Users/A%20B/History?immutable=1`.
fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file:///");
    for b in path.to_string_lossy().replace('\\', "/").bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                uri.push(b as char)
            }
            _ => uri.push_str(&format!("%{b:02X}")),
        }
    }
    uri.push_str("?immutable=1");
    uri
}

#[cfg(test)]
mod tests {
    use super::file_uri;
    use std::path::Path;

    #[test]
    fn encodes_paths_as_uris() {
        assert_eq!(
            file_uri(Path::new(r"C:\Users\A B\AppData\Local\Google\Chrome\User Data\Default\History")),
            "file:///C:/Users/A%20B/AppData/Local/Google/Chrome/User%20Data/Default/History?immutable=1"
        );
    }
}
