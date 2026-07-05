use std::collections::HashMap;

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_SCANCODE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
};

use crate::output::mouse::send_mouse_button;

fn build_key_map() -> HashMap<&'static str, u16> {
    HashMap::from([
        ("esc", 0x01), ("1", 0x02), ("2", 0x03), ("3", 0x04),
        ("4", 0x05), ("5", 0x06), ("6", 0x07), ("7", 0x08),
        ("8", 0x09), ("9", 0x0a), ("0", 0x0b),
        ("minus", 0x0c), ("equal", 0x0d),
        ("backspace", 0x0e), ("tab", 0x0f),
        ("q", 0x10), ("w", 0x11), ("e", 0x12), ("r", 0x13),
        ("t", 0x14), ("y", 0x15), ("u", 0x16), ("i", 0x17),
        ("o", 0x18), ("p", 0x19),
        ("leftbrace", 0x1a), ("rightbrace", 0x1b),
        ("enter", 0x1c), ("left_control", 0x1d),
        ("a", 0x1e), ("s", 0x1f), ("d", 0x20), ("f", 0x21),
        ("g", 0x22), ("h", 0x23), ("j", 0x24), ("k", 0x25),
        ("l", 0x26),
        ("semicolon", 0x27), ("apostrophe", 0x28), ("grave", 0x29),
        ("left_shift", 0x2a), ("backslash", 0x2b),
        ("z", 0x2c), ("x", 0x2d), ("c", 0x2e), ("v", 0x2f),
        ("b", 0x30), ("n", 0x31), ("m", 0x32),
        ("comma", 0x33), ("dot", 0x34), ("slash", 0x35),
        ("right_shift", 0x36), ("left_alt", 0x38), ("space", 0x39),
        ("caps_lock", 0x3a),
        ("f1", 0x3b), ("f2", 0x3c), ("f3", 0x3d), ("f4", 0x3e),
        ("f5", 0x3f), ("f6", 0x40), ("f7", 0x41), ("f8", 0x42),
        ("f9", 0x43), ("f10", 0x44), ("f11", 0x57), ("f12", 0x58),
        ("num_lock", 0x45), ("scroll_lock", 0x46),
        ("right_control", 0xe0_1d), ("sysrq", 0x37), ("right_alt", 0xe0_38),
        ("home", 0xe0_47), ("up", 0xe0_48), ("page_up", 0xe0_49),
        ("left", 0xe0_4b), ("right", 0xe0_4d),
        ("end", 0xe0_4f), ("down", 0xe0_50), ("page_down", 0xe0_51),
        ("insert", 0xe0_52), ("delete", 0xe0_53),
        ("left_meta", 0xe0_5b), ("right_meta", 0xe0_5c),
    ])
}

pub struct VirtualKeyboard {
    key_map: HashMap<&'static str, u16>,
}

impl VirtualKeyboard {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            key_map: build_key_map(),
        })
    }

    pub fn press(&mut self, key_name: &str) -> Result<(), String> {
        let sc = self.key_map.get(key_name.to_lowercase().as_str())
            .ok_or_else(|| format!("unknown key: {}", key_name))?;
        let (scancode, extended) = split_sc(*sc);
        send_key(scancode, extended, false)
    }

    pub fn release(&mut self, key_name: &str) -> Result<(), String> {
        let sc = self.key_map.get(key_name.to_lowercase().as_str())
            .ok_or_else(|| format!("unknown key: {}", key_name))?;
        let (scancode, extended) = split_sc(*sc);
        send_key(scancode, extended, true)
    }

    pub fn press_mouse(&mut self, btn: u8) -> Result<(), String> {
        let flags = match btn {
            1 => MOUSEEVENTF_LEFTDOWN,
            2 => MOUSEEVENTF_RIGHTDOWN,
            3 => MOUSEEVENTF_MIDDLEDOWN,
            _ => return Err(format!("unknown mouse button {}", btn)),
        };
        send_mouse_button(flags)
    }

    pub fn release_mouse(&mut self, btn: u8) -> Result<(), String> {
        let flags = match btn {
            1 => MOUSEEVENTF_LEFTUP,
            2 => MOUSEEVENTF_RIGHTUP,
            3 => MOUSEEVENTF_MIDDLEUP,
            _ => return Err(format!("unknown mouse button {}", btn)),
        };
        send_mouse_button(flags)
    }

    pub fn synchronize(&mut self) -> Result<(), String> {
        Ok(())
    }
}

fn split_sc(sc: u16) -> (u16, bool) {
    let extended = sc >> 8 == 0xe0;
    let scancode = if extended { sc & 0xff } else { sc };
    (scancode, extended)
}

fn send_key(scancode: u16, extended: bool, release: bool) -> Result<(), String> {
    let mut dw_flags = KEYEVENTF_SCANCODE;
    if release {
        dw_flags |= KEYEVENTF_KEYUP;
    }
    if extended {
        dw_flags |= 0x0001; // KEYEVENTF_EXTENDEDKEY
    }
    let ki = KEYBDINPUT {
        wVk: 0,
        wScan: scancode,
        dwFlags: dw_flags,
        time: 0,
        dwExtraInfo: 0,
    };
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki },
    };
    unsafe {
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}


