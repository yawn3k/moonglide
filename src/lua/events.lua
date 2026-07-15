-- event handlers called from Rust
-- defines on_btn_down, on_btn_up, on_update

-- ── chord check ──
local function check_chord(btn, all_held)
	for _, chord in ipairs(chords) do
		local is_in_chord = false
		for _, b in ipairs(chord.buttons) do
			if b == btn then is_in_chord = true; break end
		end
		if not is_in_chord then goto continue end

		local ok = true
		for _, b in ipairs(chord.buttons) do
			local found = false
			for _, h in ipairs(all_held) do
				if h == b then found = true; break end
			end
			if not found then ok = false; break end
		end
		if ok then
			_current_btn = btn
			chord.func()
			for _, b in ipairs(chord.buttons) do
				consumed[b] = true
			end
			return true
		end
		::continue::
	end
	return false
end

-- ── double-press check ──
local function check_double_press(btn)
	local lt = last_press[btn]
	last_press[btn] = _now()
	if lt then
		for _, dp in ipairs(double_presses) do
			if dp.button == btn and (_now() - lt) * 1000 <= dp.window_ms then
				_current_btn = btn
				dp.func()
				consumed[btn] = true
				return true
			end
		end
	end
	return false
end

-- ── modeshift check ──
local function check_modeshift(btn)
	for _, ms in ipairs(modeshifts) do
		if ms.button == btn then
			local all_mods = true
			for _, m in ipairs(ms.modifiers) do
				if not _is_held(m) then all_mods = false; break end
			end
			if all_mods then
				_current_btn = btn
				ms.func()
				consumed[btn] = true
				for _, m in ipairs(ms.modifiers) do
					consumed[m] = true
				end
				return true
			end
		end
	end
	return false
end

-- ── normal press ──
local function handle_normal_press(btn)
	local e = button_bindings[btn]
	if e and e.press then
		_current_btn = btn
		e.press()
	end
end

-- ── retroactive modeshift ──
local function check_retroactive_modeshift(btn)
	for held_btn, _ in pairs(press_times) do
		if held_btn ~= btn and _is_held(held_btn) then
			for _, ms in ipairs(modeshifts) do
				if ms.button == held_btn then
					local is_mod = false
					for _, m in ipairs(ms.modifiers) do
						if m == btn then is_mod = true; break end
					end
					if is_mod then
						local all_mods = true
						for _, m in ipairs(ms.modifiers) do
							if not _is_held(m) then all_mods = false; break end
						end
						if all_mods then
							consumed[held_btn] = true
							for _, m in ipairs(ms.modifiers) do
								consumed[m] = true
							end
						end
					end
				end
			end
		end
	end
end

-- ── release keys for a button (shared by consumed + normal paths) ──
local function release_keys_for_button(btn, clear_held)
	local map = held_mapping[btn]
	if not map then return end
	for k, _ in pairs(map) do
		local still = false
		for ob, om in pairs(held_mapping) do
			if ob ~= btn and _is_held(ob) and om[k] then still = true; break end
		end
		if not still then
			_release_key(k)
		end
	end
	if clear_held then
		held_mapping[btn] = nil
	end
end

-- ── consumed key release ──
local function release_consumed_keys(btn)
	consumed[btn] = nil
	release_keys_for_button(btn, true)
	hold_timers[btn] = nil
end

-- ── on_btn_down ──
function on_btn_down(btn)
	press_times[btn] = _now()

	-- reset hold timer for this press
	local e = button_bindings[btn]
	if e and e.hold then
		hold_timers[btn] = { fired = false, delay = e.hold_delay or (hold_press_time or 400) }
	end

	local all_held = _held_buttons()
	for _, h in ipairs(all_held) do
		if h == btn then goto found end
	end
	table.insert(all_held, btn)
	::found::

	if check_chord(btn, all_held) then return end
	if check_double_press(btn) then return end
	if check_modeshift(btn) then return end
	handle_normal_press(btn)
	check_retroactive_modeshift(btn)
end

-- ── on_btn_up ──
function on_btn_up(btn)
	_current_btn = btn

	if consumed[btn] then
		release_consumed_keys(btn)
		return
	end

	-- tap check
	local pt = press_times[btn]
	local e = button_bindings[btn]
	local tap_window = (e and e.hold_delay) or 180
	if pt and (_now() - pt) * 1000 < tap_window then
		if e and e.tap then
			local has_dp = false
			for _, dp in ipairs(double_presses) do
				if dp.button == btn then has_dp = true; break end
			end
			if has_dp then
				local window = 200
				for _, dp in ipairs(double_presses) do
					if dp.button == btn then window = dp.window_ms; break end
				end
				deferred_taps[btn] = { func = e.tap, window = window, press_at = _now() }
			else
				_current_btn = btn
				e.tap()
			end
		end
	end

	-- release binding
	if e and e.release then
		_current_btn = btn
		e.release()
	end

	release_keys_for_button(btn, true)
	hold_timers[btn] = nil
	press_times[btn] = nil
end

-- ── on_update ──
function on_update()
	-- clear frame-scoped keys from previous frame
	for k, _ in pairs(frame_keys) do
		_release_key(k)
	end
	frame_keys = {}

	-- hold timers
	for btn, t in pairs(hold_timers) do
		if _is_held(btn) and not t.fired then
			local pt = press_times[btn]
			if pt and (_now() - pt) * 1000 >= t.delay then
				t.fired = true
				local e = button_bindings[btn]
				if e and e.hold then
					_current_btn = btn
					e.hold()
				end
			end
		end
	end

	-- instant release timers
	for k, t in pairs(instant_times) do
		if (_now() - t.at) * 1000 >= t.delay then
			_release_key(k)
			instant_times[k] = nil
		end
	end

	-- deferred tap (double-press window)
	for btn, d in pairs(deferred_taps) do
		if (_now() - d.press_at) * 1000 >= d.window then
			deferred_taps[btn] = nil
		end
	end

	-- user-defined update
	if type(update) == "function" then
		_current_btn = "__frame__"
		update()
		_current_btn = ""
	end

	-- clean up orphaned consumed buttons
	for btn, _ in pairs(consumed) do
		if not _is_held(btn) then
			release_consumed_keys(btn)
		end
	end
end

-- called from Rust on reset()/reload()
function _reset_internals()
	log_level = 0
	instant_press_time = 40
	hold_press_time = 400
	double_press_window = 200
	trigger_threshold = 3000
	left_stick_inner_deadzone = 0.15
	left_stick_outer_deadzone = 1.0
	right_stick_inner_deadzone = 0.15
	right_stick_outer_deadzone = 1.0
	left_ring_position = 0.8
	right_ring_position = 0.8
	update = nil
	_toggled = {}
	gyro_reset()
	for _, name in ipairs({"_gyro_raw", "_accel_raw", "_gravity"}) do
		local t = _G[name]
		if t then t.x, t.y, t.z = 0, 0, 0 end
	end
	local o = _orientation
	if o then o.w, o.x, o.y, o.z = 1, 0, 0, 0 end
end
