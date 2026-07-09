use std::time::Instant;

use crate::bindings::GyroConfig;
use crate::gyro::GyroProcessor;
use crate::output_devices::OutputDevices;
use crate::style;

pub struct GyroState {
	processor: GyroProcessor,
	active: bool,
	hold_button: Option<String>,
	accum_x: f64,
	accum_y: f64,
	last_gyro_time: Instant,
	cal_samples: Vec<(f64, f64, f64)>,
	calibrating: bool,
}

impl GyroState {
	pub fn new(cfg: &GyroConfig) -> Self {
		let processor = GyroProcessor::new(cfg.calibration, cfg.sens_h, cfg.sens_v, cfg.in_game_sens);
		Self {
			processor,
			active: false,
			hold_button: None,
			accum_x: 0.0,
			accum_y: 0.0,
			last_gyro_time: Instant::now(),
			cal_samples: Vec::new(),
			calibrating: false,
		}
	}

	pub fn enable(&mut self) {
		self.hold_button = None;
		self.active = true;
		self.last_gyro_time = Instant::now();
	}

	pub fn disable(&mut self) {
		self.hold_button = None;
		self.active = false;
		self.accum_x = 0.0;
		self.accum_y = 0.0;
	}

	pub fn toggle(&mut self) {
		if self.active {
			self.disable();
		} else {
			self.enable();
		}
	}

	pub fn set_hold(&mut self, button: String) {
		self.hold_button = Some(button);
		self.active = true;
		self.last_gyro_time = Instant::now();
	}

	pub fn process_hold(&mut self, held: &[String]) {
		if let Some(ref btn) = self.hold_button {
			if !held.contains(btn) {
				self.disable();
			}
		}
	}

	pub fn active(&self) -> bool {
		self.active
	}

	pub fn start_calibration(&mut self) {
		self.cal_samples.clear();
		self.calibrating = true;
		println!("{}", style::info("gyro calibration started — collecting samples"));
	}

	pub fn stop_calibration(&mut self) {
		if !self.calibrating || self.cal_samples.is_empty() {
			println!("{}", style::warn("gyro calibration: no samples collected (no gyro events received)"));
			self.calibrating = false;
			return;
		}
		let n = self.cal_samples.len() as f64;
		let sum_x: f64 = self.cal_samples.iter().map(|s| s.0).sum();
		let sum_y: f64 = self.cal_samples.iter().map(|s| s.1).sum();
		self.processor.set_bias(sum_x / n, sum_y / n);
		self.cal_samples.clear();
		self.calibrating = false;
		println!("{}", style::info(&format!("gyro calibration complete ({} samples)", n as usize)));
	}

	pub fn reset(&mut self) {
		self.processor.reset();
		self.calibrating = false;
		self.cal_samples.clear();
	}

	pub fn process_gyro(&mut self, x: f32, y: f32, z: f32, dev: &mut OutputDevices) {
		if self.calibrating {
			self.cal_samples.push((x as f64, y as f64, z as f64));
			if self.cal_samples.len() % 100 == 0 {
				println!("{}", style::progress(&format!("calibrating... {} samples collected", self.cal_samples.len())));
			}
			return;
		}
		if !self.active { return; }
		let now = Instant::now();
		let dt = now.duration_since(self.last_gyro_time).as_secs_f64().min(0.1);
		self.last_gyro_time = now;
		let (vert, horiz) = self.processor.update(x as f64, y as f64, z as f64, dt);
		self.accum_x += horiz;
		self.accum_y += vert;
		let dx = self.accum_x as i32;
		let dy = self.accum_y as i32;
		if dx != 0 || dy != 0 {
			self.accum_x -= dx as f64;
			self.accum_y -= dy as f64;
			if let Some(ref mut m) = dev.mouse {
				let _ = m.move_mouse(dx as f64, dy as f64);
			}
		}
	}
}
