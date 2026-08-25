use crate::output::keyboard::VirtualKeyboard;
use crate::output::mouse::VirtualMouse;

/// Report a device write failure once per 5 s (repeats suppressed, hot-loop safe).
pub(crate) fn report(device: &str, result: Result<(), String>) {
	if let Err(e) = result {
		crate::style::err_once(&format!("uinput-{device}"), &format!("{device} output error: {e}"));
	}
}

pub struct OutputDevices {
	pub mouse: Option<VirtualMouse>,
	pub kbd: Option<VirtualKeyboard>,
}

impl OutputDevices {
	fn scroll(&mut self, dir: i32) {
		if let Some(m) = self.mouse.as_mut() {
			report("mouse", m.scroll(dir));
		}
	}

	pub fn move_mouse(&mut self, dx: f64, dy: f64) {
		if let Some(m) = self.mouse.as_mut() {
			report("mouse", m.move_mouse(dx, dy));
		}
	}

	pub fn apply(&mut self, key: &str, press: bool) {
		match key {
			"wheel_up" if press => self.scroll(1),
			"wheel_down" if press => self.scroll(-1),
			"wheel_up" | "wheel_down" => {}  // no-op on release
			"left_mouse" | "right_mouse" | "middle_mouse" => {
				if let Some(ref mut kbd) = self.kbd {
					let n = match key { "left_mouse" => 1, "right_mouse" => 2, _ => 3 };
					report("keyboard", if press { kbd.press_mouse(n) } else { kbd.release_mouse(n) });
				}
			}
			_ => {
				if let Some(ref mut kbd) = self.kbd {
					report("keyboard", if press { kbd.press(key) } else { kbd.release(key) });
				}
			}
		}
	}

	pub fn synchronize_all(&mut self) {
		if let Some(ref mut m) = self.mouse { report("mouse", m.synchronize()); }
		if let Some(ref mut k) = self.kbd { report("keyboard", k.synchronize()); }
	}
}
