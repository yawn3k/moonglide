use std::collections::HashMap;

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_SCANCODE, MOUSEINPUT, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
};

fn build_key_map() -> HashMap<&'static str, u16> {
    let mut m = HashMap::new();
    m.insert("esc", 0x01);
    m.insert("1", 0x02);
    m.insert("2", 0x03);
    m.insert("3", 0x04);
    m.insert("4", 0x05);
    m.insert("5", 0x06);
    m.insert("6", 0x07);
    m.insert("7", 0x08);
    m.insert("8", 0x09);
    m.insert("9", 0x0a);
    m.insert("0", 0x0b);
    m.insert("minus", 0x0c);
    m.insert("equal", 0x0d);
    m.insert("backspace", 0x0e);
    m.insert("tab", 0x0f);
    m.insert("q", 0x10);
    m.insert("w", 0x11);
    m.insert("e", 0x12);
    m.insert("r", 0x13);
    m.insert("t", 0x14);
    m.insert("y", 0x15);
    m.insert("u", 0x16);
    m.insert("i", 0x17);
    m.insert("o", 0x18);
    m.insert("p", 0x19);
    m.insert("leftbrace", 0x1a);
    m.insert("rightbrace", 0x1b);
    m.insert("enter", 0x1c);
    m.insert("left_control", 0x1d);
    m.insert("a", 0x1e);
    m.insert("s", 0x1f);
    m.insert("d", 0x20);
    m.insert("f", 0x21);
    m.insert("g", 0x22);
    m.insert("h", 0x23);
    m.insert("j", 0x24);
    m.insert("k", 0x25);
    m.insert("l", 0x26);
    m.insert("semicolon", 0x27);
    m.insert("apostrophe", 0x28);
    m.insert("grave", 0x29);
    m.insert("left_shift", 0x2a);
    m.insert("backslash", 0x2b);
    m.insert("z", 0x2c);
    m.insert("x", 0x2d);
    m.insert("c", 0x2e);
    m.insert("v", 0x2f);
    m.insert("b", 0x30);
    m.insert("n", 0x31);
    m.insert("m", 0x32);
    m.insert("comma", 0x33);
    m.insert("dot", 0x34);
    m.insert("slash", 0x35);
    m.insert("right_shift", 0x36);
    m.insert("left_alt", 0x38);
    m.insert("space", 0x39);
    m.insert("caps_lock", 0x3a);
    m.insert("f1", 0x3b);
    m.insert("f2", 0x3c);
    m.insert("f3", 0x3d);
    m.insert("f4", 0x3e);
    m.insert("f5", 0x3f);
    m.insert("f6", 0x40);
    m.insert("f7", 0x41);
    m.insert("f8", 0x42);
    m.insert("f9", 0x43);
    m.insert("f10", 0x44);
    m.insert("f11", 0x57);
    m.insert("f12", 0x58);
    m.insert("num_lock", 0x45);
    m.insert("scroll_lock", 0x46);
    m.insert("right_control", 0xe0_1d);
    m.insert("sysrq", 0x37);
    m.insert("right_alt", 0xe0_38);
    m.insert("home", 0xe0_47);
    m.insert("up", 0xe0_48);
    m.insert("page_up", 0xe0_49);
    m.insert("left", 0xe0_4b);
    m.insert("right", 0xe0_4d);
    m.insert("end", 0xe0_4f);
    m.insert("down", 0xe0_50);
    m.insert("page_down", 0xe0_51);
    m.insert("insert", 0xe0_52);
    m.insert("delete", 0xe0_53);
    m.insert("left_meta", 0xe0_5b);
    m.insert("right_meta", 0xe0_5c);
    m
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

fn send_mouse_button(dw_flags: u32) -> Result<(), String> {
    let mi = MOUSEINPUT {
        dx: 0,
        dy: 0,
        mouseData: 0,
        dwFlags: dw_flags,
        time: 0,
        dwExtraInfo: 0,
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 { mi },
    };
    unsafe {
        SendInput(1, &input, std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}
