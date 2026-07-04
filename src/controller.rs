use std::collections::{HashMap, HashSet};

use sdl2::controller::{Axis, GameController};
use sdl2::GameControllerSubsystem;

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
	ButtonDown(u8),
	ButtonUp(u8),
	AxisMotion(u8, i16, u32),
	TouchpadTouch,
	TouchpadUntouch,
	Gyro { x: f32, y: f32, z: f32 },
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
							gyro_enable(&c);
							mgr.controllers.insert(instance, c);
						}
						Err(e) => eprintln!("open controller {}: {}", id, e),
					}
				}
		}
		Ok(mgr)
	}

	pub fn handle_event(&mut self, event: &sdl2::event::Event) -> Vec<ControllerEvent> {
		let mut out = Vec::new();

		match event {
			sdl2::event::Event::ControllerButtonDown { which, button, .. } => {
				if self.controllers.contains_key(which) {
					out.push(ControllerEvent::ButtonDown(*button as u8));
				}
			}
			sdl2::event::Event::ControllerButtonUp { which, button, .. } => {
				if self.controllers.contains_key(which) {
					out.push(ControllerEvent::ButtonUp(*button as u8));
				}
			}
			sdl2::event::Event::ControllerAxisMotion { which, axis, value, .. } => {
				if self.controllers.contains_key(which) {
					let idx = axis_to_u8(*axis);
					out.push(ControllerEvent::AxisMotion(idx, *value, *which));
				}
			}
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
			sdl2::event::Event::ControllerSensorUpdated { sensor, data, .. } => {
				if *sensor == sdl2::sensor::SensorType::Gyroscope {
					out.push(ControllerEvent::Gyro {
						x: data[0],
						y: data[1],
						z: data[2],
					});
				}
			}
			sdl2::event::Event::ControllerDeviceAdded { which, .. } => {
				if let Ok(c) = self.sub.open(*which) {
					let instance = c.instance_id();
					gyro_enable(&c);
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

fn gyro_enable(c: &GameController) {
	if c.has_sensor(sdl2::sensor::SensorType::Gyroscope) {
		let _ = c.sensor_set_enabled(sdl2::sensor::SensorType::Gyroscope, true);
	}
}

fn axis_to_u8(axis: Axis) -> u8 {
	match axis {
		Axis::LeftX => 101,
		Axis::LeftY => 102,
		Axis::RightX => 103,
		Axis::RightY => 104,
		Axis::TriggerLeft => 105,
		Axis::TriggerRight => 106,
	}
}
