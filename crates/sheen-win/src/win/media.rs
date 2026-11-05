//! Media capsule backend: Windows System Media Transport Controls (SMTC).
//!
//! SMTC is the session API behind the stock Windows media flyout, so any app
//! that integrates with it (Spotify, browsers, Telegram, Tidal, ...) works
//! without a dedicated player.
//!
//! WinRT event handlers fire on arbitrary thread-pool threads; they only post a
//! message to this service's own thread, which re-reads state and owns all
//! session objects. That keeps every SMTC call on one thread and makes
//! re-subscribing on session changes straightforward.

use super::init_com;
use crate::Signal;
use base64::Engine;
use sheen_core::{MediaCommand, MediaInfo, SheenEvent};
use std::sync::mpsc::{channel, Receiver, Sender};
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::{Buffer, DataReader, InputStreamOptions};

/// 100 ns ticks between 1601-01-01 (FILETIME epoch) and 1970-01-01.
const FILETIME_UNIX_OFFSET: i64 = 116_444_736_000_000_000;
/// Album art larger than this is skipped rather than shipped over IPC.
const MAX_ARTWORK_BYTES: u64 = 2 * 1024 * 1024;

enum Msg {
    /// The current session changed (or the session list did).
    Rebind,
    /// Title/artist/artwork changed.
    Properties,
    /// Play state or timeline changed.
    Playback,
    Command(MediaCommand),
}

pub struct MediaHandle(Sender<Msg>);

impl MediaHandle {
    pub fn spawn(tx: Sender<Signal>) -> Self {
        let (msg_tx, msg_rx) = channel();
        let self_tx = msg_tx.clone();
        std::thread::Builder::new()
            .name("sheen-media".into())
            .spawn(move || {
                init_com();
                if let Err(e) = run(tx, self_tx, msg_rx) {
                    log::error!("media service stopped: {e:?}");
                }
            })
            .expect("spawn media service");
        Self(msg_tx)
    }

    pub fn send(&self, command: MediaCommand) {
        let _ = self.0.send(Msg::Command(command));
    }
}

/// Event registrations on the current session, removed on rebind.
struct Bound {
    session: Session,
    tokens: [i64; 3],
}

impl Drop for Bound {
    fn drop(&mut self) {
        let [props, playback, timeline] = self.tokens;
        let _ = self.session.RemoveMediaPropertiesChanged(props);
        let _ = self.session.RemovePlaybackInfoChanged(playback);
        let _ = self.session.RemoveTimelinePropertiesChanged(timeline);
    }
}

fn run(out: Sender<Signal>, me: Sender<Msg>, inbox: Receiver<Msg>) -> windows::core::Result<()> {
    let manager = Manager::RequestAsync()?.join()?;

    let tx = me.clone();
    manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
        let _ = tx.send(Msg::Rebind);
        Ok(())
    }))?;
    let tx = me.clone();
    manager.SessionsChanged(&TypedEventHandler::new(move |_, _| {
        let _ = tx.send(Msg::Rebind);
        Ok(())
    }))?;

    let mut bound: Option<Bound> = None;
    let mut media: Option<MediaInfo> = None;
    let mut artwork_hash: Option<u64> = None;
    me.send(Msg::Rebind).ok();

    while let Ok(msg) = inbox.recv() {
        let session = match msg {
            Msg::Rebind => {
                bound = manager.GetCurrentSession().ok().and_then(|s| bind(s, &me).ok());
                bound.as_ref().map(|b| b.session.clone())
            }
            _ => bound.as_ref().map(|b| b.session.clone()),
        };

        let Some(session) = session else {
            if media.take().is_some() {
                artwork_hash = None;
                send(&out, Signal::Artwork(None));
                send(&out, Signal::Event(SheenEvent::Media(None)));
            }
            continue;
        };

        if let Msg::Command(command) = msg {
            if let Err(e) = execute(&session, command) {
                log::warn!("media command {command:?} failed: {e}");
            }
            continue;
        }

        let fetch_artwork = matches!(msg, Msg::Rebind | Msg::Properties);
        match read_info(&session) {
            Ok(info) => {
                if fetch_artwork {
                    let art = read_artwork(&session).unwrap_or(None);
                    let hash = art.as_deref().map(hash_str);
                    if hash != artwork_hash {
                        artwork_hash = hash;
                        send(&out, Signal::Artwork(art));
                    }
                }
                if media.as_ref() != Some(&info) {
                    media = Some(info.clone());
                    send(&out, Signal::Event(SheenEvent::Media(Some(info))));
                }
            }
            Err(e) => log::debug!("could not read media session: {e}"),
        }
    }
    Ok(())
}

