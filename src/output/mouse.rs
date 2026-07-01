use uinput::event::relative::{Position, Wheel};

pub struct VirtualMouse {
	device: uinput::Device,
}

impl VirtualMouse {
	pub fn new() -> Result<Self, String> {
		let mut builder = uinput::default().map_err(|e| format!("open uinput: {}", e))?;
		builder = builder
			.name("Controller Remapper Virtual Mouse")
			.map_err(|e| format!("set name: {}", e))?;

		builder = builder
			.event(uinput::event::Relative::Position(Position::X))
			.map_err(|e| format!("enable rel x: {}", e))?;
		builder = builder
			.event(uinput::event::Relative::Position(Position::Y))
			.map_err(|e| format!("enable rel y: {}", e))?;
		builder = builder
			.event(uinput::event::Relative::Wheel(Wheel::Vertical))
			.map_err(|e| format!("enable wheel: {}", e))?;

		let device = builder.create().map_err(|e| format!("create device: {}", e))?;

		Ok(Self { device })
	}

	pub fn move_mouse(&mut self, dx: f64, dy: f64) -> Result<(), String> {
		self.device
			.position(&Position::X, dx as i32)
			.map_err(|e| format!("move x: {}", e))?;
		self.device
			.position(&Position::Y, dy as i32)
			.map_err(|e| format!("move y: {}", e))
	}

	pub fn synchronize(&mut self) -> Result<(), String> {
		self.device
			.synchronize()
			.map_err(|e| format!("sync: {}", e))
	}
}
