mod api;
mod config;
mod controller;
mod lua_coroutines;
mod mapping;
mod output;
mod output_devices;

mod frame_pacer;
mod style;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use mlua::Lua;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use controller::idx_to_button_name;
use controller::ControllerEvent;
use lua_coroutines::{call_on_btn_down, call_on_btn_up, poll_pending_threads, resume_thread, PendingThread};
use mapping::Mapper;
use output::keyboard::VirtualKeyboard;
use output::mouse::VirtualMouse;
use output_devices::OutputDevices;

pub(crate) static LOG_LEVEL: AtomicU8 = AtomicU8::new(0);
pub(crate) fn log_msg(level: u8, msg: &str) {
	if LOG_LEVEL.load(Ordering::Relaxed) >= level {
		println!("{} {}", style::dim(&format!("[{}]", level)), msg);
	}
}

struct OutputState {
	dev: OutputDevices,
	mapper: Arc<Mutex<Mapper>>,
	axis_state: HashMap<u32, [i16; 6]>,
	prev_buttons: HashMap<u32, u32>,
	last_frame_time: Option<Instant>,
}

fn parse_args() -> Option<String> {
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
			_ => return Some(args[1].clone()),
		}
	}
	None
}

fn init_sdl() -> (sdl2::Sdl, sdl2::EventPump, sdl2::GameControllerSubsystem) {
	let sdl = sdl2::init().unwrap_or_else(|e| {
		eprintln!("{}", style::err(&format!("error: sdl init failed: {}", e)));
		std::process::exit(1);
	});
	let event_pump = sdl.event_pump().unwrap_or_else(|e| {
		eprintln!("{}", style::err(&format!("error: event pump: {}", e)));
		std::process::exit(1);
	});
	let game_controller_subsys = sdl.game_controller().unwrap_or_else(|e| {
		eprintln!("{}", style::err(&format!("error: game controller subsystem: {}", e)));
		std::process::exit(1);
	});
	(sdl, event_pump, game_controller_subsys)
}

fn init_lua(mapper: &Arc<Mutex<Mapper>>, config_path: Option<String>) -> Lua {
	let lua = Lua::new();
	api::register_api(&lua, mapper);

	let _ = lua.globals().set("_gyro_raw", lua.create_table().unwrap());
	let _ = lua.globals().set("_accel_raw", lua.create_table().unwrap());
	let _ = lua.globals().set("_gravity", lua.create_table().unwrap());
	let _ = lua.globals().set("_orientation", lua.create_table().unwrap());

	config::setup_dsl(&lua).unwrap_or_else(|e| {
		eprintln!("{}", style::err(&format!("setup_dsl: {}", e)));
		std::process::exit(1);
	});

	match config_path {
		Some(path) => match config::load(&path, &lua) {
			Ok(()) => println!("{}", style::info(&format!("config loaded from {}", path))),
			Err(e) => {
				eprintln!("{}", style::warn(&format!("warning: config error ({}), running with empty config", e)));
				config::init_bare(&lua);
			}
		},
		None => {
			println!("{}", style::warn("no config specified, running with empty config"));
			config::init_bare(&lua);
		}
	}
	lua
}

fn init_controllers(subsys: sdl2::GameControllerSubsystem) -> controller::ControllerManager {
	controller::ControllerManager::open(subsys).unwrap_or_else(|e| {
		eprintln!("{}", style::err(&format!("error: controller manager: {}", e)));
		std::process::exit(1);
	})
}

fn handle_btn_down(name: &str, state: &mut OutputState, lua: &Lua, pending: &mut Vec<PendingThread>) {
	state.mapper.lock().unwrap().button_down(name);
	call_on_btn_down(lua, name, pending);
}

fn handle_btn_up(name: &str, state: &mut OutputState, lua: &Lua, pending: &mut Vec<PendingThread>) {
	state.mapper.lock().unwrap().button_up(name);
	call_on_btn_up(lua, name, pending);
}

