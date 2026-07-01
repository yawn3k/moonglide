use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::output::keyboard::VirtualKeyboard;
use crate::output::mouse::VirtualMouse;
use crate::log_msg;
use crate::TRIGGER_THRESHOLD;

pub struct TriggerTracker {
	left: HashMap<u32, bool>,
	right: HashMap<u32, bool>,
	last_left: Instant,
	last_right: Instant,
}

impl TriggerTracker {
	pub fn new() -> Self {
		Self {
			left: HashMap::new(),
			right: HashMap::new(),
			last_left: Instant::now(),
			last_right: Instant::now(),
		}
	}

	pub fn process(&mut self, which: u32, idx: u8, val: i16) -> Option<(String, bool)> {
		let thresh = TRIGGER_THRESHOLD.load(Ordering::Relaxed) as i16;
		let (map, name, last) = match idx {
			105 => (&mut self.left, "left_trigger", &mut self.last_left),
			106 => (&mut self.right, "right_trigger", &mut self.last_right),
			_ => return None,
		};
		let pressed = map.entry(which).or_insert(false);
		let new = val > thresh;
		if new != *pressed {
			*pressed = new;
			if last.elapsed() > Duration::from_millis(50) {
				*last = Instant::now();
				log_msg(1, &format!("{} {}", name, if new { "down" } else { "up" }));
				return Some((name.to_string(), new));
			}
		}
		None
	}


}

pub struct OutputDevices {
	pub mouse: Option<VirtualMouse>,
	pub kbd: Option<VirtualKeyboard>,
}

impl OutputDevices {
	pub fn apply(&mut self, key: &str, press: bool) {
		if let Some(ref mut kbd) = self.kbd {
			match key {
				"left_mouse" | "right_mouse" | "middle_mouse" => {
					let n = match key { "left_mouse" => 1, "right_mouse" => 2, _ => 3 };
					if press { let _ = kbd.press_mouse(n); } else { let _ = kbd.release_mouse(n); }
				}
				_ => {
					if press { let _ = kbd.press(key); } else { let _ = kbd.release(key); }
				}
			}
			let _ = kbd.synchronize();
		}
	}

	pub fn synchronize_all(&mut self) {
		if let Some(ref mut m) = self.mouse { let _ = m.synchronize(); }
		if let Some(ref mut k) = self.kbd { let _ = k.synchronize(); }
	}
}
