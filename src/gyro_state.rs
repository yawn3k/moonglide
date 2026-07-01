use std::time::{Duration, Instant};

use crate::bindings::{GyroConfig, GyroMode};
use crate::gyro::GyroProcessor;
use crate::output_devices::OutputDevices;
use crate::style;

const AXIS_TIMEOUT: Duration = Duration::from_millis(150);

pub struct GyroState {
	processor: GyroProcessor,
	mode: GyroMode,
	button: Option<String>,
	toggled_on: bool,
	button_held: bool,
	active: bool,
	accum_x: f64,
	accum_y: f64,
	last_gyro_time: Instant,
	last_axis_time: Instant,
	trigger_threshold: i16,
	cal_samples: Vec<(f64, f64, f64)>,
	calibrating: bool,
}

impl GyroState {
	pub fn new(cfg: &GyroConfig) -> Self {
		let processor = GyroProcessor::new(cfg.calibration, cfg.sens_h, cfg.sens_v, cfg.in_game_sens);
		let active = cfg.mode == GyroMode::AlwaysOn;
		Self {
			processor,
			mode: cfg.mode,
			button: cfg.button.clone(),
			toggled_on: false,
			button_held: false,
			active,
			accum_x: 0.0,
			accum_y: 0.0,
			last_gyro_time: Instant::now(),
			last_axis_time: Instant::now(),
			trigger_threshold: cfg.trigger_threshold as i16,
			cal_samples: Vec::new(),
			calibrating: false,
		}
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

	pub fn axis_motion(&mut self, idx: u8, val: i16) {
		let name = match idx {
			105 => "left_trigger",
			106 => "right_trigger",
			_ => return,
		};
		if self.button.as_deref() != Some(name) { return; }
		self.last_axis_time = Instant::now();
		self.button_held = val > self.trigger_threshold;
		self.update_active();
	}

	pub fn button_down(&mut self, name: &str) {
		if self.button.as_deref() != Some(name) { return; }
		match self.mode {
			GyroMode::Toggle => self.toggled_on = !self.toggled_on,
			GyroMode::HoldEnable => self.button_held = true,
			GyroMode::HoldDisable => self.button_held = true,
			_ => {}
		}
		self.update_active();
	}

	pub fn button_up(&mut self, name: &str) {
		if self.button.as_deref() != Some(name) { return; }
		match self.mode {
			GyroMode::HoldEnable => self.button_held = false,
			GyroMode::HoldDisable => self.button_held = false,
			_ => {}
		}
		self.update_active();
	}

	fn update_active(&mut self) {
		let was_active = self.active;
		self.active = match self.mode {
			GyroMode::Off => false,
			GyroMode::AlwaysOn => true,
			GyroMode::Toggle => self.toggled_on,
			GyroMode::HoldEnable => self.button_held,
			GyroMode::HoldDisable => !self.button_held,
		};
		if !self.active {
			self.accum_x = 0.0;
			self.accum_y = 0.0;
		} else if !was_active {
			self.last_gyro_time = Instant::now();
		}
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
		if matches!(self.mode, GyroMode::HoldEnable | GyroMode::HoldDisable) {
			let is_trigger = self.button.as_deref() == Some("left_trigger")
				|| self.button.as_deref() == Some("right_trigger");
			if is_trigger && self.last_axis_time.elapsed() > AXIS_TIMEOUT {
				self.button_held = false;
				self.update_active();
				if !self.active { return; }
			}
		}
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
