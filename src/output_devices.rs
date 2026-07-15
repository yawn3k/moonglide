use crate::output::keyboard::VirtualKeyboard;
use crate::output::mouse::VirtualMouse;

pub struct OutputDevices {
	pub mouse: Option<VirtualMouse>,
	pub kbd: Option<VirtualKeyboard>,
}

impl OutputDevices {
	pub fn apply(&mut self, key: &str, press: bool) {
		match key {
			"wheel_up" if press => { let _ = self.mouse.as_mut().map(|m| m.scroll(1)); }
			"wheel_down" if press => { let _ = self.mouse.as_mut().map(|m| m.scroll(-1)); }
			"wheel_up" | "wheel_down" => {}  // no-op on release
			"left_mouse" | "right_mouse" | "middle_mouse" => {
				if let Some(ref mut kbd) = self.kbd {
					let n = match key { "left_mouse" => 1, "right_mouse" => 2, _ => 3 };
					if press { let _ = kbd.press_mouse(n); } else { let _ = kbd.release_mouse(n); }
				}
			}
			_ => {
				if let Some(ref mut kbd) = self.kbd {
					if press { let _ = kbd.press(key); } else { let _ = kbd.release(key); }
				}
			}
		}
	}

	pub fn synchronize_all(&mut self) {
		if let Some(ref mut m) = self.mouse { let _ = m.synchronize(); }
		if let Some(ref mut k) = self.kbd { let _ = k.synchronize(); }
	}
}
