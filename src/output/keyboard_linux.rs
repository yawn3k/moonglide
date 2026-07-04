use std::collections::HashMap;

use uinput::event::controller::Mouse;
use uinput::event::keyboard::Key;

fn build_key_map() -> HashMap<&'static str, Key> {
	let mut m = HashMap::new();
	m.insert("reserved", Key::Reserved);
	m.insert("esc", Key::Esc);
	m.insert("1", Key::_1);
	m.insert("2", Key::_2);
	m.insert("3", Key::_3);
	m.insert("4", Key::_4);
	m.insert("5", Key::_5);
	m.insert("6", Key::_6);
	m.insert("7", Key::_7);
	m.insert("8", Key::_8);
	m.insert("9", Key::_9);
	m.insert("0", Key::_0);
	m.insert("minus", Key::Minus);
	m.insert("equal", Key::Equal);
	m.insert("backspace", Key::BackSpace);
	m.insert("tab", Key::Tab);
	m.insert("q", Key::Q);
	m.insert("w", Key::W);
	m.insert("e", Key::E);
	m.insert("r", Key::R);
	m.insert("t", Key::T);
	m.insert("y", Key::Y);
	m.insert("u", Key::U);
	m.insert("i", Key::I);
	m.insert("o", Key::O);
	m.insert("p", Key::P);
	m.insert("leftbrace", Key::LeftBrace);
	m.insert("rightbrace", Key::RightBrace);
	m.insert("enter", Key::Enter);
	m.insert("left_control", Key::LeftControl);
	m.insert("a", Key::A);
	m.insert("s", Key::S);
	m.insert("d", Key::D);
	m.insert("f", Key::F);
	m.insert("g", Key::G);
	m.insert("h", Key::H);
	m.insert("j", Key::J);
	m.insert("k", Key::K);
	m.insert("l", Key::L);
	m.insert("semicolon", Key::SemiColon);
	m.insert("apostrophe", Key::Apostrophe);
	m.insert("grave", Key::Grave);
	m.insert("left_shift", Key::LeftShift);
	m.insert("backslash", Key::BackSlash);
	m.insert("z", Key::Z);
	m.insert("x", Key::X);
	m.insert("c", Key::C);
	m.insert("v", Key::V);
	m.insert("b", Key::B);
	m.insert("n", Key::N);
	m.insert("m", Key::M);
	m.insert("comma", Key::Comma);
	m.insert("dot", Key::Dot);
	m.insert("slash", Key::Slash);
	m.insert("right_shift", Key::RightShift);
	m.insert("left_alt", Key::LeftAlt);
	m.insert("space", Key::Space);
	m.insert("caps_lock", Key::CapsLock);
	m.insert("f1", Key::F1);
	m.insert("f2", Key::F2);
	m.insert("f3", Key::F3);
	m.insert("f4", Key::F4);
	m.insert("f5", Key::F5);
	m.insert("f6", Key::F6);
	m.insert("f7", Key::F7);
	m.insert("f8", Key::F8);
	m.insert("f9", Key::F9);
	m.insert("f10", Key::F10);
	m.insert("f11", Key::F11);
	m.insert("f12", Key::F12);
	m.insert("num_lock", Key::NumLock);
	m.insert("scroll_lock", Key::ScrollLock);
	m.insert("right_control", Key::RightControl);
	m.insert("sysrq", Key::SysRq);
	m.insert("right_alt", Key::RightAlt);
	m.insert("home", Key::Home);
	m.insert("up", Key::Up);
	m.insert("page_up", Key::PageUp);
	m.insert("left", Key::Left);
	m.insert("right", Key::Right);
	m.insert("end", Key::End);
	m.insert("down", Key::Down);
	m.insert("page_down", Key::PageDown);
	m.insert("insert", Key::Insert);
	m.insert("delete", Key::Delete);
	m.insert("left_meta", Key::LeftMeta);
	m.insert("right_meta", Key::RightMeta);
	m
}

pub struct VirtualKeyboard {
	device: uinput::Device,
	key_map: HashMap<&'static str, Key>,
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

		Ok(Self {
			device,
			key_map: build_key_map(),
		})
	}

	pub fn press(&mut self, key_name: &str) -> Result<(), String> {
		let key = self
			.key_map
			.get(key_name.to_lowercase().as_str())
			.ok_or_else(|| format!("unknown key: {}", key_name))?;
		self.device
			.press(key)
			.map_err(|e| format!("press key: {}", e))
	}

	pub fn release(&mut self, key_name: &str) -> Result<(), String> {
		let key = self
			.key_map
			.get(key_name.to_lowercase().as_str())
			.ok_or_else(|| format!("unknown key: {}", key_name))?;
		self.device
			.release(key)
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
