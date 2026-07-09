mod bindings;
mod config;
mod controller;
mod gyro;
mod gyro_state;
mod lua_coroutines;
mod mapping;
mod output;
mod output_devices;
mod stick;
mod style;

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mlua::{Lua, RegistryKey};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use bindings::{BindingEvent, Config, GyroCmd, GyroConfig};
use config::CalCmd;
use controller::{idx_to_button_name, ControllerEvent};
use gyro_state::GyroState;
use lua_coroutines::{execute_lua_function, poll_pending_threads, PendingThread};
use mapping::Mapper;
use output::keyboard::VirtualKeyboard;
use output::mouse::VirtualMouse;
use output_devices::{OutputDevices, TriggerTracker};
use stick::{process_stick_buttons, MAX_AXIS};

pub(crate) static LOG_LEVEL: AtomicU8 = AtomicU8::new(0);
pub(crate) static TRIGGER_THRESHOLD: AtomicU16 = AtomicU16::new(3000);
static INSTANT_PRESS_TIME: AtomicU64 = AtomicU64::new(40);

pub(crate) fn log_msg(level: u8, msg: &str) {
	if LOG_LEVEL.load(Ordering::Relaxed) >= level {
		println!("{}{}", style::dim(&format!("[{}]", level)), msg);
	}
}

struct OutputState {
	dev: OutputDevices,
	mapper: Arc<Mutex<Mapper>>,
	triggers: TriggerTracker,
	gyro: GyroState,
	axis_state: HashMap<u32, [i16; 6]>,
	prev_stick_dirs: HashMap<u32, HashSet<String>>,
	defer_map: HashMap<String, (Instant, usize)>,
	pending: Vec<PendingThread>,
	cfg: Config,
	gyro_rx: std::sync::mpsc::Receiver<GyroCmd>,
}

fn handle_btn_down(
	name: &str,
	state: &mut OutputState,
	callbacks: &[Arc<RegistryKey>],
	lua: &Lua,
) {
	log_msg(1, &format!("{} down", name));
	let now = Instant::now();

	let held = state.mapper.lock().unwrap().held_buttons();
	let all_held: Vec<&str> = held.iter().map(|s| s.as_str()).collect();
	let chord_fired = fire_chord(&all_held, name, &state.cfg, callbacks, lua, &state.mapper, &mut state.pending);

	if !chord_fired {
		let mut fired = false;

		{
			let mut m = state.mapper.lock().unwrap();
			let dp = state.cfg.double_press.iter().find(|d| d.button == name).cloned();
			let is_dp = dp.as_ref().is_some_and(|d| m.is_double_press(name, now, d.window_ms));
			m.last_press_times.insert(name.to_string(), now);
			if let Some(ref dp_binding) = dp {
				if is_dp {
					state.defer_map.remove(name);
					for k in m.button_up(name) {
						state.dev.apply(&k, false);
					}
					m.button_down(name, now);
					m.mark_consumed(name);
					drop(m);
					let _ = lua.globals().set("_current_btn", name);
					execute_lua_function(dp_binding.func_idx, callbacks, lua, &mut state.pending);
					fired = true;
				}
			}
		}

		if !fired {
			let held = state.mapper.lock().unwrap().held_buttons();
			if let Some(ms) = state.cfg.modeshifts.iter().find(|m| {
				m.button == name && m.modifiers.iter().all(|modifier| held.contains(modifier))
			}) {
				let _ = lua.globals().set("_current_btn", name);
				state.mapper.lock().unwrap().button_down(name, now);
				state.mapper.lock().unwrap().mark_consumed(name);
				execute_lua_function(ms.func_idx, callbacks, lua, &mut state.pending);
				fired = true;
			}
		}

		if !fired {
			let _ = lua.globals().set("_current_btn", name);
			state.mapper.lock().unwrap().button_down(name, now);
			for b in &state.cfg.bindings {
				if b.button == name && b.event == BindingEvent::Press {
					execute_lua_function(b.func_idx, callbacks, lua, &mut state.pending);
				}
			}
		}
	}

	// Retractively consume any held button whose modeshift is now fully satisfied
	// by this newly pressed button acting as a modifier
	let retro_consume: Vec<String> = {
		let held = state.mapper.lock().unwrap().held_buttons();
		held.iter().filter(|held_btn| {
			**held_btn != *name
			&& state.cfg.modeshifts.iter().any(|ms| {
				ms.button == **held_btn
				&& ms.modifiers.iter().any(|m| m == name)
				&& ms.modifiers.iter().all(|m| m == name || held.contains(m))
			})
		}).cloned().collect()
	};
	for btn in retro_consume {
		state.mapper.lock().unwrap().mark_consumed(&btn);
	}
}

