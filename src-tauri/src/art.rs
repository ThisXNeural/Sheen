//! `sheen-art://` protocol: cover thumbnails for the library and images on
//! the shelf, served straight to `<img>` tags. Compared with base64 over IPC
//! this lets the webview load lazily, decode off the main thread, and cache.
//!
//! Paths: `thumb/<art key>`, `cover/<art key>`, `shelf/<item id>`.

use crate::music::{COVER_PX, THUMB_PX};
use crate::Shared;
use std::sync::Arc;
use tauri::http::{header, Response, StatusCode};
use tauri::{Manager, UriSchemeContext, UriSchemeResponder, Wry};

pub const SCHEME: &str = "sheen-art";
const SHELF_PX: u32 = 160;

pub fn handle(
    ctx: UriSchemeContext<'_, Wry>,
    request: tauri::http::Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    let path = percent_decode(request.uri().path().trim_start_matches('/'));
    // Decoding and resizing must not block the webview's thread.
    std::thread::spawn(move || {
        let state = app.state::<Arc<Shared>>();
        let (kind, key) = path.split_once('/').unwrap_or((&path, ""));
        let image = match kind {
            "thumb" => state.music.cover(key, THUMB_PX),
            "cover" => state.music.cover(key, COVER_PX),
            "shelf" => key.parse().ok().and_then(|id| state.shelf_thumbnail(id, SHELF_PX)),
            _ => None,
        };
        let response = match image {
            Some(bytes) => Response::builder()
                .header(header::CONTENT_TYPE, "image/jpeg")
                .header(header::CACHE_CONTROL, "max-age=604800")
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(bytes),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                .body(Vec::new()),
        };
        responder.respond(response.expect("valid response"));
    });
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(b) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    #[test]
    fn decodes_percent_escapes() {
        assert_eq!(super::percent_decode("thumb%2Fabc"), "thumb/abc");
        assert_eq!(super::percent_decode("a%20b%"), "a b%");
        assert_eq!(super::percent_decode("plain"), "plain");
    }
}
