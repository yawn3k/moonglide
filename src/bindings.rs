#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingEvent {
	Press,
	Tap,
	Hold,
	Release,
	Turbo,
}

#[derive(Debug, Clone)]
pub struct Binding {
	pub button: String,
	pub event: BindingEvent,
	pub func_idx: usize,
	pub hold_delay_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ChordBinding {
	pub buttons: Vec<String>,
	pub func_idx: usize,
}

#[derive(Debug, Clone)]
pub struct DoublePressBinding {
	pub button: String,
	pub func_idx: usize,
	pub window_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ModeshiftBinding {
	pub modifiers: Vec<String>,
	pub button: String,
	pub func_idx: usize,
}

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
	pub bindings: Vec<Binding>,
	pub chords: Vec<ChordBinding>,
	pub double_press: Vec<DoublePressBinding>,
	pub modeshifts: Vec<ModeshiftBinding>,
	pub gyro: GyroConfig,
}

impl Default for Config {
	fn default() -> Self {
		Self {
			bindings: Vec::new(),
			chords: Vec::new(),
			double_press: Vec::new(),
			modeshifts: Vec::new(),
			gyro: GyroConfig::default(),
		}
	}
}