fn handle_btn_up(
	name: &str,
	state: &mut OutputState,
	callbacks: &[Arc<RegistryKey>],
	lua: &Lua,
) {
	log_msg(1, &format!("{} up", name));
	let now = Instant::now();

	let consumed = {
		let mut m = state.mapper.lock().unwrap();
		if m.is_consumed(name) {
			m.unmark_consumed(name);
			for key in m.button_up(name) {
				state.dev.apply(&key, false);
			}
			true
		} else {
			false
		}
	};

	if !consumed {
		let _ = lua.globals().set("_current_btn", name);
		let has_dp = state.cfg.double_press.iter().any(|d| d.button == name);

		let is_tap = state.mapper.lock().unwrap().is_tap(name, now);
		if is_tap {
			for b in &state.cfg.bindings {
				if b.button == name && b.event == BindingEvent::Tap {
					if has_dp {
						let window = state.cfg.double_press.iter()
							.find(|d| d.button == name)
							.map(|d| d.window_ms)
							.unwrap_or(200);
						state.defer_map.insert(name.to_string(), (Instant::now() + Duration::from_millis(window), b.func_idx));
					} else {
						execute_lua_function(b.func_idx, callbacks, lua, &mut state.pending);
					}
				}
			}
		}

		for b in &state.cfg.bindings {
			if b.button == name && b.event == BindingEvent::Release {
				execute_lua_function(b.func_idx, callbacks, lua, &mut state.pending);
			}
		}

		for key in state.mapper.lock().unwrap().button_up(name) {
			state.dev.apply(&key, false);
		}
	}
}

fn extract_helper_key(val: mlua::Value) -> mlua::Result<String> {
	match val {
		mlua::Value::Table(t) => {
			if let Ok(kind) = t.get::<String>("__kind") {
				if kind == "ref" {
					return t.get("val");
				}
			}
			Err(mlua::Error::external("expected a key/mouse/con reference"))
		}
		mlua::Value::String(s) => Ok(s.to_string_lossy().to_string()),
		_ => Err(mlua::Error::external("expected a key/mouse/con reference")),
	}
}

