-- binding library
-- defines bind.* and user-facing helpers

local button_bindings = {}   -- btn -> { press=fn, release=fn, tap=fn, hold=fn, hold_delay=ms, turbo=fn }
local chords = {}            -- { buttons={...}, func=fn }
local double_presses = {}    -- { button=btn, func=fn, window_ms=ms }
local modeshifts = {}        -- { modifiers={...}, button=btn, func=fn }
local consumed = {}          -- { [btn]=true }
local press_times = {}       -- { [btn]=time }
local held_mapping = {}      -- { [btn]={ [key]=true } }
local last_press = {}        -- { [btn]=time }
local hold_timers = {}       -- { [btn]={ fired=bool, delay=ms } }
local instant_times = {}     -- { [key]={ at=time, delay=ms } }
local deferred_taps = {}     -- { [btn]={ func=fn, window=ms, press_at=time } }
local frame_keys = {}        -- keys pressed during update(), cleared each frame

local function ref_val(v)
	if type(v) == "table" and v.__kind == "ref" then
		return v.val
	end
	return v
end

local function extract_action(action)
	if action == nil then error("bind.*: action is nil", 2) end
	if type(action) == "function" then
		return action
	end
	local v = ref_val(action)
	return function() press(v) end
end

local function extract_instant_action(action)
	if action == nil then error("bind.*: action is nil", 2) end
	if type(action) == "function" then
		return action
	end
	local v = ref_val(action)
	return function() instant(v) end
end

-- ── bind.* registration ──

bind = {}

function bind.press(btn, action)
	btn = ref_val(btn)
	if btn == nil then error("bind.press: button resolves to nil — check your spelling", 2) end
	local e = button_bindings[btn] or {}
	e.press = extract_action(action)
	button_bindings[btn] = e
end

function bind.tap(btn, action)
	btn = ref_val(btn)
	if btn == nil then error("bind.tap: button resolves to nil — check your spelling", 2) end
	local e = button_bindings[btn] or {}
	e.tap = extract_instant_action(action)
	button_bindings[btn] = e
end

function bind.hold(btn, action, opts)
	btn = ref_val(btn)
	if btn == nil then error("bind.hold: button resolves to nil — check your spelling", 2) end
	local e = button_bindings[btn] or {}
	e.hold = extract_action(action)
	e.hold_delay = (opts and opts.delay) or (hold_press_time or 400)
	hold_timers[btn] = { fired = false, delay = e.hold_delay }
	button_bindings[btn] = e
end

function bind.release(btn, action)
	btn = ref_val(btn)
	if btn == nil then error("bind.release: button resolves to nil — check your spelling", 2) end
	local e = button_bindings[btn] or {}
	e.release = extract_instant_action(action)
	button_bindings[btn] = e
end

function bind.turbo(btn, action)
	btn = ref_val(btn)
	if btn == nil then error("bind.turbo: button resolves to nil — check your spelling", 2) end
	local e = button_bindings[btn] or {}
	e.turbo = extract_instant_action(action)
	e._last_turbo = 0
	button_bindings[btn] = e
end

function bind.chord(buttons, action)
	local names = {}
	for _, b in ipairs(buttons) do
		local r = ref_val(b)
		if r == nil then error("bind.chord: button resolves to nil in chord table", 2) end
		table.insert(names, r)
	end
	table.insert(chords, { buttons = names, func = extract_action(action) })
end

function bind.double_press(btn, action, opts)
	btn = ref_val(btn)
	if btn == nil then error("bind.double_press: button resolves to nil", 2) end
	local window = (opts and opts.window) or (double_press_window or 200)
	table.insert(double_presses, { button = btn, func = extract_instant_action(action), window_ms = window })
end

function bind.modeshift(modifiers, btn, action)
	local mods = {}
	for _, m in ipairs(modifiers) do
		local r = ref_val(m)
		if r == nil then error("bind.modeshift: modifier resolves to nil in modifiers table", 2) end
		table.insert(mods, r)
	end
	btn = ref_val(btn)
	if btn == nil then error("bind.modeshift: button resolves to nil", 2) end
	table.insert(modeshifts, { modifiers = mods, button = btn, func = extract_action(action) })
end

-- ── user-facing helpers ──

function press(key)
	local raw = ref_val(key)
	_press_key(raw)
	local btn = _current_btn
	if btn == "__frame__" then
		frame_keys[raw] = true
	elseif btn and btn ~= "" then
		local m = held_mapping[btn] or {}
		m[raw] = true
		held_mapping[btn] = m
	end
end

function release(key)
	local raw = ref_val(key)
	_release_key(raw)
	local btn = _current_btn
	if btn and held_mapping[btn] then
		held_mapping[btn][raw] = nil
	end
end

function instant(key, opts)
	local raw = ref_val(key)
	_press_key(raw)
	local delay = (opts and opts.press_time) or (instant_press_time or 40)
	instant_times[raw] = { at = _now(), delay = delay }
end

function toggle(key)
	local raw = ref_val(key)
	if _toggled[raw] then
		_toggled[raw] = nil
		_release_key(raw)
	else
		_toggled[raw] = true
		_press_key(raw)
	end
end

function turbo(key)
	local raw = ref_val(key)
	local btn = _current_btn
	if btn and btn ~= "" then
		local e = button_bindings[btn] or {}
		e.turbo = function() instant(raw) end
		e._last_turbo = 0
		button_bindings[btn] = e
	end
end

function held(btn)
	return _is_held(ref_val(btn))
end

function wait(seconds)
	local saved = _current_btn
	coroutine.yield(seconds)
	_current_btn = saved
end

-- toggled state
_toggled = {}
