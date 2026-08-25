use std::collections::HashMap;
use std::io::IsTerminal;
use std::sync::atomic::AtomicBool;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

static USE_COLOR: AtomicBool = AtomicBool::new(true);

static LAST_ERR: LazyLock<Mutex<HashMap<String, Instant>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Print an error to stderr, suppressing repeats of the same key within 5 seconds.
/// Safe for hot loops that would otherwise spam 1000 lines per second.
pub fn err_once(key: &str, msg: &str) {
	let now = Instant::now();
	let mut last = LAST_ERR.lock().unwrap();
	match last.get(key) {
		Some(t) if now.duration_since(*t) < Duration::from_secs(5) => {}
		_ => {
			last.insert(key.to_string(), now);
			drop(last);
			eprintln!("{}", err(msg));
		}
	}
}

pub fn init() {
	let tty = std::io::stdout().is_terminal();
	USE_COLOR.store(tty, std::sync::atomic::Ordering::Relaxed);
}

fn color(s: &str, code: &str) -> String {
	if USE_COLOR.load(std::sync::atomic::Ordering::Relaxed) {
		format!("{}{}{}", code, s, "\x1b[0m")
	} else {
		s.to_string()
	}
}

pub fn bold(s: &str) -> String { color(s, "\x1b[1m") }
pub fn dim(s: &str) -> String { color(s, "\x1b[2m") }
pub fn green(s: &str) -> String { color(s, "\x1b[32m") }
pub fn yellow(s: &str) -> String { color(s, "\x1b[33m") }
pub fn info(s: &str) -> String { color(s, "\x1b[1m\x1b[32m") }
pub fn warn(s: &str) -> String { color(s, "\x1b[1m\x1b[33m") }
pub fn err(s: &str) -> String { color(s, "\x1b[1m\x1b[31m") }
pub fn progress(s: &str) -> String { dim(s) }
