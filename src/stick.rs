use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU16, Ordering};

pub(crate) static LEFT_STICK_INNER: AtomicU16 = AtomicU16::new(0);
pub(crate) static LEFT_STICK_OUTER: AtomicU16 = AtomicU16::new(u16::MAX);
pub(crate) static RIGHT_STICK_INNER: AtomicU16 = AtomicU16::new(0);
pub(crate) static RIGHT_STICK_OUTER: AtomicU16 = AtomicU16::new(u16::MAX);
pub(crate) static LEFT_RING_POSITION: AtomicU16 = AtomicU16::new(0);
pub(crate) static RIGHT_RING_POSITION: AtomicU16 = AtomicU16::new(0);

pub const MAX_AXIS: f64 = 32767.0;

fn process_deadzones(x: &mut f64, y: &mut f64, inner: f64, outer: f64) {
	let len = (*x * *x + *y * *y).sqrt();
	if len == 0.0 { return; }
	if len < inner {
		*x = 0.0;
		*y = 0.0;
	} else if len > outer {
		let scale = outer / len;
		*x *= scale;
		*y *= scale;
	} else {
		let mapped = (len - inner) / (outer - inner);
		let scale = mapped / len;
		*x *= scale;
		*y *= scale;
	}
}

fn cross_gate(x: f64, y: f64, prefix: &str, out: &mut HashSet<String>) {
	if x == 0.0 && y == 0.0 { return; }
	let angle = y.atan2(x).to_degrees();
	let (c1, c2) = if angle >= -22.5 && angle < 22.5 {
		("right", "")
	} else if angle >= 22.5 && angle < 67.5 {
		("up", "right")
	} else if angle >= 67.5 && angle < 112.5 {
		("up", "")
	} else if angle >= 112.5 && angle < 157.5 {
		("up", "left")
	} else if angle >= 157.5 || angle < -157.5 {
		("left", "")
	} else if angle >= -157.5 && angle < -112.5 {
		("down", "left")
	} else if angle >= -112.5 && angle < -67.5 {
		("down", "")
	} else {
		("down", "right")
	};
	out.insert(format!("{}_{}", prefix, c1));
	if !c2.is_empty() {
		out.insert(format!("{}_{}", prefix, c2));
	}
}

pub fn process_stick_buttons(
	axis_state: &HashMap<u32, [i16; 6]>,
	prev: &mut HashMap<u32, HashSet<String>>,
) -> (Vec<String>, Vec<String>) {
	let inner_l = LEFT_STICK_INNER.load(Ordering::Relaxed) as f64;
	let outer_l = LEFT_STICK_OUTER.load(Ordering::Relaxed) as f64;
	let inner_r = RIGHT_STICK_INNER.load(Ordering::Relaxed) as f64;
	let outer_r = RIGHT_STICK_OUTER.load(Ordering::Relaxed) as f64;
	let ring_pos_l = LEFT_RING_POSITION.load(Ordering::Relaxed) as f64 / MAX_AXIS;
	let ring_pos_r = RIGHT_RING_POSITION.load(Ordering::Relaxed) as f64 / MAX_AXIS;

	let mut new_dirs = Vec::new();
	let mut removed_dirs = Vec::new();

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
		cross_gate(lx, ly, "left_stick", &mut current);
		cross_gate(rx, ry, "right_stick", &mut current);

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

		for dir in current.iter() {
			if !prev_dirs.contains(dir.as_str()) {
				new_dirs.push(dir.clone());
			}
		}

		for dir in prev_dirs.iter() {
			if !current.contains(dir.as_str()) {
				removed_dirs.push(dir.clone());
			}
		}

		*prev_dirs = current;
	}

	(new_dirs, removed_dirs)
}
