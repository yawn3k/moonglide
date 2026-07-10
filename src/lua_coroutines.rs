use std::time::Instant;

use mlua::{Lua, RegistryKey};

pub struct PendingThread {
	pub key: RegistryKey,
	pub resume_at: Instant,
}

pub fn resume_thread(lua: &Lua, thread: mlua::Thread, pending: &mut Vec<PendingThread>, label: &str) {
	let values = match thread.resume::<mlua::MultiValue>(()) {
		Ok(v) => v,
		Err(e) => { println!("[dbg] {} resume error: {}", label, e); return; }
	};
	if thread.status() != mlua::ThreadStatus::Resumable { return; }
	let delay = values.get(0).and_then(|v| {
		v.as_f64().or_else(|| v.as_i64().map(|i| i as f64))
	});
	match delay {
		Some(d) => {
			println!("[dbg] {} re-yielding for {}s", label, d);
			if let Ok(key) = lua.create_registry_value(thread) {
				pending.push(PendingThread {
					key,
					resume_at: Instant::now() + std::time::Duration::from_secs_f64(d),
				});
			}
		}
		None => println!("[dbg] {} resumable but no delay", label),
	}
}

pub fn poll_pending_threads(pending: &mut Vec<PendingThread>, lua: &Lua) {
	let now = Instant::now();
	for i in (0..pending.len()).rev() {
		if now < pending[i].resume_at { continue; }
		let pt = pending.swap_remove(i);
		let due = pt.resume_at;
		match lua.registry_value::<mlua::Thread>(&pt.key) {
			Ok(thread) => {
				println!("[dbg] polling: resuming thread (was due at {:?})", due);
				resume_thread(lua, thread, pending, "re-scheduled");
			}
			Err(_) => println!("[dbg] polling: failed to retrieve thread from registry"),
		}
	}
}
