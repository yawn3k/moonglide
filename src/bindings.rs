#[derive(Debug, Clone)]
pub struct GyroConfig {
	pub calibration: f64,
	pub sens_h: f64,
	pub sens_v: f64,
	pub in_game_sens: f64,
}

impl Default for GyroConfig {
	fn default() -> Self {
		Self {
			calibration: 45.454,
			sens_h: 1.0,
			sens_v: 1.0,
			in_game_sens: 1.0,
		}
	}
}

pub enum GyroCmd {
	Enable,
	Disable,
	Toggle,
	Hold(String),
}

#[derive(Debug, Clone)]
pub struct Config {
	pub gyro: GyroConfig,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			gyro: GyroConfig::default(),
		}
	}
}
