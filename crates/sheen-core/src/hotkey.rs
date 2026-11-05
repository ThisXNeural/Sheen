//! Global shortcut strings ("Ctrl+Alt+N", "ScrollLock") and their Win32
//! representation (modifier flags and a virtual-key code).

use std::fmt;

pub const MOD_ALT: u32 = 0x1;
pub const MOD_CONTROL: u32 = 0x2;
pub const MOD_SHIFT: u32 = 0x4;
pub const MOD_WIN: u32 = 0x8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accelerator {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
    pub vk: u16,
}

/// Named keys and their virtual-key codes. Letters and digits are handled
/// separately.
const KEYS: &[(&str, u16)] = &[
    ("Backspace", 0x08),
    ("Tab", 0x09),
    ("Enter", 0x0D),
    ("Pause", 0x13),
    ("CapsLock", 0x14),
    ("Escape", 0x1B),
    ("Space", 0x20),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("End", 0x23),
    ("Home", 0x24),
    ("Left", 0x25),
    ("Up", 0x26),
    ("Right", 0x27),
    ("Down", 0x28),
    ("PrintScreen", 0x2C),
    ("Insert", 0x2D),
    ("Delete", 0x2E),
    ("NumLock", 0x90),
    ("ScrollLock", 0x91),
    ("Numpad0", 0x60),
    ("Numpad1", 0x61),
    ("Numpad2", 0x62),
    ("Numpad3", 0x63),
    ("Numpad4", 0x64),
    ("Numpad5", 0x65),
    ("Numpad6", 0x66),
    ("Numpad7", 0x67),
    ("Numpad8", 0x68),
    ("Numpad9", 0x69),
    ("NumpadMultiply", 0x6A),
    ("NumpadAdd", 0x6B),
    ("NumpadSubtract", 0x6D),
    ("NumpadDecimal", 0x6E),
    ("NumpadDivide", 0x6F),
    (";", 0xBA),
    ("=", 0xBB),
    (",", 0xBC),
    ("-", 0xBD),
    (".", 0xBE),
    ("/", 0xBF),
    ("`", 0xC0),
    ("[", 0xDB),
    ("\\", 0xDC),
    ("]", 0xDD),
    ("'", 0xDE),
];

fn key_name(vk: u16) -> Option<String> {
    match vk {
        0x30..=0x39 | 0x41..=0x5A => Some((vk as u8 as char).to_string()),
        0x70..=0x87 => Some(format!("F{}", vk - 0x6F)),
        _ => KEYS.iter().find(|(_, v)| *v == vk).map(|(n, _)| n.to_string()),
    }
}

fn key_code(name: &str) -> Option<u16> {
    let upper = name.to_ascii_uppercase();
    if upper.len() == 1 {
        let c = upper.as_bytes()[0];
        if c.is_ascii_alphanumeric() {
            return Some(c as u16);
        }
    }
    if let Some(n) = upper.strip_prefix('F').and_then(|n| n.parse::<u16>().ok()) {
        if (1..=24).contains(&n) {
            return Some(0x6F + n);
        }
    }
    KEYS.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| *v)
}

impl Accelerator {
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut acc = Accelerator { ctrl: false, alt: false, shift: false, win: false, vk: 0 };
        let parts: Vec<&str> = s.split('+').map(str::trim).collect();
        // "Ctrl++" style is not supported; '+' is reached with "=" plus Shift.
        if parts.iter().any(|p| p.is_empty()) {
            return Err(format!("\"{s}\" is not a valid shortcut"));
        }
        let (key, mods) = parts.split_last().ok_or("empty shortcut")?;
        for m in mods {
            match m.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => acc.ctrl = true,
                "alt" => acc.alt = true,
                "shift" => acc.shift = true,
                "win" | "super" | "meta" => acc.win = true,
                other => return Err(format!("unknown modifier \"{other}\"")),
            }
        }
        acc.vk = key_code(key).ok_or_else(|| format!("unknown key \"{key}\""))?;
        if !acc.has_modifier() && !acc.allowed_alone() {
            return Err(format!(
                "\"{key}\" needs Ctrl, Alt, Shift or Win so it doesn't block typing"
            ));
        }
        Ok(acc)
    }

    fn has_modifier(&self) -> bool {
        self.ctrl || self.alt || self.shift || self.win
    }

    /// Keys that are safe to grab without a modifier.
    fn allowed_alone(&self) -> bool {
        matches!(self.vk, 0x70..=0x87 | 0x13 | 0x91)
    }

    pub fn modifiers(&self) -> u32 {
        (if self.alt { MOD_ALT } else { 0 })
            | (if self.ctrl { MOD_CONTROL } else { 0 })
            | (if self.shift { MOD_SHIFT } else { 0 })
            | (if self.win { MOD_WIN } else { 0 })
    }
}

impl fmt::Display for Accelerator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (on, name) in
            [(self.ctrl, "Ctrl"), (self.alt, "Alt"), (self.shift, "Shift"), (self.win, "Win")]
        {
            if on {
                write!(f, "{name}+")?;
            }
        }
        f.write_str(&key_name(self.vk).unwrap_or_else(|| format!("0x{:02X}", self.vk)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalises() {
        let a = Accelerator::parse("alt+ctrl+n").unwrap();
        assert_eq!(a.vk, b'N' as u16);
        assert_eq!(a.modifiers(), MOD_ALT | MOD_CONTROL);
        assert_eq!(a.to_string(), "Ctrl+Alt+N");
    }

    #[test]
    fn standalone_keys() {
        assert_eq!(Accelerator::parse("ScrollLock").unwrap().vk, 0x91);
        assert_eq!(Accelerator::parse("F8").unwrap().to_string(), "F8");
        assert!(Accelerator::parse("N").is_err(), "plain letters would block typing");
        assert!(Accelerator::parse("Space").is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(Accelerator::parse("").is_err());
        assert!(Accelerator::parse("Ctrl+").is_err());
        assert!(Accelerator::parse("Hyper+K").is_err());
        assert!(Accelerator::parse("Ctrl+Nope").is_err());
    }

    #[test]
    fn round_trips_every_named_key() {
        for (name, _) in KEYS {
            let s = format!("Ctrl+{name}");
            assert_eq!(Accelerator::parse(&s).unwrap().to_string(), s);
        }
        for s in ["Shift+Win+Space", "Ctrl+F12", "Alt+5"] {
            assert_eq!(Accelerator::parse(s).unwrap().to_string(), s);
        }
    }
}