fn register_lua_helpers(lua: &Lua, mapper: &Arc<Mutex<Mapper>>, gyro_tx: Sender<GyroCmd>) {
	let mapper = mapper.clone();

	let wrap_btn = |lua: &Lua, f: fn(&mut Mapper, &str, &str)| -> mlua::Function {
		let m = mapper.clone();
		lua.clone()		.create_function(move |lua, (key_val,): (mlua::Value,)| -> mlua::Result<()> {
			let raw_btn: String = lua.globals().get("_current_btn").unwrap_or_default();
			let btn: &str = if raw_btn.is_empty() { "__frame__" } else { &raw_btn };
			let key = extract_helper_key(key_val)?;
			f(&mut *m.lock().unwrap(), btn, &key);
			Ok(())
		}).unwrap()
	};

	lua.globals().set("press", wrap_btn(lua, |m, btn, key| m.press_key(btn, key))).unwrap();

	{
		let m = mapper.clone();
		lua.globals().set("instant", lua.clone()
			.create_function(move |_, (key_val, opts): (mlua::Value, Option<mlua::Table>)| -> mlua::Result<()> {
				let key = extract_helper_key(key_val)?;
				let override_ms = opts.and_then(|t| t.get::<u64>("press_time").ok());
				m.lock().unwrap().instant_key(&key, override_ms);
				Ok(())
			}).unwrap()
		).unwrap();
	}

	lua.globals().set("release", wrap_btn(lua, |m, btn, key| m.release_key(btn, key))).unwrap();

	{
		let m = mapper.clone();
		lua.globals().set("toggle", lua.clone()
			.create_function(move |_, (key_val,): (mlua::Value,)| -> mlua::Result<()> {
				let key = extract_helper_key(key_val)?;
				m.lock().unwrap().toggle_key(&key);
				Ok(())
			}).unwrap()
		).unwrap();
	}

	lua.globals().set("turbo", wrap_btn(lua, |m, btn, key| m.turbo_key(btn, key, Instant::now()))).unwrap();

	{
		let m = mapper.clone();
		lua.globals().set("held", lua.clone()
			.create_function(move |_, (key_val,): (mlua::Value,)| -> mlua::Result<bool> {
				let key = extract_helper_key(key_val)?;
				let held = m.lock().unwrap().held_buttons();
				Ok(held.contains(&key))
			}).unwrap()
		).unwrap();
	}

	lua.globals().set("log", lua.clone()
		.create_function(|_, (level, msg): (u8, String)| -> mlua::Result<()> {
			log_msg(level, &msg);
			Ok(())
		}).unwrap()
	).unwrap();

	let gt = gyro_tx.clone();
	lua.globals().set("gyro_enable", lua.clone().create_function(move |_, ()| {
		gt.send(GyroCmd::Enable).ok();
		Ok(())
	}).unwrap()).unwrap();

	let gt = gyro_tx.clone();
	lua.globals().set("gyro_disable", lua.clone().create_function(move |_, ()| {
		gt.send(GyroCmd::Disable).ok();
		Ok(())
	}).unwrap()).unwrap();

	let gt = gyro_tx.clone();
	lua.globals().set("gyro_toggle", lua.clone().create_function(move |_, ()| {
		gt.send(GyroCmd::Toggle).ok();
		Ok(())
	}).unwrap()).unwrap();

	let gt = gyro_tx.clone();
	let hold_lua = lua.clone();
	lua.globals().set("gyro_hold", lua.clone().create_function(move |_, ()| {
		let btn: String = hold_lua.globals().get("_current_btn").unwrap_or_default();
		gt.send(GyroCmd::Hold(btn)).ok();
		Ok(())
	}).unwrap()).unwrap();
}

