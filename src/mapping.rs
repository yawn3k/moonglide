use std::collections::{HashMap, HashSet};
use std::time::Instant;

pub struct Mapper {
	held_buttons: HashMap<String, Instant>,
	held_keys: HashSet<String>,
	action_queue: Vec<(String, bool)>,
}

impl Mapper {
	pub fn new() -> Self {
		Self {
			held_buttons: HashMap::new(),
			held_keys: HashSet::new(),
			action_queue: Vec::new(),
		}
	}

	pub fn button_down(&mut self, btn: &str, now: Instant) {
		self.held_buttons.insert(btn.to_string(), now);
	}

	pub fn button_up(&mut self, btn: &str) {
		self.held_buttons.remove(btn);
	}

	pub fn is_held(&self, btn: &str) -> bool {
		self.held_buttons.contains_key(btn)
	}

	pub fn held_buttons(&self) -> Vec<String> {
		self.held_buttons.keys().cloned().collect()
	}

	pub fn press_key(&mut self, key: &str) {
		if self.held_keys.insert(key.to_string()) {
			self.action_queue.push((key.to_string(), true));
		}
	}

	pub fn release_key(&mut self, key: &str) {
		if self.held_keys.remove(key) {
			self.action_queue.push((key.to_string(), false));
		}
	}

	pub fn drain_actions(&mut self) -> Vec<(String, bool)> {
		std::mem::take(&mut self.action_queue)
	}

	pub fn release_all(&mut self) {
		for key in self.held_keys.drain() {
			self.action_queue.push((key, false));
		}
		self.held_buttons.clear();
	}
}
