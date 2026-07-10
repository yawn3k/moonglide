use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use mlua::prelude::*;

use crate::types::{Config, GyroConfig};

pub enum CalCmd {
	Start,
	Stop,
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

	// ── load Lua scripts ──
	lua.load(format!(
		"{}\n{}\n{}",
		include_str!("lua/tables.lua"),
		include_str!("lua/bindings.lua"),
		include_str!("lua/events.lua"),
	))
	.exec()
	.map_err(|e| format!("load lua: {}", e))?;

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
