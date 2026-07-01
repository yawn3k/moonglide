use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use mlua::prelude::*;

use crate::bindings::{Binding, BindingEvent, ChordBinding, Config, DoublePressBinding, GyroConfig, GyroMode, ModeshiftBinding};

pub enum CalCmd {
	Start,
	Stop,
}

fn store_action(entry: &mlua::Table, action: mlua::Value) -> mlua::Result<()> {
	match action {
		mlua::Value::Function(f) => entry.set("func", f),
		mlua::Value::String(s) => {
			let key = s.to_str().map_err(|e| mlua::Error::external(e.to_string()))?;
			entry.set("action_string", key.to_string())
		}
		mlua::Value::Table(t) => {
			let key: String = t.get("key").map_err(|_| mlua::Error::external("table action missing 'key'"))?;
			entry.set("action_string", key)
		}
		_ => Err(mlua::Error::external("action must be a function, string, or helper table")),
	}
}

fn extract_func(
	lua: &Lua,
	entry: &mlua::Table,
	callbacks: &mut Vec<Arc<mlua::RegistryKey>>,
) -> Result<usize, String> {
	if let Ok(func) = entry.get::<mlua::Function>("func") {
		let key = lua.create_registry_value(func).map_err(|e| e.to_string())?;
		let idx = callbacks.len();
		callbacks.push(Arc::new(key));
		Ok(idx)
	} else if let Ok(action_str) = entry.get::<String>("action_string") {
		let event_str: String = entry.get("event").map_err(|e| e.to_string())?;
		let helper = match event_str.as_str() {
			"release" => "instant",
			"tap" => "instant",
			_ => "press",
		};
		let script = format!("return function() {}({:?}) end", helper, action_str);
		let wrapper: mlua::Function = lua.load(&script).eval().map_err(|e| e.to_string())?;
		let key = lua.create_registry_value(wrapper).map_err(|e| e.to_string())?;
		let idx = callbacks.len();
		callbacks.push(Arc::new(key));
		Ok(idx)
	} else {
		Err("binding entry has neither func nor action_string".into())
	}
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

fn setup_dsl(
	lua: &Lua,
	gyro_shared: &std::sync::Arc<std::sync::Mutex<GyroConfig>>,
	cal_tx: &mpsc::Sender<CalCmd>,
) -> Result<(), String> {
	let globals = lua.globals();

	let bindings = lua.create_table().map_err(|e| e.to_string())?;
	let chords = lua.create_table().map_err(|e| e.to_string())?;
	let double_press = lua.create_table().map_err(|e| e.to_string())?;
	let modeshifts = lua.create_table().map_err(|e| e.to_string())?;

	globals.set("_bindings", bindings.clone()).map_err(|e| e.to_string())?;
	globals.set("_chords", chords.clone()).map_err(|e| e.to_string())?;
	globals.set("_double_press", double_press.clone()).map_err(|e| e.to_string())?;
	globals.set("_modeshifts", modeshifts.clone()).map_err(|e| e.to_string())?;

	let bind_table = lua.create_table().map_err(|e| e.to_string())?;

	let make_fn = |b: mlua::Table, lua2: Lua, event: &'static str, has_opts: bool|
		-> mlua::Result<mlua::Function>
	{
		if has_opts {
			lua.clone().create_function(move |_, (btn, action, opts): (String, mlua::Value, Option<mlua::Table>)| {
				let entry = lua2.create_table()?;
				entry.set("button", btn)?;
				entry.set("event", event)?;
				store_action(&entry, action)?;
				if let Some(t) = opts {
					if let Ok(d) = t.get::<Option<u64>>("delay") {
						if let Some(d) = d { entry.set("hold_delay_ms", d)?; }
					}
		}
		b.push(entry)
	})
		} else {
			lua.clone().create_function(move |_, (btn, action): (String, mlua::Value)| {
				let entry = lua2.create_table()?;
				entry.set("button", btn)?;
				entry.set("event", event)?;
				store_action(&entry, action)?;
				b.push(entry)
			})
		}
	};

	let b = bindings.clone();
	let lua2 = lua.clone();
	let press_fn = make_fn(b, lua2, "press", false).map_err(|e| e.to_string())?;
	bind_table.set("press", press_fn).map_err(|e| e.to_string())?;

	let b = bindings.clone();
	let lua2 = lua.clone();
	let tap_fn = make_fn(b, lua2, "tap", false).map_err(|e| e.to_string())?;
	bind_table.set("tap", tap_fn).map_err(|e| e.to_string())?;

	let b = bindings.clone();
	let lua2 = lua.clone();
	let hold_fn = make_fn(b, lua2, "hold", true).map_err(|e| e.to_string())?;
	bind_table.set("hold", hold_fn).map_err(|e| e.to_string())?;

	let b = bindings.clone();
	let lua2 = lua.clone();
	let release_fn = make_fn(b, lua2, "release", false).map_err(|e| e.to_string())?;
	bind_table.set("release", release_fn).map_err(|e| e.to_string())?;

	let b = bindings.clone();
	let lua2 = lua.clone();
	let turbo_fn = make_fn(b, lua2, "turbo", false).map_err(|e| e.to_string())?;
	bind_table.set("turbo", turbo_fn).map_err(|e| e.to_string())?;

	let c = chords.clone();
	let lua2 = lua.clone();
	let chord_fn = lua.clone().create_function(move |_, (btns, action, _opts): (Vec<String>, mlua::Value, Option<mlua::Table>)| {
		let entry = lua2.create_table()?;
		entry.set("event", "press")?;
		entry.set("buttons", btns)?;
		store_action(&entry, action)?;
		c.push(entry)
	}).map_err(|e| e.to_string())?;
	bind_table.set("chord", chord_fn).map_err(|e| e.to_string())?;

	let dp = double_press.clone();
	let lua2 = lua.clone();
	let double_press_fn = lua.create_function(move |_, (btn, action, opts): (String, mlua::Value, Option<mlua::Table>)| {
		let entry = lua2.create_table()?;
		entry.set("button", btn)?;
		entry.set("event", "tap")?;
		store_action(&entry, action)?;
		let global_window: u64 = lua2.globals().get("double_press_window").unwrap_or(200);
		let window_ms: u64 = opts.and_then(|t| t.get("window").ok()).unwrap_or(global_window);
		entry.set("window_ms", window_ms)?;
		dp.push(entry)
	}).map_err(|e| e.to_string())?;
	bind_table.set("double_press", double_press_fn).map_err(|e| e.to_string())?;

	let ms = modeshifts.clone();
	let lua2 = lua.clone();
	let modeshift_fn = lua.create_function(move |_, (modifiers, btn, action): (Vec<String>, String, mlua::Value)| {
		let entry = lua2.create_table()?;
		entry.set("modifiers", modifiers)?;
		entry.set("button", btn)?;
		entry.set("event", "press")?;
		store_action(&entry, action)?;
		ms.push(entry)
	}).map_err(|e| e.to_string())?;
	bind_table.set("modeshift", modeshift_fn).map_err(|e| e.to_string())?;

	globals.set("bind", bind_table).map_err(|e| e.to_string())?;

	let gs = gyro_shared.clone();
	let gyro_fn = lua.create_function(move |_, tbl: mlua::Table| {
		let mode_str: String = tbl.get("mode").unwrap_or_else(|_| "always_on".into());
		let mode = match mode_str.as_str() {
			"off" => GyroMode::Off,
			"toggle" => GyroMode::Toggle,
			"hold_enable" => GyroMode::HoldEnable,
			"hold_disable" => GyroMode::HoldDisable,
			_ => GyroMode::AlwaysOn,
		};
		let btn: Option<String> = tbl.get("button").ok();
		let sens_val: mlua::Value = tbl.get("sensitivity").unwrap_or(mlua::Value::Number(1.0));
		let gyro_val: mlua::Value = tbl.get("gyro_sens").unwrap_or(sens_val);
		let (sens_h, sens_v) = parse_sens_pair(&gyro_val);
		let calibration: f64 = tbl.get("calibration").unwrap_or(45.454);
		let in_game_sens: f64 = tbl.get("in_game_sens").unwrap_or(1.0);
		let mut g = gs.lock().unwrap();
		g.mode = mode;
		g.button = btn;
		g.calibration = calibration;
		g.sens_h = sens_h;
		g.sens_v = sens_v;
		g.in_game_sens = in_game_sens;
		Ok(())
	}).map_err(|e| e.to_string())?;
	globals.set("gyro", gyro_fn).map_err(|e| e.to_string())?;

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

	lua.load("function wait(s) coroutine.yield(s) end")
		.exec()
		.map_err(|e| format!("register wait: {}", e))?;

	Ok(())
}


pub fn init_bare(
	config: &Arc<Mutex<Config>>,
	callbacks: &Arc<Mutex<Vec<Arc<mlua::RegistryKey>>>>,
	gyro_shared: &Arc<Mutex<GyroConfig>>,
) -> (Lua, mpsc::Receiver<CalCmd>) {
	let lua = Lua::new();
	let (cal_tx, cal_rx) = mpsc::channel();
	gyro_shared.lock().unwrap().mode = GyroMode::Off;

	if let Err(e) = setup_dsl(&lua, gyro_shared, &cal_tx) {
		eprintln!("warning: setup_dsl failed: {}", e);
	}

	let lua2 = lua.clone();
	let cb = callbacks.clone();
	let cfg = config.clone();
	let gs = gyro_shared.clone();
	let include_fn = lua.create_function(move |_, rel: String| -> mlua::Result<()> {
		let full = if rel.starts_with('/') {
			rel
		} else if rel.starts_with("~/") || rel == "~" {
			let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
			if rel == "~" { home } else { format!("{}/{}", home, &rel[2..]) }
		} else {
			rel
		};
		let code = std::fs::read_to_string(&full)
			.map_err(|e| mlua::Error::external(format!("include {}: {}", full, e)))?;
		lua2.load(&code).exec()
			.map_err(|e| mlua::Error::external(format!("include {}: {}", full, e)))?;
		process_pending(&lua2, &mut *cb.lock().unwrap(), &cfg, &gs)
			.map_err(|e| mlua::Error::external(e))
	}).expect("create include_fn");
	lua.globals().set("include", include_fn).expect("set include");

	(lua, cal_rx)
}

pub fn load(
	path: &str,
	config: &Arc<Mutex<Config>>,
	callbacks: &Arc<Mutex<Vec<Arc<mlua::RegistryKey>>>>,
	gyro_shared: &Arc<Mutex<GyroConfig>>,
) -> Result<(Config, Lua, mpsc::Receiver<CalCmd>), String> {
	let lua = Lua::new();
	let (cal_tx, cal_rx) = mpsc::channel();
	gyro_shared.lock().unwrap().mode = GyroMode::Off;

	setup_dsl(&lua, gyro_shared, &cal_tx)?;

	let lua2 = lua.clone();
	let cb = callbacks.clone();
	let cfg = config.clone();
	let gs = gyro_shared.clone();
	let include_fn = lua.create_function(move |_, rel: String| -> mlua::Result<()> {
		let full = if rel.starts_with('/') {
			rel
		} else if rel.starts_with("~/") || rel == "~" {
			let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
			if rel == "~" { home } else { format!("{}/{}", home, &rel[2..]) }
		} else {
			rel
		};
		let code = std::fs::read_to_string(&full)
			.map_err(|e| mlua::Error::external(format!("include {}: {}", full, e)))?;
		lua2.load(&code).exec()
			.map_err(|e| mlua::Error::external(format!("include {}: {}", full, e)))?;
		process_pending(&lua2, &mut *cb.lock().unwrap(), &cfg, &gs)
			.map_err(|e| mlua::Error::external(e))
	}).map_err(|e| e.to_string())?;
	lua.globals().set("include", include_fn).map_err(|e| e.to_string())?;

	let src = std::fs::read_to_string(path).map_err(|e| format!("read config: {}", e))?;
	lua.load(&src).exec().map_err(|e| format!("lua exec: {}", e))?;

	if let Ok(thresh) = lua.globals().get::<u16>("trigger_threshold") {
		let mut g = gyro_shared.lock().unwrap();
		g.trigger_threshold = thresh;
	}

	process_pending(&lua, &mut *callbacks.lock().unwrap(), config, gyro_shared)?;

	let gyro = gyro_shared.lock().unwrap().clone();
	let mut final_cfg = config.lock().unwrap().clone();
	final_cfg.gyro = gyro;

	Ok((final_cfg, lua, cal_rx))
}

fn process_pending(
	lua: &Lua,
	callbacks: &mut Vec<Arc<mlua::RegistryKey>>,
	config: &std::sync::Arc<std::sync::Mutex<Config>>,
	gyro_shared: &std::sync::Arc<std::sync::Mutex<GyroConfig>>,
) -> Result<(), String> {
	let bindings_table: mlua::Table = lua.globals().get("_bindings")
		.map_err(|e| format!("get _bindings: {}", e))?;
	let chords_table: mlua::Table = lua.globals().get("_chords")
		.map_err(|e| format!("get _chords: {}", e))?;
	let dp_table: mlua::Table = lua.globals().get("_double_press")
		.map_err(|e| format!("get _double_press: {}", e))?;
	let ms_table: mlua::Table = lua.globals().get("_modeshifts")
		.map_err(|e| format!("get _modeshifts: {}", e))?;

	let mut new_bindings = Vec::new();
	let mut new_chords = Vec::new();
	let mut new_double_press = Vec::new();
	let mut new_modeshifts = Vec::new();

	for pair in bindings_table.pairs::<mlua::Value, mlua::Value>() {
		let (_, val) = pair.map_err(|e| e.to_string())?;
		if let mlua::Value::Table(t) = val {
			let btn: String = t.get("button").map_err(|e| e.to_string())?;
			let event_str: String = t.get("event").map_err(|e| e.to_string())?;
			let event = match event_str.as_str() {
				"press" => BindingEvent::Press,
				"tap" => BindingEvent::Tap,
				"hold" => BindingEvent::Hold,
				"release" => BindingEvent::Release,
				"turbo" => BindingEvent::Turbo,
				_ => return Err(format!("unknown event type: {}", event_str)),
			};
			let func_idx = extract_func(lua, &t, callbacks)?;
			let global_hold: u64 = lua.globals().get("hold_press_time").unwrap_or(400);
			let hold_delay_ms: u64 = t.get("hold_delay_ms").unwrap_or(global_hold);
			new_bindings.push(Binding { button: btn, event, func_idx, hold_delay_ms });
		}
	}

	for pair in chords_table.pairs::<mlua::Value, mlua::Value>() {
		let (_, val) = pair.map_err(|e| e.to_string())?;
		if let mlua::Value::Table(t) = val {
			let buttons: Vec<String> = t.get("buttons").map_err(|e| e.to_string())?;
			let func_idx = extract_func(lua, &t, callbacks)?;
			new_chords.push(ChordBinding { buttons, func_idx });
		}
	}

	for pair in dp_table.pairs::<mlua::Value, mlua::Value>() {
		let (_, val) = pair.map_err(|e| e.to_string())?;
		if let mlua::Value::Table(t) = val {
			let button: String = t.get("button").map_err(|e| e.to_string())?;
			let window_ms: u64 = t.get("window_ms").map_err(|e| e.to_string())?;
			let func_idx = extract_func(lua, &t, callbacks)?;
			new_double_press.push(DoublePressBinding { button, func_idx, window_ms });
		}
	}

	for pair in ms_table.pairs::<mlua::Value, mlua::Value>() {
		let (_, val) = pair.map_err(|e| e.to_string())?;
		if let mlua::Value::Table(t) = val {
			let modifiers: Vec<String> = t.get("modifiers").map_err(|e| e.to_string())?;
			let button: String = t.get("button").map_err(|e| e.to_string())?;
			let func_idx = extract_func(lua, &t, callbacks)?;
			new_modeshifts.push(ModeshiftBinding { modifiers, button, func_idx });
		}
	}

	// Clear the tables so subsequent includes start fresh
	let clear = "_bindings = {}; _chords = {}; _double_press = {}; _modeshifts = {}";
	lua.load(clear).exec().map_err(|e| format!("clear tables: {}", e))?;

	let mut c = config.lock().unwrap();
	c.bindings.extend(new_bindings);
	c.chords.extend(new_chords);
	c.double_press.extend(new_double_press);
	c.modeshifts.extend(new_modeshifts);
	let gyro = gyro_shared.lock().unwrap().clone();
	c.gyro = gyro;

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_config_parser_works() {
		let config = Arc::new(Mutex::new(Config::default()));
		let callbacks = Arc::new(Mutex::new(Vec::new()));
		let gyro_shared = Arc::new(Mutex::new(GyroConfig::default()));

		let lua = Lua::new();
		let (cal_tx, _cal_rx) = mpsc::channel();
		setup_dsl(&lua, &gyro_shared, &cal_tx).unwrap();

		lua.load(r#"
			bind.press("a", "space")
			bind.press("b", "enter")
		"#).exec().unwrap();

		process_pending(&lua, &mut *callbacks.lock().unwrap(), &config, &gyro_shared).unwrap();

		let cfg = config.lock().unwrap();
		assert!(!cfg.bindings.is_empty());
		assert_eq!(cfg.bindings[0].button, "a");
	}
}
