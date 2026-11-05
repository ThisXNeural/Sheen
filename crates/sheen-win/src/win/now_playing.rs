//! Publishes Sheen's built-in player to Windows' System Media Transport
//! Controls, the same API Spotify or a browser uses.
//!
//! This gives the built-in player hardware media keys, the Windows media
//! flyout and lock-screen controls. It also means the island shows it through
//! the ordinary SMTC watcher in `media.rs`, so there is one code path for every
//! source.

use super::init_com;
use crate::{MediaButton, NowPlaying};
use std::sync::mpsc::{channel, Sender};
use windows::core::{Ref, HSTRING};
use windows::Foundation::{TimeSpan, TypedEventHandler};
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, PlaybackPositionChangeRequestedEventArgs,
    SystemMediaTransportControls, SystemMediaTransportControlsButton as Button,
    SystemMediaTransportControlsButtonPressedEventArgs,
    SystemMediaTransportControlsTimelineProperties,
};
use windows::Storage::Streams::{
    DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::WinRT::ISystemMediaTransportControlsInterop;

pub struct NowPlayingHandle(Sender<NowPlaying>);

impl NowPlayingHandle {
    pub fn spawn(hwnd: isize, on_button: Box<dyn Fn(MediaButton) + Send + Sync>) -> Self {
        let (tx, rx) = channel::<NowPlaying>();
        std::thread::Builder::new()
            .name("sheen-now-playing".into())
            .spawn(move || {
                init_com();
                let smtc = match controls(hwnd, on_button) {
                    Ok(smtc) => smtc,
                    Err(e) => return log::error!("SMTC registration failed: {e:?}"),
                };
                let mut last_cover: Option<u64> = None;
                while let Ok(mut np) = rx.recv() {
                    // Coalesce bursts: only the newest state matters.
                    while let Ok(newer) = rx.try_recv() {
                        np = newer;
                    }
                    if let Err(e) = publish(&smtc, &np, &mut last_cover) {
                        log::warn!("SMTC update failed: {e:?}");
                    }
                }
            })
            .expect("spawn now-playing publisher");
        Self(tx)
    }

    pub fn update(&self, now_playing: NowPlaying) {
        let _ = self.0.send(now_playing);
    }
}

fn controls(
    hwnd: isize,
    on_button: Box<dyn Fn(MediaButton) + Send + Sync>,
) -> windows::core::Result<SystemMediaTransportControls> {
    let interop = windows::core::factory::<
        SystemMediaTransportControls,
        ISystemMediaTransportControlsInterop,
    >()?;
    let smtc: SystemMediaTransportControls =
        unsafe { interop.GetForWindow(HWND(hwnd as *mut core::ffi::c_void))? };

    smtc.SetIsPlayEnabled(true)?;
    smtc.SetIsPauseEnabled(true)?;
    smtc.SetIsNextEnabled(true)?;
    smtc.SetIsPreviousEnabled(true)?;
    smtc.SetIsEnabled(false)?;

    let on_button = std::sync::Arc::new(on_button);
    let cb = on_button.clone();
    smtc.ButtonPressed(&TypedEventHandler::new(
        move |_, args: Ref<SystemMediaTransportControlsButtonPressedEventArgs>| {
            let button = match args.ok()?.Button()? {
                Button::Play => MediaButton::Play,
                Button::Pause => MediaButton::Pause,
                Button::Next => MediaButton::Next,
                Button::Previous => MediaButton::Previous,
                _ => return Ok(()),
            };
            cb(button);
            Ok(())
        },
    ))?;
    smtc.PlaybackPositionChangeRequested(&TypedEventHandler::new(
        move |_, args: Ref<PlaybackPositionChangeRequestedEventArgs>| {
            let ticks = args.ok()?.RequestedPlaybackPosition()?.Duration;
            on_button(MediaButton::Seek((ticks.max(0) / 10_000) as u64));
            Ok(())
        },
    ))?;
    Ok(smtc)
}

fn publish(
    smtc: &SystemMediaTransportControls,
    np: &NowPlaying,
    last_cover: &mut Option<u64>,
) -> windows::core::Result<()> {
    let Some(track) = &np.track else {
        smtc.SetPlaybackStatus(MediaPlaybackStatus::Stopped)?;
        smtc.DisplayUpdater()?.ClearAll()?;
        smtc.SetIsEnabled(false)?;
        *last_cover = None;
        return Ok(());
    };

    smtc.SetIsEnabled(true)?;
    smtc.SetPlaybackStatus(if np.playing {
        MediaPlaybackStatus::Playing
    } else {
        MediaPlaybackStatus::Paused
    })?;

    // Metadata and art only when the track (or its art) actually changed.
    if *last_cover != Some(track.key) {
        let display = smtc.DisplayUpdater()?;
        display.SetType(MediaPlaybackType::Music)?;
        let music = display.MusicProperties()?;
        music.SetTitle(&HSTRING::from(&track.title))?;
        music.SetArtist(&HSTRING::from(&track.artist))?;
        music.SetAlbumTitle(&HSTRING::from(&track.album))?;
        match &track.cover {
            Some(bytes) => display.SetThumbnail(&stream_reference(bytes)?)?,
            None => display.SetThumbnail(None::<&RandomAccessStreamReference>)?,
        }
        display.Update()?;
        *last_cover = Some(track.key);
    }

    let timeline = SystemMediaTransportControlsTimelineProperties::new()?;
    let ts = |ms: u64| TimeSpan {
        Duration: ms as i64 * 10_000,
    };
    timeline.SetStartTime(ts(0))?;
    timeline.SetMinSeekTime(ts(0))?;
    timeline.SetPosition(ts(np.position_ms))?;
    timeline.SetEndTime(ts(track.duration_ms))?;
    timeline.SetMaxSeekTime(ts(track.duration_ms))?;
    smtc.UpdateTimelineProperties(&timeline)?;
    Ok(())
}

fn stream_reference(bytes: &[u8]) -> windows::core::Result<RandomAccessStreamReference> {
    let writer = DataWriter::new()?;
    writer.WriteBytes(bytes)?;
    let buffer = writer.DetachBuffer()?;
    let stream = InMemoryRandomAccessStream::new()?;
    stream.WriteAsync(&buffer)?.join()?;
    stream.Seek(0)?;
    RandomAccessStreamReference::CreateFromStream(&stream)
}
