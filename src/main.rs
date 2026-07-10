mod api;
mod types;
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
use std::time::Instant;

use mlua::Lua;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use types::{Config, GyroCmd, GyroConfig};
use config::CalCmd;
use controller::{idx_to_button_name, ControllerEvent};
use gyro_state::GyroState;
use lua_coroutines::{call_on_btn_down, call_on_btn_up, poll_pending_threads, resume_thread, PendingThread};
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
	gyro_rx: std::sync::mpsc::Receiver<GyroCmd>,
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

	let gyro_shared: Arc<Mutex<GyroConfig>> = Arc::new(Mutex::new(GyroConfig::default()));
	let (cal_tx, cal_rx) = std::sync::mpsc::channel::<CalCmd>();
	let (gyro_tx, gyro_rx) = std::sync::mpsc::channel::<GyroCmd>();

	let mapper = Arc::new(Mutex::new(Mapper::new()));

	api::register_api(&lua, &mapper, &gyro_tx);

	// ── setup DSL + bindings library ──
	if let Err(e) = config::setup_dsl(&lua, &gyro_shared, &cal_tx) {
		eprintln!("{}", style::err(&format!("setup_dsl: {}", e)));
		return;
	}

	// ── load config ──
	let cfg: Config = match std::env::args().nth(1) {
		Some(path) => match config::load(&path, &lua, &Arc::new(Mutex::new(Config::default())), &gyro_shared) {
			Ok(cfg) => {
				println!("{}", style::info(&format!("config loaded from {}", path)));
				cfg
			}
			Err(e) => {
				eprintln!("{}", style::warn(&format!("warning: config error ({}), running with empty config", e)));
				config::init_bare(&lua, &Arc::new(Mutex::new(Config::default())), &gyro_shared);
				Config::default()
			}
		},
		None => {
			println!("{}", style::warn("no config specified, running with empty config"));
			config::init_bare(&lua, &Arc::new(Mutex::new(Config::default())), &gyro_shared);
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
		gyro_rx,
	};

	println!("{}", style::bold("Moonglide running. Press Escape to quit."));
	println!("{}", style::dim("Type Lua commands in the terminal."));

	let mut pending: Vec<PendingThread> = Vec::new();

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
								handle_btn_down(&name, &mut state, &lua, &mut pending);
							} else {
								handle_btn_up(&name, &mut state, &lua, &mut pending);
							}
						}
					}
					ControllerEvent::ButtonDown(btn_idx) => {
						handle_btn_down(&idx_to_button_name(btn_idx), &mut state, &lua, &mut pending);
					}
					ControllerEvent::ButtonUp(btn_idx) => {
						handle_btn_up(&idx_to_button_name(btn_idx), &mut state, &lua, &mut pending);
					}
					ControllerEvent::TouchpadTouch => {
						handle_btn_down("touchpad_touch", &mut state, &lua, &mut pending);
					}
					ControllerEvent::TouchpadUntouch => {
						handle_btn_up("touchpad_touch", &mut state, &lua, &mut pending);
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

		while let Ok(cmd) = repl_rx.try_recv() {
			match lua.load(&cmd).exec() {
				Ok(()) => {
					println!("{}", style::green("> ok"));
					load_globals(&lua);
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

		// ── process stick directions ──
		let (new_dirs, removed_dirs) = process_stick_buttons(
			&state.axis_state, &mut state.prev_stick_dirs,
		);
		for dir in &new_dirs {
			state.mapper.lock().unwrap().button_down(dir, Instant::now());
			call_on_btn_down(&lua, dir, &mut pending);
		}
		for dir in &removed_dirs {
			state.mapper.lock().unwrap().button_up(dir);
			call_on_btn_up(&lua, dir, &mut pending);
		}

		// ── Lua frame callback ──
		if let Ok(on_update_fn) = lua.globals().get::<mlua::Function>("on_update") {
			if let Ok(thread) = lua.create_thread(on_update_fn) {
				resume_thread(&lua, thread, &mut pending, "on_update");
			}
		}

		poll_pending_threads(&mut pending, &lua);

		for (key, press) in state.mapper.lock().unwrap().drain_actions() {
			state.dev.apply(&key, press);
		}

		state.dev.synchronize_all();

		std::thread::sleep(std::time::Duration::from_secs_f64(1.0 / 240.0));
	}

	state.mapper.lock().unwrap().release_all();
	repl_running.store(false, std::sync::atomic::Ordering::Relaxed);
}

fn handle_btn_down(name: &str, state: &mut OutputState, lua: &Lua, pending: &mut Vec<PendingThread>) {
	let now = Instant::now();
	state.mapper.lock().unwrap().button_down(name, now);
	call_on_btn_down(lua, name, pending);
}

fn handle_btn_up(name: &str, state: &mut OutputState, lua: &Lua, pending: &mut Vec<PendingThread>) {
	state.mapper.lock().unwrap().button_up(name);
	call_on_btn_up(lua, name, pending);
}
