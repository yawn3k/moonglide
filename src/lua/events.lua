-- event handlers called from Rust
-- defines on_btn_down, on_btn_up, on_update

function on_btn_down(btn)
	press_times[btn] = _now()

	-- chord check
	local all_held = _held_buttons()
	for _, h in ipairs(all_held) do
		if h == btn then goto found end
	end
	table.insert(all_held, btn)
	::found::

	for _, chord in ipairs(chords) do
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
			return
		end
	end

	-- double-press check
	local lt = last_press[btn]
	last_press[btn] = _now()
	if lt then
		for _, dp in ipairs(double_presses) do
			if dp.button == btn and (_now() - lt) * 1000 <= dp.window_ms then
				_current_btn = btn
				dp.func()
				consumed[btn] = true
				return
			end
		end
	end

	-- modeshift check
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
				return
			end
		end
	end

	-- normal press
	local e = button_bindings[btn]
	if e and e.press then
		_current_btn = btn
		e.press()
	end

	-- retroactive modeshift: this btn might be a mod for a held button
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

local function release_consumed_keys(btn)
	consumed[btn] = nil
	local map = held_mapping[btn]
	if map then
		for k, _ in pairs(map) do
			local still = false
			for ob, om in pairs(held_mapping) do
				if ob ~= btn and _is_held(ob) and om[k] then still = true; break end
			end
			if not still then
				_release_key(k)
			end
		end
		held_mapping[btn] = nil
	end
	hold_timers[btn] = nil
end

function on_btn_up(btn)
	_current_btn = btn

	if consumed[btn] then
		release_consumed_keys(btn)
		return
	end

	-- tap check
	local pt = press_times[btn]
	local e = button_bindings[btn]
	if pt and (_now() - pt) * 1000 < 180 then
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

	-- release mapped keys
	local map = held_mapping[btn]
	if map then
		for k, _ in pairs(map) do
			local still = false
			for ob, om in pairs(held_mapping) do
				if ob ~= btn and _is_held(ob) and om[k] then still = true; break end
			end
			if not still then
				_release_key(k)
			end
		end
		held_mapping[btn] = nil
	end

	hold_timers[btn] = nil
	press_times[btn] = nil
end

function on_update()
	-- gyro hold check
	if gyro_state.hold_button and not _is_held(gyro_state.hold_button) then
		gyro_state.active = false
		gyro_state.hold_button = nil
	end

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

	-- turbo
	for btn, e in pairs(button_bindings) do
		if e.turbo and _is_held(btn) then
			local last = e._last_turbo or 0
			if _now() - last >= 0.1 then
				e._last_turbo = _now()
				_current_btn = btn
				e.turbo()
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

	-- clean up orphaned consumed buttons (coroutine finished after button released)
	for btn, _ in pairs(consumed) do
		if not _is_held(btn) then
			release_consumed_keys(btn)
		end
	end
end
