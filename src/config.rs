use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use mlua::prelude::*;

use crate::bindings::{Config, GyroConfig};

pub enum CalCmd {
	Start,
	Stop,
}

fn make_ref(lua: &Lua, src: &str, field: &str, val: &str) -> mlua::Result<mlua::Table> {
	let t = lua.create_table()?;
	t.set("__kind", "ref")?;
	t.set("src", src)?;
	t.set("field", field)?;
	t.set("val", val)?;
	Ok(t)
}

fn parse_sens_pair(val: &mlua::Value) -> (f64, f64) {
	match val {
		mlua::Value::Number(n) => (*n, *n),
		mlua::Value::Table(t) => {
			let h: f64 = t.get(1).unwrap_or(1.0);
			let v: f64 = t.get(2).unwrap_or(h);
			(h, v)
		}
		mlua::Value::String(s) => {
			let s = s.to_string_lossy();
			let parts: Vec<f64> = s.split_whitespace()
				.filter_map(|p| p.parse::<f64>().ok())
				.collect();
			match parts.len() {
				0 => (1.0, 1.0),
				1 => (parts[0], parts[0]),
				_ => (parts[0], parts[1]),
			}
		}
		_ => (1.0, 1.0),
	}
}

pub fn setup_dsl(
	lua: &Lua,
	gyro_shared: &std::sync::Arc<std::sync::Mutex<GyroConfig>>,
	cal_tx: &mpsc::Sender<CalCmd>,
) -> Result<(), String> {
	let globals = lua.globals();

	// ── con/key/mouse tables ──
	let ok = |r: mlua::Result<()>| r.map_err(|e| e.to_string());
	let con = lua.create_table().map_err(|e| e.to_string())?;
	for &name in &[
		"a", "b", "x", "y",
		"dpad_up", "dpad_down", "dpad_left", "dpad_right",
		"left_shoulder", "right_shoulder",
		"left_stick", "right_stick",
		"start", "back", "guide",
		"left_trigger", "right_trigger",
		"touchpad_click", "touchpad_touch",
		"misc_1",
		"paddle_1", "paddle_2", "paddle_3", "paddle_4",
		"left_stick_up", "left_stick_down", "left_stick_left", "left_stick_right",
		"right_stick_up", "right_stick_down", "right_stick_left", "right_stick_right",
		"left_ring_inner", "left_ring_outer",
		"right_ring_inner", "right_ring_outer",
	] {
		ok(con.set(name, make_ref(lua, "con", name, name).map_err(|e| e.to_string())?))?;
	}
	ok(globals.set("con", con))?;

	let key = lua.create_table().map_err(|e| e.to_string())?;
	for &name in &[
		"esc", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0",
		"minus", "equal", "backspace", "tab",
		"q", "w", "e", "r", "t", "y", "u", "i", "o", "p",
		"leftbrace", "rightbrace", "enter",
		"left_control", "a", "s", "d", "f", "g", "h", "j", "k", "l",
		"semicolon", "apostrophe", "grave",
		"left_shift", "backslash",
		"z", "x", "c", "v", "b", "n", "m",
		"comma", "dot", "slash", "right_shift",
		"left_alt", "space", "caps_lock",
		"f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12",
		"num_lock", "scroll_lock", "right_control",
		"sysrq", "right_alt",
		"home", "up", "page_up", "left", "right", "end", "down", "page_down",
		"insert", "delete",
		"left_meta", "right_meta",
		"zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
	] {
		let val = match name {
			"zero" => "0", "one" => "1", "two" => "2", "three" => "3", "four" => "4",
			"five" => "5", "six" => "6", "seven" => "7", "eight" => "8", "nine" => "9",
			_ => name,
		};
		let r = make_ref(lua, "key", name, val).map_err(|e| e.to_string())?;
		ok(key.set(name, r))?;
	}
	ok(globals.set("key", key))?;

	let mouse = lua.create_table().map_err(|e| e.to_string())?;
	ok(mouse.set("left", make_ref(lua, "mouse", "left", "left_mouse").map_err(|e| e.to_string())?))?;
	ok(mouse.set("right", make_ref(lua, "mouse", "right", "right_mouse").map_err(|e| e.to_string())?))?;
	ok(mouse.set("middle", make_ref(lua, "mouse", "middle", "middle_mouse").map_err(|e| e.to_string())?))?;
	ok(globals.set("mouse", mouse))?;

	// ── gyro function ──
	let gs = gyro_shared.clone();
	let gyro_fn = lua.create_function(move |_, tbl: mlua::Table| {
		let sens_val: mlua::Value = tbl.get("sensitivity").unwrap_or(mlua::Value::Number(1.0));
		let gyro_val: mlua::Value = tbl.get("gyro_sens").unwrap_or(sens_val);
		let (sens_h, sens_v) = parse_sens_pair(&gyro_val);
		let calibration: f64 = tbl.get("calibration").unwrap_or(45.454);
		let in_game_sens: f64 = tbl.get("in_game_sens").unwrap_or(1.0);
		let mut g = gs.lock().unwrap();
		g.calibration = calibration;
		g.sens_h = sens_h;
		g.sens_v = sens_v;
		g.in_game_sens = in_game_sens;
		Ok(())
	}).map_err(|e| e.to_string())?;
	globals.set("gyro", gyro_fn).map_err(|e| e.to_string())?;

	// ── calibration helpers ──
	let ct = cal_tx.clone();
	let cal_start = lua.create_function(move |_, ()| {
		let _ = ct.send(CalCmd::Start);
		Ok(())
	}).map_err(|e| e.to_string())?;
	globals.set("gyro_calibrate_start", cal_start).map_err(|e| e.to_string())?;

	let ct = cal_tx.clone();
	let cal_stop = lua.create_function(move |_, ()| {
		let _ = ct.send(CalCmd::Stop);
		Ok(())
	}).map_err(|e| e.to_string())?;
	globals.set("gyro_calibrate_stop", cal_stop).map_err(|e| e.to_string())?;

	// ── wait helper ──
	lua.load("function wait(s) local saved = _current_btn; coroutine.yield(s); _current_btn = saved end")
		.exec()
		.map_err(|e| format!("register wait: {}", e))?;

	// ── load bindings library ──
	lua.load(include_str!("bindings.lua"))
		.exec()
		.map_err(|e| format!("load bindings.lua: {}", e))?;

	Ok(())
}