fn main() {
	let args: Vec<String> = std::env::args().collect();
	if args.len() >= 2 {
		match args[1].as_str() {
			"--gen-meta" => {
				let path = args.get(2).map(|s| s.as_str()).unwrap_or("moonglide.d.lua");
				match std::fs::write(path, include_str!("meta.d.lua")) {
					Ok(_) => {
						println!("wrote {}", style::info(path));
						std::process::exit(0);
					}
					Err(e) => {
						eprintln!("{}", style::err(&format!("error writing {}: {}", path, e)));
						std::process::exit(1);
					}
				}
			}
			flag if flag.starts_with("--") => {
				eprintln!("{}", style::err(&format!("unknown flag: {flag}")));
				eprintln!("usage: moonglide [config.lua]");
				eprintln!("       moonglide --gen-meta [path]");
				std::process::exit(1);
			}
			_ => {}
		}
	}

	let sdl = match sdl2::init() {
		Ok(s) => s,
		Err(e) => { eprintln!("{}", style::err(&format!("error: sdl init failed: {}", e))); return; }
	};

	let mut event_pump = match sdl.event_pump() {
		Ok(p) => p,
		Err(e) => { eprintln!("{}", style::err(&format!("error: event pump: {}", e))); return; }
	};

	let game_controller_subsys = match sdl.game_controller() {
		Ok(g) => g,
		Err(e) => { eprintln!("{}", style::err(&format!("error: game controller subsystem: {}", e))); return; }
	};

	let lua = Lua::new();

	let shared_cfg = Arc::new(Mutex::new(Config::default()));
	let shared_cbs: Arc<Mutex<Vec<Arc<RegistryKey>>>> = Arc::new(Mutex::new(Vec::new()));
	let gyro_shared: Arc<Mutex<GyroConfig>> = Arc::new(Mutex::new(GyroConfig::default()));
	let (cal_tx, cal_rx) = std::sync::mpsc::channel::<CalCmd>();
	let (gyro_tx, gyro_rx) = std::sync::mpsc::channel::<GyroCmd>();

	let mapper = Arc::new(Mutex::new(Mapper::new()));

	if let Err(e) = config::setup_dsl(&lua, &gyro_shared, &cal_tx) {
		eprintln!("{}", style::err(&format!("setup_dsl: {}", e)));
		return;
	}
	register_lua_helpers(&lua, &mapper, gyro_tx);

	let cfg: Config = match std::env::args().nth(1) {
		Some(path) => match config::load(&path, &lua, &shared_cfg, &shared_cbs, &gyro_shared) {
			Ok(cfg) => {
				println!("{}", style::info(&format!("config loaded from {}: {} bindings, {} chords",
					path, cfg.bindings.len(), cfg.chords.len())));
				cfg
			}
			Err(e) => {
				eprintln!("{}", style::warn(&format!("warning: config error ({}), running with empty config", e)));
				config::init_bare(&lua, &shared_cfg, &shared_cbs, &gyro_shared);
				Config::default()
			}
		},
		None => {
			println!("{}", style::warn("no config specified, running with empty config"));
			config::init_bare(&lua, &shared_cfg, &shared_cbs, &gyro_shared);
			Config::default()
		}
	};

	let mut ctrl_mgr = match controller::ControllerManager::open(game_controller_subsys) {
		Ok(m) => { println!("{}", style::info(&format!("{} controller(s) found at startup", m.controllers.len()))); m }
		Err(e) => { eprintln!("{}", style::err(&format!("error: controller manager: {}", e))); return; }
	};

	let mouse = VirtualMouse::new().ok();
	let kbd = VirtualKeyboard::new().ok();

	let read_dz = |name: &str, def: f64| -> u16 {
		((lua.globals().get::<f64>(name).unwrap_or(def)).clamp(0.0, 1.0) * MAX_AXIS) as u16
	};

	let load_globals = |lua: &Lua| {
		LOG_LEVEL.store(lua.globals().get::<u8>("log_level").unwrap_or(0), Ordering::Relaxed);
		TRIGGER_THRESHOLD.store(lua.globals().get::<u16>("trigger_threshold").unwrap_or(3000), Ordering::Relaxed);
		INSTANT_PRESS_TIME.store(lua.globals().get::<u64>("instant_press_time").unwrap_or(40), Ordering::Relaxed);
		stick::LEFT_STICK_INNER.store(read_dz("left_stick_inner_deadzone", 0.15), Ordering::Relaxed);
		stick::LEFT_STICK_OUTER.store(read_dz("left_stick_outer_deadzone", 1.0), Ordering::Relaxed);
		stick::RIGHT_STICK_INNER.store(read_dz("right_stick_inner_deadzone", 0.15), Ordering::Relaxed);
		stick::RIGHT_STICK_OUTER.store(read_dz("right_stick_outer_deadzone", 1.0), Ordering::Relaxed);
		stick::LEFT_RING_POSITION.store(read_dz("left_ring_position", 0.8), Ordering::Relaxed);
		stick::RIGHT_RING_POSITION.store(read_dz("right_ring_position", 0.8), Ordering::Relaxed);
	};

	load_globals(&lua);

	let mut state = OutputState {
		dev: OutputDevices { mouse, kbd },
		mapper,
		triggers: TriggerTracker::new(),
		gyro: GyroState::new(&cfg.gyro),
		axis_state: HashMap::new(),
		prev_stick_dirs: HashMap::new(),
		defer_map: HashMap::new(),
		pending: Vec::new(),
		cfg,
		gyro_rx,
	};

	println!("{}", style::bold("Moonglide running. Press Escape to quit."));
	println!("{}", style::dim("Type Lua commands in the terminal."));

	{
		let sc = shared_cfg.clone();
		let scb = shared_cbs.clone();
		let lua2 = lua.clone();
		let m = state.mapper.clone();
		let reset_fn = lua.create_function(move |_, ()| -> mlua::Result<()> {
			m.lock().unwrap().release_all();
			sc.lock().unwrap().bindings.clear();
			sc.lock().unwrap().chords.clear();
			sc.lock().unwrap().double_press.clear();
			sc.lock().unwrap().modeshifts.clear();
			scb.lock().unwrap().clear();
			let _ = lua2.load("_bindings = {}; _chords = {}; _double_press = {}; _modeshifts = {}").exec();
			Ok(())
		}).unwrap();
		lua.globals().set("reset", reset_fn).unwrap();
	}

	let (repl_tx, repl_rx): (Sender<String>, Receiver<String>) = std::sync::mpsc::channel();
	let repl_running = Arc::new(AtomicBool::new(true));

	{
		let running = repl_running.clone();
		std::thread::spawn(move || {
			let mut line = String::new();
			while running.load(std::sync::atomic::Ordering::Relaxed) {
				line.clear();
				match std::io::stdin().read_line(&mut line) {
					Ok(0) => break,
					Ok(_) => { let t = line.trim().to_string(); if !t.is_empty() { let _ = repl_tx.send(t); } }
					Err(_) => break,
				}
			}
		});
	}

	'running: loop {
		for event in event_pump.poll_iter() {
			match &event {
				Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'running,
				_ => {}
			}

			for ce in ctrl_mgr.handle_event(&event) {
				match ce {
					ControllerEvent::AxisMotion(idx, val, which) => {
						let axes = state.axis_state.entry(which).or_insert([0i16; 6]);
						if idx >= 101 && idx <= 106 {
							axes[(idx - 101) as usize] = val;
						}
				if let Some((name, pressed)) = state.triggers.process(which, idx, val) {
					if pressed {
						handle_btn_down(&name, &mut state, &shared_cbs.lock().unwrap().clone(), &lua);
					} else {
						handle_btn_up(&name, &mut state, &shared_cbs.lock().unwrap().clone(), &lua);
					}
				}
					}
					ControllerEvent::ButtonDown(btn_idx) => {
						handle_btn_down(&idx_to_button_name(btn_idx), &mut state, &shared_cbs.lock().unwrap().clone(), &lua);
					}
					ControllerEvent::ButtonUp(btn_idx) => {
						handle_btn_up(&idx_to_button_name(btn_idx), &mut state, &shared_cbs.lock().unwrap().clone(), &lua);
					}
					ControllerEvent::TouchpadTouch => {
						handle_btn_down("touchpad_touch", &mut state, &shared_cbs.lock().unwrap().clone(), &lua);
					}
					ControllerEvent::TouchpadUntouch => {
						handle_btn_up("touchpad_touch", &mut state, &shared_cbs.lock().unwrap().clone(), &lua);
					}
					ControllerEvent::Gyro { x, y, z } => state.gyro.process_gyro(x, y, z, &mut state.dev),
					ControllerEvent::Connected(id) => println!("{}", style::green(&format!("controller connected (instance {})", id))),
					ControllerEvent::Disconnected(id) => {
						println!("{}", style::yellow(&format!("controller disconnected (instance {})", id)));
						state.gyro.reset();
						state.axis_state.remove(&id);
					}
				}
			}
		}

		poll_pending_threads(&mut state.pending, &lua);

		{
			let now = Instant::now();
			state.defer_map.retain(|name, (fire_at, func_idx)| {
				if now >= *fire_at {
					let _ = lua.globals().set("_current_btn", name.clone());
					let cbs = shared_cbs.lock().unwrap().clone();
					execute_lua_function(*func_idx, &cbs, &lua, &mut state.pending);
					false
				} else {
					true
				}
			});
		}

		while let Ok(cmd) = repl_rx.try_recv() {
			match lua.load(&cmd).exec() {
				Ok(()) => {
					println!("{}", style::green("> ok"));
					state.cfg = shared_cfg.lock().unwrap().clone();
					load_globals(&lua);
					state.gyro = GyroState::new(&state.cfg.gyro);
				}
				Err(e) => eprintln!("{}", style::err(&format!("> error: {}", e))),
			}
		}

		while let Ok(cmd) = cal_rx.try_recv() {
			match cmd {
				CalCmd::Start => state.gyro.start_calibration(),
				CalCmd::Stop => state.gyro.stop_calibration(),
			}
		}

		for cmd in state.gyro_rx.try_iter() {
			match cmd {
				GyroCmd::Enable => state.gyro.enable(),
				GyroCmd::Disable => state.gyro.disable(),
				GyroCmd::Toggle => state.gyro.toggle(),
				GyroCmd::Hold(btn) => state.gyro.set_hold(btn),
			}
		}

		{
			let held = state.mapper.lock().unwrap().held_buttons();
			state.gyro.process_hold(&held);
		}

		state.mapper.lock().unwrap().begin_frame();

		{
			let lua_globals = lua.globals();
			if let Ok(update_fn) = lua_globals.get::<mlua::Function>("update") {
				let _ = lua_globals.set("_current_btn", "__frame__");
				let _ = lua.create_thread(update_fn)
					.and_then(|t| t.resume::<mlua::MultiValue>(()));
				let _ = lua_globals.set("_current_btn", "");
			}
		}

		let cbs = shared_cbs.lock().unwrap().clone();
		process_stick_buttons(
			&state.axis_state, &mut state.prev_stick_dirs, &state.cfg, &cbs,
			&lua, &state.mapper, &mut state.dev, &mut state.pending,
		);
		process_hold_turbo(&state.cfg, &cbs, &lua, &state.mapper, &mut state.pending);

		{
			let mut m = state.mapper.lock().unwrap();
			m.process_turbo(Instant::now());
			let delay = INSTANT_PRESS_TIME.load(Ordering::Relaxed);
			m.process_instant_releases(Instant::now(), delay);
		}

		for (key, press) in state.mapper.lock().unwrap().drain_actions() {
			state.dev.apply(&key, press);
		}

		state.dev.synchronize_all();

		std::thread::sleep(std::time::Duration::from_secs_f64(1.0 / 240.0));
	}

	state.mapper.lock().unwrap().release_all();
	repl_running.store(false, std::sync::atomic::Ordering::Relaxed);
}

