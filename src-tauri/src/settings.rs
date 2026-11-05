//! User settings, persisted as JSON in the app config directory.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemePref {
    /// Follow the Windows app theme (Settings → Personalization → Colors).
    #[default]
    System,
    Light,
    Dark,
}

impl ThemePref {
    pub fn as_tauri(self) -> Option<tauri::Theme> {
        match self {
            ThemePref::System => None,
            ThemePref::Light => Some(tauri::Theme::Light),
            ThemePref::Dark => Some(tauri::Theme::Dark),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: ThemePref,
    pub music_dir: Option<PathBuf>,
    pub player_volume: f32,
    /// Global shortcut for the quick note ("" disables it).
    pub note_shortcut: String,
    /// Show a peek in the island when something is copied.
    pub clipboard_peek: bool,
    /// Show background downloads and file copies in the island.
    pub show_transfers: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemePref::System,
            music_dir: None,
            player_volume: 0.8,
            note_shortcut: "ScrollLock".into(),
            clipboard_peek: true,
            show_transfers: true,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        let result = (|| -> std::io::Result<()> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(path, serde_json::to_vec_pretty(self)?)
        })();
        if let Err(e) = result {
            log::warn!("could not save settings: {e}");
        }
    }
}
