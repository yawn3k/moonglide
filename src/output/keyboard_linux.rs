use uinput::event::controller::Mouse;
use uinput::event::keyboard::Key;

fn key_from_name(name: &str) -> Option<Key> {
	match name.to_lowercase().as_str() {
		"esc" => Some(Key::Esc),
		"1" => Some(Key::_1),
		"2" => Some(Key::_2),
		"3" => Some(Key::_3),
		"4" => Some(Key::_4),
		"5" => Some(Key::_5),
		"6" => Some(Key::_6),
		"7" => Some(Key::_7),
		"8" => Some(Key::_8),
		"9" => Some(Key::_9),
		"0" => Some(Key::_0),
		"minus" => Some(Key::Minus),
		"equal" => Some(Key::Equal),
		"backspace" => Some(Key::BackSpace),
		"tab" => Some(Key::Tab),
		"q" => Some(Key::Q),
		"w" => Some(Key::W),
		"e" => Some(Key::E),
		"r" => Some(Key::R),
		"t" => Some(Key::T),
		"y" => Some(Key::Y),
		"u" => Some(Key::U),
		"i" => Some(Key::I),
		"o" => Some(Key::O),
		"p" => Some(Key::P),
		"leftbrace" => Some(Key::LeftBrace),
		"rightbrace" => Some(Key::RightBrace),
		"enter" => Some(Key::Enter),
		"left_control" => Some(Key::LeftControl),
		"a" => Some(Key::A),
		"s" => Some(Key::S),
		"d" => Some(Key::D),
		"f" => Some(Key::F),
		"g" => Some(Key::G),
		"h" => Some(Key::H),
		"j" => Some(Key::J),
		"k" => Some(Key::K),
		"l" => Some(Key::L),
		"semicolon" => Some(Key::SemiColon),
		"apostrophe" => Some(Key::Apostrophe),
		"grave" => Some(Key::Grave),
		"left_shift" => Some(Key::LeftShift),
		"backslash" => Some(Key::BackSlash),
		"z" => Some(Key::Z),
		"x" => Some(Key::X),
		"c" => Some(Key::C),
		"v" => Some(Key::V),
		"b" => Some(Key::B),
		"n" => Some(Key::N),
		"m" => Some(Key::M),
		"comma" => Some(Key::Comma),
		"dot" => Some(Key::Dot),
		"slash" => Some(Key::Slash),
		"right_shift" => Some(Key::RightShift),
		"left_alt" => Some(Key::LeftAlt),
		"space" => Some(Key::Space),
		"caps_lock" => Some(Key::CapsLock),
		"f1" => Some(Key::F1),
		"f2" => Some(Key::F2),
		"f3" => Some(Key::F3),
		"f4" => Some(Key::F4),
		"f5" => Some(Key::F5),
		"f6" => Some(Key::F6),
		"f7" => Some(Key::F7),
		"f8" => Some(Key::F8),
		"f9" => Some(Key::F9),
		"f10" => Some(Key::F10),
		"f11" => Some(Key::F11),
		"f12" => Some(Key::F12),
		"num_lock" => Some(Key::NumLock),
		"scroll_lock" => Some(Key::ScrollLock),
		"right_control" => Some(Key::RightControl),
		"sysrq" => Some(Key::SysRq),
		"right_alt" => Some(Key::RightAlt),
		"home" => Some(Key::Home),
		"up" => Some(Key::Up),
		"page_up" => Some(Key::PageUp),
		"left" => Some(Key::Left),
		"right" => Some(Key::Right),
		"end" => Some(Key::End),
		"down" => Some(Key::Down),
		"page_down" => Some(Key::PageDown),
		"insert" => Some(Key::Insert),
		"delete" => Some(Key::Delete),
		"left_meta" => Some(Key::LeftMeta),
		"right_meta" => Some(Key::RightMeta),
		_ => None,
	}
}

pub struct VirtualKeyboard {
	device: uinput::Device,
}

impl VirtualKeyboard {
	pub fn new() -> Result<Self, String> {
		let mut builder = uinput::default().map_err(|e| format!("open uinput: {}", e))?;
		builder = builder
			.name("Controller Remapper Virtual Keyboard")
			.map_err(|e| format!("set name: {}", e))?;

		builder = builder
			.event(uinput::event::Keyboard::All)
			.map_err(|e| format!("enable keyboard: {}", e))?;

		builder = builder
			.event(uinput::event::Controller::Mouse(Mouse::Left))
			.map_err(|e| format!("enable mouse left: {}", e))?;
		builder = builder
			.event(uinput::event::Controller::Mouse(Mouse::Right))
			.map_err(|e| format!("enable mouse right: {}", e))?;
		builder = builder
			.event(uinput::event::Controller::Mouse(Mouse::Middle))
			.map_err(|e| format!("enable mouse middle: {}", e))?;

		let device = builder.create().map_err(|e| format!("create device: {}", e))?;

		Ok(Self { device })
	}

	pub fn press(&mut self, key_name: &str) -> Result<(), String> {
		let key = key_from_name(key_name).ok_or_else(|| format!("unknown key: {}", key_name))?;
		self.device
			.press(&key)
			.map_err(|e| format!("press key: {}", e))
	}

	pub fn release(&mut self, key_name: &str) -> Result<(), String> {
		let key = key_from_name(key_name).ok_or_else(|| format!("unknown key: {}", key_name))?;
		self.device
			.release(&key)
			.map_err(|e| format!("release key: {}", e))
	}

	pub fn press_mouse(&mut self, btn: u8) -> Result<(), String> {
		let ev = uinput::event::Controller::Mouse(mouse_btn(btn)?);
		self.device
			.press(&ev)
			.map_err(|e| format!("press mouse: {}", e))
	}

	pub fn release_mouse(&mut self, btn: u8) -> Result<(), String> {
		let ev = uinput::event::Controller::Mouse(mouse_btn(btn)?);
		self.device
			.release(&ev)
			.map_err(|e| format!("release mouse: {}", e))
	}

	pub fn synchronize(&mut self) -> Result<(), String> {
		self.device
			.synchronize()
			.map_err(|e| format!("sync: {}", e))
	}
}

fn mouse_btn(btn: u8) -> Result<Mouse, String> {
	match btn {
		1 => Ok(Mouse::Left),
		2 => Ok(Mouse::Right),
		3 => Ok(Mouse::Middle),
		_ => Err(format!("unknown mouse button {}", btn)),
	}
}
