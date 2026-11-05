//! Volume OSD and scroll-to-change-volume via Core Audio.
//!
//! Listens to the default render endpoint's volume/mute changes (from any
//! source: keyboard media keys, the tray, other apps, or our own scroll
//! handler) and re-binds when the default device changes (e.g. headphones
//! plugged in).

use super::init_com;
use crate::Signal;
use sheen_core::{OsdKind, SheenEvent};
use std::sync::mpsc::{channel, Sender};
use windows::core::{implement, Result, PCWSTR};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    eConsole, eRender, EDataFlow, ERole, IMMDeviceEnumerator, IMMNotificationClient,
    IMMNotificationClient_Impl, MMDeviceEnumerator, AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};

enum Msg {
    Rebind,
    Adjust(f32),
}

pub struct AudioHandle(Sender<Msg>);

impl AudioHandle {
    pub fn spawn(out: Sender<Signal>) -> Self {
        let (tx, rx) = channel();
        let me = tx.clone();
        std::thread::Builder::new()
            .name("sheen-audio".into())
            .spawn(move || {
                init_com();
                let run = || -> Result<()> {
                    let enumerator: IMMDeviceEnumerator =
                        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
                    let client: IMMNotificationClient = DeviceWatcher(me).into();
                    unsafe { enumerator.RegisterEndpointNotificationCallback(&client)? };

                    let mut endpoint = bind(&enumerator, &out);
                    while let Ok(msg) = rx.recv() {
                        match msg {
                            Msg::Rebind => {
                                drop(endpoint.take());
                                endpoint = bind(&enumerator, &out);
                            }
                            Msg::Adjust(delta) => {
                                if let Some(endpoint) = &endpoint {
                                    if let Err(e) = adjust(&endpoint.volume, delta) {
                                        log::warn!("volume change failed: {e}");
                                    }
                                }
                            }
                        }
                    }
                    Ok(())
                };
                if let Err(e) = run() {
                    log::error!("audio service stopped: {e:?}");
                }
            })
            .expect("spawn audio service");
        Self(tx)
    }

    pub fn adjust(&self, delta: f32) {
        let _ = self.0.send(Msg::Adjust(delta));
    }
}

/// The default endpoint plus the callback registered on it, unregistered on
/// drop so a re-bind does not leave a stale listener on the old device.
struct Endpoint {
    volume: IAudioEndpointVolume,
    callback: IAudioEndpointVolumeCallback,
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        let _ = unsafe { self.volume.UnregisterControlChangeNotify(&self.callback) };
    }
}

fn bind(enumerator: &IMMDeviceEnumerator, out: &Sender<Signal>) -> Option<Endpoint> {
    let result = (|| -> Result<Endpoint> {
        let device = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole)? };
        let volume: IAudioEndpointVolume = unsafe { device.Activate(CLSCTX_ALL, None)? };
        let callback: IAudioEndpointVolumeCallback = VolumeWatcher(out.clone()).into();
        unsafe { volume.RegisterControlChangeNotify(&callback)? };
        Ok(Endpoint { volume, callback })
    })();
    result.map_err(|e| log::warn!("no default audio endpoint: {e}")).ok()
}

fn adjust(volume: &IAudioEndpointVolume, delta: f32) -> Result<()> {
    unsafe {
        let level = (volume.GetMasterVolumeLevelScalar()? + delta).clamp(0.0, 1.0);
        if delta > 0.0 && volume.GetMute()?.as_bool() {
            volume.SetMute(false, std::ptr::null())?;
        }
        volume.SetMasterVolumeLevelScalar(level, std::ptr::null())
    }
}

#[implement(IAudioEndpointVolumeCallback)]
struct VolumeWatcher(Sender<Signal>);

impl IAudioEndpointVolumeCallback_Impl for VolumeWatcher_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> Result<()> {
        if let Some(data) = unsafe { data.as_ref() } {
            let osd = OsdKind::Volume { level: data.fMasterVolume, muted: data.bMuted.as_bool() };
            let _ = self.0.send(Signal::Event(SheenEvent::Osd(osd)));
        }
        Ok(())
    }
}

#[implement(IMMNotificationClient)]
struct DeviceWatcher(Sender<Msg>);

impl IMMNotificationClient_Impl for DeviceWatcher_Impl {
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, _id: &PCWSTR) -> Result<()> {
        if flow == eRender && role == eConsole {
            let _ = self.0.send(Msg::Rebind);
        }
        Ok(())
    }
    fn OnDeviceStateChanged(&self, _id: &PCWSTR, _state: DEVICE_STATE) -> Result<()> {
        Ok(())
    }
    fn OnDeviceAdded(&self, _id: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> Result<()> {
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _id: &PCWSTR, _key: &PROPERTYKEY) -> Result<()> {
        Ok(())
    }
}