fn add_config_dir_to_package_path(lua: &Lua, config_dir: &str) {
	let _ = lua.load(&format!(
		"package.path = '{}/?.lua;' .. package.path",
		config_dir.replace('\'', "'\\''")
	)).exec();
}

pub fn init_bare(
	lua: &Lua,
	_config: &Arc<Mutex<Config>>,
	_gyro_shared: &Arc<Mutex<GyroConfig>>,
) {
	add_config_dir_to_package_path(lua, ".");
}

pub fn load(
	path: &str,
	lua: &Lua,
	_config: &Arc<Mutex<Config>>,
	gyro_shared: &Arc<Mutex<GyroConfig>>,
) -> Result<Config, String> {
	let abs = std::path::Path::new(path)
		.canonicalize()
		.map_err(|e| format!("canonicalize {}: {}", path, e))?;
	let dir = abs.parent().unwrap_or(std::path::Path::new("."))
		.to_string_lossy()
		.to_string();
	add_config_dir_to_package_path(lua, &dir);

	let src = std::fs::read_to_string(path).map_err(|e| format!("read config: {}", e))?;
	lua.load(&src).exec().map_err(|e| format!("lua exec: {}", e))?;

	if let Ok(thresh) = lua.globals().get::<u16>("trigger_threshold") {
		crate::TRIGGER_THRESHOLD.store(thresh, std::sync::atomic::Ordering::Relaxed);
	}

	let gyro = gyro_shared.lock().unwrap().clone();
	let mut final_cfg = Config::default();
	final_cfg.gyro = gyro;

	Ok(final_cfg)
}
