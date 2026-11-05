//! Shelf drag-and-drop helpers.

use crate::DropOutcome;
use std::path::PathBuf;
use windows::core::{w, HSTRING};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::Ole::{
    DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE, DROPEFFECT_NONE,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    BHID_DataObject, IShellItemArray, SHCreateShellItemArrayFromIDLists, SHDoDragDrop,
    SHParseDisplayName,
};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindowVisible};

/// A shell drag (files from Explorer, the desktop, a browser, ...) is in
/// progress. Shell drags show their image in a `SysDragImage` window while
/// the left button is held, and that is cheap to detect.
pub fn file_drag_active() -> bool {
    unsafe {
        if GetAsyncKeyState(VK_LBUTTON.0 as i32) >= 0 {
            return false;
        }
        FindWindowW(w!("SysDragImage"), None).is_ok_and(|h| IsWindowVisible(h).as_bool())
    }
}

/// Left or right mouse button is held right now (anywhere on the desktop).
pub fn mouse_button_down() -> bool {
    unsafe {
        GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 || GetAsyncKeyState(VK_RBUTTON.0 as i32) < 0
    }
}

/// Start a shell drag of `paths`. Must run on the UI (STA) thread while the
/// mouse button is down; it returns when the drop finishes. The shell
/// provides the drag image and data formats, so the files behave exactly as
/// if dragged from Explorer.
pub fn drag_files_out(paths: &[PathBuf]) -> windows::core::Result<DropOutcome> {
    unsafe {
        let mut pidls: Vec<*mut ITEMIDLIST> = Vec::with_capacity(paths.len());
        for p in paths {
            let mut pidl = std::ptr::null_mut();
            if SHParseDisplayName(&HSTRING::from(p.as_os_str()), None, &mut pidl, 0, None).is_ok() {
                pidls.push(pidl);
            }
        }
        let result = (|| {
            let list: Vec<*const ITEMIDLIST> = pidls.iter().map(|p| *p as *const _).collect();
            let items: IShellItemArray = SHCreateShellItemArrayFromIDLists(&list)?;
            let data: windows::Win32::System::Com::IDataObject =
                items.BindToHandler(None, &BHID_DataObject)?;
            let effect = SHDoDragDrop(
                None,
                &data,
                None,
                DROPEFFECT_COPY | DROPEFFECT_MOVE | DROPEFFECT_LINK,
            )?;
            Ok(if effect == DROPEFFECT_NONE {
                DropOutcome::Cancelled
            } else if effect.0 & DROPEFFECT_MOVE.0 != 0 {
                DropOutcome::Moved
            } else if effect.0 & DROPEFFECT_LINK.0 != 0 {
                DropOutcome::Linked
            } else {
                DropOutcome::Copied
            })
        })();
        for pidl in pidls {
            CoTaskMemFree(Some(pidl as *const _));
        }
        result
    }
}
