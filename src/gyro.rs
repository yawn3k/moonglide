const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;

pub struct GyroProcessor {
	pub calibration: f64,
	pub sens_h: f64,
	pub sens_v: f64,
	pub in_game_sens: f64,
	bias_x: f64,
	bias_y: f64,
}

impl GyroProcessor {
	pub fn new(calibration: f64, sens_h: f64, sens_v: f64, in_game_sens: f64) -> Self {
		Self { calibration, sens_h, sens_v, in_game_sens, bias_x: 0.0, bias_y: 0.0 }
	}

	pub fn update(&mut self, gx: f64, gy: f64, _gz: f64, dt: f64) -> (f64, f64) {
		let rx = gx - self.bias_x;
		let ry = gy - self.bias_y;

		// RWS: angle (deg) × calibration × sens / in_game_sens
		let pitch_deg = rx * RAD_TO_DEG * dt;  // X-axis → mouse Y (vertical)
		let yaw_deg   = ry * RAD_TO_DEG * dt;   // Y-axis → mouse X (horizontal)

		let dx = -yaw_deg * self.calibration * self.sens_h / self.in_game_sens;
		let dy = -pitch_deg * self.calibration * self.sens_v / self.in_game_sens;

		(dy, dx)  // (vert, horiz)
	}

	pub fn set_bias(&mut self, bx: f64, by: f64) {
		println!("\x1b[1m\x1b[32mgyro bias set: x={:.4} y={:.4}\x1b[0m", bx, by);
		self.bias_x = bx;
		self.bias_y = by;
	}

	pub fn reset(&mut self) {
		self.bias_x = 0.0;
		self.bias_y = 0.0;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_gyro_returns_values() {
		let mut g = GyroProcessor::new(45.454, 1.0, 1.0, 1.0);
		let (v, h) = g.update(1.0, 1.0, 0.0, 1.0 / 60.0);
		assert!(v != 0.0);
		assert!(h != 0.0);
	}
}
