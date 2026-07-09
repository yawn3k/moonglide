use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Instant;

use mlua::{Lua, RegistryKey};

use crate::bindings::{BindingEvent, Config};
use crate::log_msg;
use crate::lua_coroutines::{execute_lua_function, PendingThread};
use crate::mapping::Mapper;
use crate::output_devices::OutputDevices;

pub const MAX_AXIS: f64 = 32767.0;
pub const CROSS_GATE_THRESHOLD: f64 = 0.5;

pub static LEFT_STICK_INNER: AtomicU16 = AtomicU16::new(4915);
pub static LEFT_STICK_OUTER: AtomicU16 = AtomicU16::new(32767);
pub static RIGHT_STICK_INNER: AtomicU16 = AtomicU16::new(4915);
pub static RIGHT_STICK_OUTER: AtomicU16 = AtomicU16::new(32767);
pub static LEFT_RING_POSITION: AtomicU16 = AtomicU16::new(26213);
pub static RIGHT_RING_POSITION: AtomicU16 = AtomicU16::new(26213);

pub fn process_deadzones(x: &mut f64, y: &mut f64, inner: f64, outer: f64) -> bool {
	let length = ((*x) * (*x) + (*y) * (*y)).sqrt();
	if length <= inner {
		*x = 0.0;
		*y = 0.0;
		return false;
	}
	if length >= outer {
		*x /= length;
		*y /= length;
		return true;
	}
	let scaled = (length - inner) / (outer - inner);
	let rescale = scaled / length;
	*x *= rescale;
	*y *= rescale;
	false
}

pub fn cross_gate(x: f64, y: f64, side: &str, out: &mut HashSet<String>) {
	let abs_x = x.abs();
	let abs_y = y.abs();
	if x < -CROSS_GATE_THRESHOLD * abs_y { out.insert(format!("{}_stick_left", side)); }
	if x >  CROSS_GATE_THRESHOLD * abs_y { out.insert(format!("{}_stick_right", side)); }
	if y < -CROSS_GATE_THRESHOLD * abs_x { out.insert(format!("{}_stick_down", side)); }
	if y >  CROSS_GATE_THRESHOLD * abs_x { out.insert(format!("{}_stick_up", side)); }
}

pub fn process_stick_buttons(
	axis_state: &HashMap<u32, [i16; 6]>,
	prev: &mut HashMap<u32, HashSet<String>>,
	cfg: &Config,
	callbacks: &[Arc<RegistryKey>],
	lua: &Lua,
	mapper: &Arc<Mutex<Mapper>>,
	dev: &mut OutputDevices,
	pending: &mut Vec<PendingThread>,
) {
	let now = Instant::now();

	let inner_l = LEFT_STICK_INNER.load(Ordering::Relaxed) as f64;
	let outer_l = LEFT_STICK_OUTER.load(Ordering::Relaxed) as f64;
	let inner_r = RIGHT_STICK_INNER.load(Ordering::Relaxed) as f64;
	let outer_r = RIGHT_STICK_OUTER.load(Ordering::Relaxed) as f64;
	let ring_pos_l = LEFT_RING_POSITION.load(Ordering::Relaxed) as f64 / MAX_AXIS;
	let ring_pos_r = RIGHT_RING_POSITION.load(Ordering::Relaxed) as f64 / MAX_AXIS;

	for (which, axes) in axis_state.iter() {
		let mut lx = axes[0] as f64;
		let mut ly = -(axes[1] as f64);
		let mut rx = axes[2] as f64;
		let mut ry = -(axes[3] as f64);

		process_deadzones(&mut lx, &mut ly, inner_l, outer_l);
		process_deadzones(&mut rx, &mut ry, inner_r, outer_r);

		let proc_l_len = (lx * lx + ly * ly).sqrt();
		let proc_r_len = (rx * rx + ry * ry).sqrt();

		let mut current = HashSet::new();
		cross_gate(lx, ly, "left", &mut current);
		cross_gate(rx, ry, "right", &mut current);

		let prev_dirs = prev.entry(*which).or_default();

		if proc_l_len > 0.0 && proc_l_len < ring_pos_l {
			current.insert("left_ring_inner".to_string());
		}
		if proc_l_len > ring_pos_l {
			current.insert("left_ring_outer".to_string());
		}
		if proc_r_len > 0.0 && proc_r_len < ring_pos_r {
			current.insert("right_ring_inner".to_string());
		}
		if proc_r_len > ring_pos_r {
			current.insert("right_ring_outer".to_string());
		}

		log_msg(1, &format!("ctrl[{}] raw=({},{},{},{}) dz=({:.3},{:.3},{:.3},{:.3}) dirs={:?}",
			which, axes[0], axes[1], axes[2], axes[3],
			lx, ly, rx, ry, current));

		let new_dirs: Vec<String> = current.iter()
			.filter(|d| !prev_dirs.contains(d.as_str()))
			.cloned()
			.collect();

		for dir in &new_dirs {
			log_msg(1, &format!(">> press {}", dir));
			mapper.lock().unwrap().button_down(dir, now);
		}

		let mut chord_consumed: HashSet<&str> = HashSet::new();
		{
			let held = mapper.lock().unwrap().held_buttons();
			for chord in &cfg.chords {
				if chord.buttons.iter().all(|b| held.contains(b)) {
					log_msg(1, &format!(">> chord {:?}", chord.buttons));
					let _ = lua.globals().set("_current_btn", chord.buttons.last().unwrap().clone());
					execute_lua_function(chord.func_idx, callbacks, lua, pending);
					for b in &chord.buttons {
						chord_consumed.insert(b.as_str());
					}
				}
			}
		}

		for dir in &new_dirs {
			if chord_consumed.contains(dir.as_str()) { continue; }
			let held = mapper.lock().unwrap().held_buttons();
			if let Some(ms) = cfg.modeshifts.iter().find(|m| m.button == *dir && m.modifiers.iter().all(|modifier| held.contains(modifier))) {
				log_msg(1, &format!(">> modeshift {:?} on {}", ms.modifiers, dir));
				let _ = lua.globals().set("_current_btn", dir.clone());
				execute_lua_function(ms.func_idx, callbacks, lua, pending);
				continue;
			}
			let _ = lua.globals().set("_current_btn", dir.clone());
			for b in &cfg.bindings {
				if b.button == *dir && b.event == BindingEvent::Press {
					execute_lua_function(b.func_idx, callbacks, lua, pending);
				}
			}
		}

		for dir in prev_dirs.iter() {
			if !current.contains(dir.as_str()) {
				log_msg(1, &format!("<< release {}", dir));
				let _ = lua.globals().set("_current_btn", dir.clone());
				if mapper.lock().unwrap().is_tap(dir, now) {
					for b in &cfg.bindings {
						if b.button == *dir && b.event == BindingEvent::Tap {
							execute_lua_function(b.func_idx, callbacks, lua, pending);
						}
					}
				}
				for b in &cfg.bindings {
					if b.button == *dir && b.event == BindingEvent::Release {
						execute_lua_function(b.func_idx, callbacks, lua, pending);
					}
				}
				for key in mapper.lock().unwrap().button_up(dir) {
					dev.apply(&key, false);
				}
			}
		}

		*prev_dirs = current;
	}
}
