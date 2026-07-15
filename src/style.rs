use std::io::IsTerminal;
use std::sync::atomic::AtomicBool;

static USE_COLOR: AtomicBool = AtomicBool::new(true);

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
