use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEINPUT, MOUSEEVENTF_MOVE, MOUSEEVENTF_WHEEL,
};

pub struct VirtualMouse {
    accum_x: f64,
    accum_y: f64,
}

impl VirtualMouse {
    pub fn new() -> Result<Self, String> {
        Ok(Self { accum_x: 0.0, accum_y: 0.0 })
    }

    pub fn move_mouse(&mut self, dx: f64, dy: f64) -> Result<(), String> {
        self.accum_x += dx;
        self.accum_y += dy;
        let ix = self.accum_x as i32;
        let iy = self.accum_y as i32;
        if ix == 0 && iy == 0 {
            return Ok(());
        }
        self.accum_x -= ix as f64;
        self.accum_y -= iy as f64;

        let mi = MOUSEINPUT {
            dx: ix,
            dy: iy,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_MOVE,
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

    pub fn scroll(&mut self, amount: i32) -> Result<(), String> {
        let mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: (amount * 120) as u32,
            dwFlags: MOUSEEVENTF_WHEEL,
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

    pub fn synchronize(&mut self) -> Result<(), String> {
        Ok(())
    }
}

pub fn send_mouse_button(dw_flags: u32) -> Result<(), String> {
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