fn fire_chord(
	held: &[&str],
	new_btn: &str,
	cfg: &Config,
	callbacks: &[Arc<RegistryKey>],
	lua: &Lua,
	mapper: &Arc<Mutex<Mapper>>,
	pending: &mut Vec<PendingThread>,
) -> bool {
	let mut all: Vec<String> = held.iter().map(|s| s.to_string()).collect();
	if !all.contains(&new_btn.to_string()) {
		all.push(new_btn.to_string());
	}

	for chord in &cfg.chords {
		if chord.buttons.iter().all(|b| all.contains(b)) {
			let _ = lua.globals().set("_current_btn", new_btn);
			mapper.lock().unwrap().button_down(new_btn, Instant::now());
			execute_lua_function(chord.func_idx, callbacks, lua, pending);
			return true;
		}
	}
	false
}

fn process_hold_turbo(
	cfg: &Config,
	callbacks: &[Arc<RegistryKey>],
	lua: &Lua,
	mapper: &Arc<Mutex<Mapper>>,
	pending: &mut Vec<PendingThread>,
) {
	let now = Instant::now();
	let held = mapper.lock().unwrap().held_buttons();

	for btn in &held {
		let _ = lua.globals().set("_current_btn", btn.to_string());

		for b in &cfg.bindings {
			if &b.button != btn { continue; }

			let fire = match b.event {
				BindingEvent::Hold => mapper.lock().unwrap().hold_elapsed(btn, b.hold_delay_ms, now),
				BindingEvent::Turbo => mapper.lock().unwrap().turbo_event_due(btn, now),
				_ => false,
			};
			if fire {
				execute_lua_function(b.func_idx, callbacks, lua, pending);
			}
		}
	}
}