fn main() {
	style::init();
	let config_path = parse_args();
	let (_sdl, mut event_pump, game_controller_subsys) = init_sdl();

	let mapper = Arc::new(Mutex::new(Mapper::new()));
	let lua = init_lua(&mapper, config_path);

	let mut ctrl_mgr = init_controllers(game_controller_subsys);
	println!("{}", style::info(&format!("{} controller(s) found at startup", ctrl_mgr.controllers.len())));

	let mouse = VirtualMouse::new().ok();
	let kbd = VirtualKeyboard::new().ok();

	let load_globals = |lua: &Lua| {
		LOG_LEVEL.store(lua.globals().get::<u8>("log_level").unwrap_or(0), Ordering::Relaxed);
	};

	load_globals(&lua);

	let mut state = OutputState {
		dev: OutputDevices { mouse, kbd },
		mapper,
		axis_state: HashMap::new(),
		prev_buttons: HashMap::new(),
		last_frame_time: None,
	};

	println!("{}", style::bold("Moonglide running. Press Escape to quit."));
	println!("{}", style::dim("Type Lua commands in the terminal."));

	let mut pending: Vec<PendingThread> = Vec::new();
	let mut pacer = frame_pacer::FramePacer::new(1000.0);

	let (repl_tx, repl_rx) = std::sync::mpsc::channel::<String>();

	std::thread::spawn(move || {
		let mut line = String::new();
		while let Ok(n) = std::io::stdin().read_line(&mut line) {
			if n == 0 { break; }
			let t = line.trim().to_string();
			if !t.is_empty() { let _ = repl_tx.send(t); }
			line.clear();
		}
	});

	'running: loop {
		for event in event_pump.poll_iter() {
			match &event {
				Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'running,
				_ => {}
			}

			for ce in ctrl_mgr.handle_event(&event) {
				match ce {
					ControllerEvent::TouchpadTouch => {
						handle_btn_down("touchpad_touch", &mut state, &lua, &mut pending);
					}
					ControllerEvent::TouchpadUntouch => {
						handle_btn_up("touchpad_touch", &mut state, &lua, &mut pending);
					}
					ControllerEvent::Connected(id) => println!("{}", style::green(&format!("controller connected (instance {})", id))),
					ControllerEvent::Disconnected(id) => {
						println!("{}", style::yellow(&format!("controller disconnected (instance {})", id)));
						let _ = lua.globals().get::<mlua::Function>("gyro_reset").map(|f| f.call::<()>(()));
						let _ = lua.globals().get::<mlua::Function>("cleanup_controller").map(|f| f.call::<()>((id,)));
						state.axis_state.remove(&id);
						state.prev_buttons.remove(&id);
					}
				}
			}
		}

		// dt capped at 100ms to prevent fusion explosion on pause/resume
		let frame_dt = state.last_frame_time
			.map(|t| (Instant::now() - t).as_secs_f64().min(0.1))
			.unwrap_or(0.0);
		state.last_frame_time = Some(Instant::now());

		for id in ctrl_mgr.connected_ids() {
			for (btn_idx, pressed) in ctrl_mgr.poll_buttons(id, state.prev_buttons.entry(id).or_insert(0)) {
				let name = idx_to_button_name(btn_idx);
				if pressed {
					handle_btn_down(&name, &mut state, &lua, &mut pending);
				} else {
					handle_btn_up(&name, &mut state, &lua, &mut pending);
				}
			}

			if let Some(axes) = ctrl_mgr.poll_axes(id) {
				state.axis_state.insert(id, axes);
			}

			if let Some((gx, gy, gz, ax, ay, az)) = ctrl_mgr.poll_sensors(id) {
				if let Ok(f) = lua.globals().get::<mlua::Function>("on_sensor_event") {
					let _ = f.call::<()>((gx as f64, gy as f64, gz as f64, ax as f64, ay as f64, az as f64, frame_dt, true));
				}

				if let Ok(result) = lua.globals().get::<mlua::Function>("process_gyro")
					.and_then(|f| f.call::<mlua::Table>((gx as f64, gy as f64, gz as f64, frame_dt)))
				{
					let dx = result.get("dx").unwrap_or(0.0);
					let dy = result.get("dy").unwrap_or(0.0);
					if dx != 0.0 || dy != 0.0 {
						if let Some(mouse) = state.dev.mouse.as_mut() {
							let _ = mouse.move_mouse(dx, dy);
						}
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

		// ── process stick directions ──
		if let Ok(on_sticks) = lua.globals().get::<mlua::Function>("process_sticks") {
			for (which, axes) in state.axis_state.iter() {
				let lx = axes[0] as f64;
				let ly = -(axes[1] as f64);
				let rx = axes[2] as f64;
				let ry = -(axes[3] as f64);
				let lt = axes[4] as f64;
				let rt = axes[5] as f64;
				match on_sticks.call::<mlua::Table>((*which, lx, ly, rx, ry, lt, rt)) {
					Ok(result) => {
						if let Ok(pressed) = result.get::<Vec<String>>("pressed") {
							for dir in &pressed {
								state.mapper.lock().unwrap().button_down(dir);
								call_on_btn_down(&lua, dir, &mut pending);
							}
						}
						if let Ok(released) = result.get::<Vec<String>>("released") {
							for dir in &released {
								state.mapper.lock().unwrap().button_up(dir);
								call_on_btn_up(&lua, dir, &mut pending);
							}
						}
					}
					Err(_) => {}
				}
			}
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

		pacer.wait();
	}

	state.mapper.lock().unwrap().release_all();
}
