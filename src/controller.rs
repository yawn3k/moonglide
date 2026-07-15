use std::collections::{HashMap, HashSet};

use sdl2::controller::{Axis, Button, GameController};
use sdl2::sensor::SensorType;
use sdl2::GameControllerSubsystem;

const ALL_BUTTONS: [Button; 21] = [
	Button::A, Button::B, Button::X, Button::Y,
	Button::Back, Button::Guide, Button::Start,
	Button::LeftStick, Button::RightStick,
	Button::LeftShoulder, Button::RightShoulder,
	Button::DPadUp, Button::DPadDown, Button::DPadLeft, Button::DPadRight,
	Button::Misc1, Button::Paddle1, Button::Paddle2, Button::Paddle3, Button::Paddle4,
	Button::Touchpad,
];

const ALL_AXES: [Axis; 6] = [
	Axis::LeftX, Axis::LeftY, Axis::RightX, Axis::RightY,
	Axis::TriggerLeft, Axis::TriggerRight,
];

pub fn idx_to_button_name(idx: u8) -> String {
	match idx {
		0 => "a", 1 => "b", 2 => "x", 3 => "y",
		4 => "back", 5 => "guide", 6 => "start",
		7 => "left_stick", 8 => "right_stick",
		9 => "left_shoulder", 10 => "right_shoulder",
		11 => "dpad_up", 12 => "dpad_down", 13 => "dpad_left", 14 => "dpad_right",
		15 => "misc_1", 16 => "paddle_1", 17 => "paddle_2", 18 => "paddle_3", 19 => "paddle_4",
		20 => "touchpad_click",
		_ => return format!("unknown_{}", idx),
	}.to_string()
}

pub struct ControllerManager {
	pub controllers: HashMap<u32, GameController>,
	pub touch_fingers: HashSet<(u32, u32)>,
	sub: GameControllerSubsystem,
}

#[derive(Debug, Clone)]
pub enum ControllerEvent {
	TouchpadTouch,
	TouchpadUntouch,
	Connected(u32),
	Disconnected(u32),
}

impl ControllerManager {
	pub fn open(sub: GameControllerSubsystem) -> Result<Self, String> {
		let mut mgr = Self {
			controllers: HashMap::new(),
			touch_fingers: HashSet::new(),
			sub,
		};
		let available = mgr
			.sub
			.num_joysticks()
			.map_err(|e| format!("num_joysticks: {}", e))?;
		for id in 0..available {
				if mgr.sub.is_game_controller(id) {
					match mgr.sub.open(id) {
						Ok(c) => {
							let instance = c.instance_id();
							enable_sensors(&c);
							mgr.controllers.insert(instance, c);
						}
						Err(e) => eprintln!("open controller {}: {}", id, e),
					}
				}
		}
		Ok(mgr)
	}

	pub fn connected_ids(&self) -> Vec<u32> {
		self.controllers.keys().copied().collect()
	}

	pub fn poll_buttons(&self, id: u32, prev: &mut u32) -> Vec<(u8, bool)> {
		let ctrl = match self.controllers.get(&id) {
			Some(c) => c,
			None => return Vec::new(),
		};

		let mut curr = 0u32;
		for (i, btn) in ALL_BUTTONS.iter().enumerate() {
			if ctrl.button(*btn) {
				curr |= 1 << i;
			}
		}

		let pressed = curr & !*prev;
		let released = *prev & !curr;
		*prev = curr;

		let mut out = Vec::new();
		for i in 0..21 {
			if pressed & (1 << i) != 0 {
				out.push((i as u8, true));
			}
			if released & (1 << i) != 0 {
				out.push((i as u8, false));
			}
		}
		out
	}

	pub fn poll_axes(&self, id: u32) -> Option<[i16; 6]> {
		let ctrl = self.controllers.get(&id)?;
		Some([
			ctrl.axis(ALL_AXES[0]),
			ctrl.axis(ALL_AXES[1]),
			ctrl.axis(ALL_AXES[2]),
			ctrl.axis(ALL_AXES[3]),
			ctrl.axis(ALL_AXES[4]),
			ctrl.axis(ALL_AXES[5]),
		])
	}

	pub fn poll_sensors(&self, id: u32) -> Option<(f32, f32, f32, f32, f32, f32)> {
		let ctrl = self.controllers.get(&id)?;
		let mut gbuf = [0.0f32; 3];
		let mut abuf = [0.0f32; 3];
		ctrl.sensor_get_data(SensorType::Gyroscope, &mut gbuf).ok()?;
		let _ = ctrl.sensor_get_data(SensorType::Accelerometer, &mut abuf);
		Some((gbuf[0], gbuf[1], gbuf[2], abuf[0], abuf[1], abuf[2]))
	}

	pub fn handle_event(&mut self, event: &sdl2::event::Event) -> Vec<ControllerEvent> {
		let mut out = Vec::new();

		match event {
			sdl2::event::Event::ControllerTouchpadDown { touchpad, finger, .. } => {
				let key = (*touchpad, *finger);
				if self.touch_fingers.is_empty() {
					out.push(ControllerEvent::TouchpadTouch);
				}
				self.touch_fingers.insert(key);
			}
			sdl2::event::Event::ControllerTouchpadUp { touchpad, finger, .. } => {
				let key = (*touchpad, *finger);
				self.touch_fingers.remove(&key);
				if self.touch_fingers.is_empty() {
					out.push(ControllerEvent::TouchpadUntouch);
				}
			}
			sdl2::event::Event::ControllerDeviceAdded { which, .. } => {
				if let Ok(c) = self.sub.open(*which) {
					let instance = c.instance_id();
					enable_sensors(&c);
					out.push(ControllerEvent::Connected(instance));
					self.controllers.insert(instance, c);
				}
			}
			sdl2::event::Event::ControllerDeviceRemoved { which, .. } => {
				out.push(ControllerEvent::Disconnected(*which));
				self.controllers.remove(which);
				self.touch_fingers.clear();
			}
			_ => {}
		}

		out
	}
}

fn enable_sensors(c: &GameController) {
	if c.has_sensor(sdl2::sensor::SensorType::Gyroscope) {
		let _ = c.sensor_set_enabled(sdl2::sensor::SensorType::Gyroscope, true);
	}
	if c.has_sensor(sdl2::sensor::SensorType::Accelerometer) {
		let _ = c.sensor_set_enabled(sdl2::sensor::SensorType::Accelerometer, true);
	}
}
