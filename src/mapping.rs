use std::collections::{HashMap, HashSet};
use std::time::Instant;

const TAP_THRESHOLD_MS: u128 = 180;
const TURBO_INTERVAL_MS: u128 = 100;

pub struct Mapper {
	held_buttons: HashMap<String, Instant>,
	press_held: HashMap<String, Vec<String>>,
	held_keys: HashSet<String>,
	toggled: HashSet<String>,
	action_queue: Vec<(String, bool)>,
	turbo_keys: HashMap<String, Instant>,
	turbo_event_timers: HashMap<String, Instant>,
	instant_release_timers: HashMap<String, (Instant, u64)>,
	pub last_press_times: HashMap<String, Instant>,
	consumed_presses: HashSet<String>,
}

impl Mapper {
	pub fn new() -> Self {
		Self {
			held_buttons: HashMap::new(),
			press_held: HashMap::new(),
			held_keys: HashSet::new(),
			toggled: HashSet::new(),
			action_queue: Vec::new(),
			turbo_keys: HashMap::new(),
			turbo_event_timers: HashMap::new(),
			instant_release_timers: HashMap::new(),
			last_press_times: HashMap::new(),
			consumed_presses: HashSet::new(),
		}
	}

	pub fn button_down(&mut self, btn: &str, now: Instant) {
		self.held_buttons.insert(btn.to_string(), now);
	}

	pub fn button_up(&mut self, btn: &str) -> Vec<String> {
		self.held_buttons.remove(btn);
		self.turbo_event_timers.remove(btn);
		let keys = self.press_held.remove(btn).unwrap_or_default();
		let mut released = Vec::new();
		for k in &keys {
			let still_needed = self.press_held
				.iter()
				.any(|(other_btn, ks)| self.held_buttons.contains_key(other_btn.as_str()) && ks.contains(k));
			if !still_needed {
				self.held_keys.remove(k);
				released.push(k.clone());
			}
		}
		released
	}

	pub fn held_buttons(&self) -> Vec<String> {
		self.held_buttons.keys().cloned().collect()
	}

	pub fn is_tap(&self, btn: &str, now: Instant) -> bool {
		self.held_buttons
			.get(btn)
			.map(|t| now.duration_since(*t).as_millis() < TAP_THRESHOLD_MS)
			.unwrap_or(false)
	}

	pub fn hold_elapsed(&self, btn: &str, delay_ms: u64, now: Instant) -> bool {
		self.held_buttons
			.get(btn)
			.map(|t| now.duration_since(*t).as_millis() >= delay_ms as u128)
			.unwrap_or(false)
	}

	pub fn mark_consumed(&mut self, btn: &str) {
		self.consumed_presses.insert(btn.to_string());
	}

	pub fn is_consumed(&self, btn: &str) -> bool {
		self.consumed_presses.contains(btn)
	}

	pub fn unmark_consumed(&mut self, btn: &str) {
		self.consumed_presses.remove(btn);
	}

	pub fn is_double_press(&mut self, btn: &str, now: Instant, window_ms: u64) -> bool {
		if let Some(last) = self.last_press_times.get(btn) {
			if now.duration_since(*last).as_millis() <= window_ms as u128 {
				return true;
			}
		}
		false
	}

	pub fn turbo_event_due(&mut self, btn: &str, now: Instant) -> bool {
		let due = self
			.turbo_event_timers
			.get(btn)
			.map(|t| now.duration_since(*t).as_millis() >= TURBO_INTERVAL_MS)
			.unwrap_or(true);
		if due {
			self.turbo_event_timers.insert(btn.to_string(), now);
		}
		due
	}

	/// Hold key while button is held
	pub fn press_key(&mut self, btn: &str, key: &str) {
		self.press_held
			.entry(btn.to_string())
			.or_default()
			.push(key.to_string());
		self.queue_press(key);
	}

	/// Tap key immediately (press + release after delay)
	pub fn instant_key(&mut self, key: &str, delay_override: Option<u64>) {
		self.queue_press(key);
		self.instant_release_timers.insert(key.to_string(), (Instant::now(), delay_override.unwrap_or(0)));
	}

	pub fn process_instant_releases(&mut self, now: Instant, default_delay_ms: u64) {
		let elapsed: Vec<String> = self.instant_release_timers
			.iter()
			.filter(|(_, (t, override_ms))| {
				let delay = if *override_ms > 0 { *override_ms } else { default_delay_ms };
				now.duration_since(*t).as_millis() >= delay as u128
			})
			.map(|(k, _)| k.clone())
			.collect();
		for key in &elapsed {
			self.queue_release(key);
			self.instant_release_timers.remove(key);
		}
	}

	/// Release key on button-up (don't press now)
	pub fn release_key(&mut self, btn: &str, key: &str) {
		self.press_held
			.entry(btn.to_string())
			.or_default()
			.push(key.to_string());
	}

	/// Toggle key on/off each call
	pub fn toggle_key(&mut self, key: &str) {
		if !self.toggled.insert(key.to_string()) {
			self.toggled.remove(key);
			self.queue_release(key);
		} else {
			self.queue_press(key);
		}
	}

	/// Register a key for turbo pulsing while button is held
	pub fn turbo_key(&mut self, btn: &str, key: &str, now: Instant) {
		self.press_held
			.entry(btn.to_string())
			.or_default()
			.push(key.to_string());
		self.turbo_keys.entry(key.to_string()).or_insert(now);
	}

	/// Process turbo pulses
	pub fn process_turbo(&mut self, now: Instant) {
		let due: Vec<String> = self.turbo_keys
			.iter()
			.filter(|(_, last)| now.duration_since(**last).as_millis() >= TURBO_INTERVAL_MS)
			.map(|(k, _)| k.clone())
			.collect();
		for key in &due {
			if self.held_keys.contains(key) {
				self.queue_release(key);
			} else {
				self.queue_press(key);
			}
			self.turbo_keys.insert(key.clone(), now);
		}
	}

	pub fn release_all(&mut self) {
		for key in self.held_keys.drain() {
			self.action_queue.push((key, false));
		}
		self.press_held.clear();
		self.turbo_keys.clear();
		self.held_buttons.clear();
		self.turbo_event_timers.clear();
		self.instant_release_timers.clear();
		self.consumed_presses.clear();
	}

	pub fn drain_actions(&mut self) -> Vec<(String, bool)> {
		std::mem::take(&mut self.action_queue)
	}

	fn queue_press(&mut self, key: &str) {
		if self.held_keys.insert(key.to_string()) {
			self.action_queue.push((key.to_string(), true));
		}
	}

	fn queue_release(&mut self, key: &str) {
		if self.held_keys.remove(key) {
			self.action_queue.push((key.to_string(), false));
		}
	}
}