fn bind(session: Session, me: &Sender<Msg>) -> windows::core::Result<Bound> {
    let tx = me.clone();
    let props = session.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
        let _ = tx.send(Msg::Properties);
        Ok(())
    }))?;
    let tx = me.clone();
    let playback = session.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
        let _ = tx.send(Msg::Playback);
        Ok(())
    }))?;
    let tx = me.clone();
    let timeline = session.TimelinePropertiesChanged(&TypedEventHandler::new(move |_, _| {
        let _ = tx.send(Msg::Playback);
        Ok(())
    }))?;
    Ok(Bound { session, tokens: [props, playback, timeline] })
}

fn read_info(session: &Session) -> windows::core::Result<MediaInfo> {
    let props = session.TryGetMediaPropertiesAsync()?.join()?;
    let playing = session.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing;
    let timeline = session.GetTimelineProperties()?;

    let ticks_to_ms = |t: i64| (t.max(0) / 10_000) as u64;
    let duration = timeline.EndTime()?.Duration - timeline.StartTime()?.Duration;
    let (position_ms, duration_ms) = if duration > 0 {
        (Some(ticks_to_ms(timeline.Position()?.Duration)), Some(ticks_to_ms(duration)))
    } else {
        (None, None)
    };
    let sampled_at_ms =
        ticks_to_ms(timeline.LastUpdatedTime()?.UniversalTime - FILETIME_UNIX_OFFSET);

    Ok(MediaInfo {
        source_app: session.SourceAppUserModelId()?.to_string(),
        title: props.Title()?.to_string(),
        artist: props.Artist()?.to_string(),
        album: props.AlbumTitle()?.to_string(),
        playing,
        position_ms,
        duration_ms,
        sampled_at_ms,
    })
}

fn read_artwork(session: &Session) -> windows::core::Result<Option<String>> {
    let props = session.TryGetMediaPropertiesAsync()?.join()?;
    let Ok(reference) = props.Thumbnail() else {
        return Ok(None);
    };
    let stream = reference.OpenReadAsync()?.join()?;
    let size = stream.Size()?;
    if size > MAX_ARTWORK_BYTES {
        return Ok(None);
    }
    // Streams from other apps may deliver the image in several pieces, and
    // some report no size at all: keep reading until the end.
    let mut bytes: Vec<u8> = Vec::with_capacity(size as usize);
    loop {
        let want =
            if size > 0 { (size - bytes.len() as u64).min(256 * 1024) as u32 } else { 64 * 1024 };
        if want == 0 || bytes.len() as u64 >= MAX_ARTWORK_BYTES {
            break;
        }
        let buffer = Buffer::Create(want)?;
        let filled = stream.ReadAsync(&buffer, want, InputStreamOptions::ReadAhead)?.join()?;
        let n = filled.Length()? as usize;
        if n == 0 {
            break;
        }
        let mut chunk = vec![0u8; n];
        DataReader::FromBuffer(&filled)?.ReadBytes(&mut chunk)?;
        bytes.extend_from_slice(&chunk);
    }
    let Some(mime) = image_mime(&bytes) else {
        return Ok(None);
    };
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(Some(format!("data:{mime};base64,{b64}")))
}

/// Identify the image by its first bytes (apps often leave the content type
/// empty or wrong). Anything else is not usable as artwork.
fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
        [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("image/webp"),
        [b'B', b'M', ..] => Some("image/bmp"),
        _ => None,
    }
}

fn execute(session: &Session, command: MediaCommand) -> windows::core::Result<()> {
    let accepted = match command {
        MediaCommand::TogglePlayPause => session.TryTogglePlayPauseAsync()?.join()?,
        MediaCommand::Next => session.TrySkipNextAsync()?.join()?,
        MediaCommand::Previous => session.TrySkipPreviousAsync()?.join()?,
        MediaCommand::Seek { position_ms } => {
            session.TryChangePlaybackPositionAsync(position_ms as i64 * 10_000)?.join()?
        }
    };
    if !accepted {
        log::debug!("source app declined {command:?}");
    }
    Ok(())
}

fn send(out: &Sender<Signal>, signal: Signal) {
    let _ = out.send(signal);
}

fn hash_str(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::image_mime;

    #[test]
    fn sniffs_image_types() {
        assert_eq!(image_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(image_mime(&[0x89, b'P', b'N', b'G', 13, 10]), Some("image/png"));
        assert_eq!(image_mime(b"RIFF1234WEBPVP8"), Some("image/webp"));
        assert_eq!(image_mime(b"<html>"), None);
        assert_eq!(image_mime(&[]), None);
    }
}
