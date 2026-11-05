//! File Explorer's copy/move dialog ("OperationStatusWindow"). Its contents
//! are DirectUI, not Win32 controls, so the progress bars are read through UI
//! Automation. The same dialog handles USB drives, network shares and phones
//! (MTP), so all of those are covered.

use super::windows_list::{Snapshot, Win};
use super::{strip_percent, transfer_id};
use sheen_core::{TransferKind, TransferSample, TransferUnit};
use std::collections::HashMap;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::Variant::{VARIANT, VT_I4};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationRangeValuePattern,
    TreeScope_Descendants, UIA_ControlTypePropertyId, UIA_ProgressBarControlTypeId,
    UIA_RangeValuePatternId,
};

/// Re-discover progress bars this often (in polls); in between, only their
/// values are read, which is a single cross-process call each.
const REFRESH_EVERY: u32 = 5;

pub struct ExplorerProvider {
    uia: Option<IUIAutomation>,
    bars: HashMap<isize, (u32, Vec<IUIAutomationElement>)>,
}

impl ExplorerProvider {
    pub fn new() -> Self {
        let uia = unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }
            .map_err(|e| log::warn!("UI Automation unavailable: {e}"))
            .ok();
        Self { uia, bars: HashMap::new() }
    }

    pub fn poll(&mut self, snap: &Snapshot) -> Vec<TransferSample> {
        let Some(uia) = &self.uia else { return Vec::new() };
        let dialogs: Vec<&Win> =
            snap.windows.iter().filter(|w| w.class == "OperationStatusWindow").collect();
        self.bars.retain(|h, _| dialogs.iter().any(|w| w.hwnd.0 as isize == *h));

        let mut out = Vec::new();
        for w in dialogs {
            let key = w.hwnd.0 as isize;
            let entry = self.bars.entry(key).or_insert((0, Vec::new()));
            if entry.0.is_multiple_of(REFRESH_EVERY) || entry.1.is_empty() {
                entry.1 = find_bars(uia, w).unwrap_or_default();
            }
            entry.0 += 1;

            let name = match strip_percent(&w.title) {
                n if n.is_empty() || n.eq_ignore_ascii_case("complete") => "Files".to_string(),
                n => n,
            };
            for (i, bar) in entry.1.iter().enumerate() {
                let Some((value, max)) = read_bar(bar) else { continue };
                if max <= 0.0 {
                    continue;
                }
                // Scale to a fine integer range; the dialog reports 0..100
                // with fractions.
                out.push(TransferSample {
                    id: transfer_id(("explorer", key, i)),
                    kind: TransferKind::Copy,
                    app: "File Explorer".into(),
                    name: name.clone(),
                    done: (value / max * 10_000.0).round() as u64,
                    total: Some(10_000),
                    unit: TransferUnit::Ratio,
                    source_in_view: snap.in_view(w),
                });
            }
        }
        out
    }
}

fn find_bars(uia: &IUIAutomation, w: &Win) -> windows::core::Result<Vec<IUIAutomationElement>> {
    unsafe {
        let root = uia.ElementFromHandle(w.hwnd)?;
        let mut v = VARIANT::default();
        (*v.Anonymous.Anonymous).vt = VT_I4;
        (*v.Anonymous.Anonymous).Anonymous.lVal = UIA_ProgressBarControlTypeId.0;
        let cond = uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &v)?;
        let found = root.FindAll(TreeScope_Descendants, &cond)?;
        (0..found.Length()?).map(|i| found.GetElement(i)).collect()
    }
}

fn read_bar(bar: &IUIAutomationElement) -> Option<(f64, f64)> {
    unsafe {
        let range: IUIAutomationRangeValuePattern =
            bar.GetCurrentPatternAs(UIA_RangeValuePatternId).ok()?;
        let min = range.CurrentMinimum().ok()?;
        let max = range.CurrentMaximum().ok()?;
        let value = range.CurrentValue().ok()?;
        Some((value - min, max - min))
    }
}
