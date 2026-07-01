use std::sync::Arc;
use std::time::Instant;

use mlua::{Lua, RegistryKey};

use crate::log_msg;

pub struct PendingThread {
	pub key: RegistryKey,
	pub resume_at: Instant,
}

fn resume_thread(lua: &Lua, thread: mlua::Thread, pending: &mut Vec<PendingThread>, label: &str) {
	let values = match thread.resume::<mlua::MultiValue>(()) {
		Ok(v) => v,
		Err(e) => { log_msg(2, &format!("[coro] {} resume error: {}", label, e)); return; }
	};
	log_msg(2, &format!("[coro] {} resume Ok, status={:?}, nvalues={}", label, thread.status(), values.len()));
	if thread.status() != mlua::ThreadStatus::Resumable { return; }
	let delay = values.get(0).and_then(|v| {
		v.as_f64().or_else(|| v.as_i64().map(|i| i as f64))
	});
	match delay {
		Some(d) => {
			log_msg(2, &format!("[coro] {} resuming in {}s", label, d));
			if let Ok(key) = lua.create_registry_value(thread) {
				pending.push(PendingThread {
					key,
					resume_at: Instant::now() + std::time::Duration::from_secs_f64(d),
				});
			}
		}
		None => log_msg(2, &format!("[coro] {} resumable but no delay value (got {:?})", label, values.get(0))),
	}
}

pub fn execute_lua_function(
	func_idx: usize,
	callbacks: &[Arc<RegistryKey>],
	lua: &Lua,
	pending: &mut Vec<PendingThread>,
) {
	let key = match callbacks.get(func_idx) {
		Some(k) => k.as_ref(),
		None => { log_msg(2, &format!("[coro] no callback at idx {}", func_idx)); return; }
	};
	let f: mlua::Function = match lua.registry_value(key) {
		Ok(f) => f,
		Err(e) => { log_msg(2, &format!("[coro] registry_value: {}", e)); return; }
	};
	match lua.create_thread(f) {
		Ok(thread) => resume_thread(lua, thread, pending, "initial"),
		Err(e) => log_msg(2, &format!("[coro] create_thread: {}", e)),
	}
}

pub fn poll_pending_threads(pending: &mut Vec<PendingThread>, lua: &Lua) {
	let now = Instant::now();
	for i in (0..pending.len()).rev() {
		if now < pending[i].resume_at { continue; }
		let pt = pending.swap_remove(i);
		match lua.registry_value::<mlua::Thread>(&pt.key) {
			Ok(thread) => resume_thread(lua, thread, pending, "re-scheduled"),
			Err(_) => log_msg(2, &format!("[coro] failed to retrieve thread from registry")),
		}
	}
}
