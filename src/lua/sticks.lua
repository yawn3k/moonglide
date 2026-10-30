-- stick processing
-- process_sticks is called from Rust every frame per controller
-- can be overridden by user config for custom stick handling

local MAX_AXIS = 32767

-- scalar → {h, v}; table → horizontal, vertical
local function parse_deadzone(val, default)
	if type(val) == "table" then
		local h = val[1] or default
		local v = val[2] or h
		return h, v
	end
	if val == nil then return default, default end
	return val, val
end

-- per-axis deadzone: each axis is independent, zeroed below its inner
-- threshold, then linearly rescaled over [inner, outer] → [0, 1]
local function apply_axis(v, inner, outer)
	local a = math.abs(v)
	if a <= inner then return 0 end
	if outer <= inner then return (v < 0 and -1 or 1) end
	local scaled = (a - inner) / (outer - inner)
	if scaled > 1 then scaled = 1 end
	return (v < 0 and -1 or 1) * scaled
end

-- JSM-style circular deadzone: zeroed inside the inner circle, then radially
-- rescaled over [inner, outer] → [0, 1], preserving the stick's angle
local function apply_circular(s, inner, outer)
	local len = (s.x * s.x + s.y * s.y) ^ 0.5
	if len == 0 then return end
	if len < inner then
		s.x = 0; s.y = 0
	elseif len > outer then
		local scale = outer / len
		s.x = s.x * scale; s.y = s.y * scale
	else
		local mapped = (len - inner) / (outer - inner)
		local scale = mapped / len
		s.x = s.x * scale; s.y = s.y * scale
	end
end

-- scalar → circular (JSM-style); {h, v} table → per-axis cross/square
local function apply_deadzone(s, val, default, outer)
	if type(val) == "table" then
		local h, v = parse_deadzone(val, default)
		s.x = apply_axis(s.x, h, outer)
		s.y = apply_axis(s.y, v, outer)
	else
		apply_circular(s, val == nil and default or val, outer)
	end
end

local function cross_gate(x, y, prefix, out, da)
	if x == 0 and y == 0 then return end
	local card = 45 - (da or 22.5)
	local angle = math.atan2(math.abs(y), math.abs(x)) * 180 / math.pi
	local horiz = angle <= 90 - card
	local vert = angle >= card
	if horiz and x < 0 then out[prefix .. "_left"] = true
	elseif horiz then out[prefix .. "_right"] = true end
	if vert and y > 0 then out[prefix .. "_up"] = true
	elseif vert then out[prefix .. "_down"] = true end
end

local stick_state = {}
local trigger_state = {}
local trigger_last_time = {}

-- stick processing config: global defaults with per-stick overrides.
-- `sticks = { ... }` (global rebind) deep-merges into `default_sticks`, so
-- partial tables are fine and per-stick overrides (`sticks.right.x = ...`)
-- survive rebinding.

local default_sticks = {
	mode = "movement",
	trigger_threshold = 3000,
	diagonal_angle = 22.5,
	inner_deadzone = 0.15,
	outer_deadzone = 1.0,
	ring_position = 0.8,
	left = {},
	right = {},
}

setmetatable(_G, {
	__index = function(_, k)
		if k == "sticks" then return default_sticks end
		return rawget(_G, k)
	end,
	__newindex = function(_, k, v)
		if k == "sticks" and type(v) == "table" then
			for kk, vv in pairs(v) do
				if type(vv) == "table" and type(default_sticks[kk]) == "table" then
					for kkk, vvv in pairs(vv) do default_sticks[kk][kkk] = vvv end
				else
					default_sticks[kk] = vv
				end
			end
		else
			rawset(_G, k, v)
		end
	end,
})

local DEFAULT_MODE = "movement"
local DEFAULT_TRIGGER_THRESHOLD = 3000
local DEFAULT_DIAGONAL_ANGLE = 22.5
local DEFAULT_INNER_DEADZONE = 0.15
local DEFAULT_OUTER_DEADZONE = 1.0
local DEFAULT_RING_POSITION = 0.8

-- resolve a stick setting: per-stick override -> global -> default
local function stick_cfg(which, field, default)
	local per = sticks[which]
	local v = per and per[field]
	if v ~= nil then return v end
	v = sticks[field]
	if v ~= nil then return v end
	return default
end

function process_sticks(which, lx, ly, rx, ry, lt, rt)
	local mode_l = stick_cfg("left", "mode", DEFAULT_MODE)
	local mode_r = stick_cfg("right", "mode", DEFAULT_MODE)
	if mode_l ~= "movement" or mode_r ~= "movement" then
		error(string.format("invalid stick mode: %s / %s", tostring(mode_l), tostring(mode_r)))
	end

	local inner_l = stick_cfg("left", "inner_deadzone", DEFAULT_INNER_DEADZONE)
	local outer_l = stick_cfg("left", "outer_deadzone", DEFAULT_OUTER_DEADZONE)
	local diag_l = stick_cfg("left", "diagonal_angle", DEFAULT_DIAGONAL_ANGLE)
	local ring_l = stick_cfg("left", "ring_position", DEFAULT_RING_POSITION)

	local inner_r = stick_cfg("right", "inner_deadzone", DEFAULT_INNER_DEADZONE)
	local outer_r = stick_cfg("right", "outer_deadzone", DEFAULT_OUTER_DEADZONE)
	local diag_r = stick_cfg("right", "diagonal_angle", DEFAULT_DIAGONAL_ANGLE)
	local ring_r = stick_cfg("right", "ring_position", DEFAULT_RING_POSITION)
	local trigger_threshold = sticks.trigger_threshold or DEFAULT_TRIGGER_THRESHOLD

	local nl = { x = lx / MAX_AXIS, y = ly / MAX_AXIS }
	local nr = { x = rx / MAX_AXIS, y = ry / MAX_AXIS }

	local lo = outer_l
	local ro = outer_r
	local lr = ring_l
	local rr = ring_r

	apply_deadzone(nl, inner_l, inner_l, lo)
	apply_deadzone(nr, inner_r, inner_r, ro)

	local current = {}
	cross_gate(nl.x, nl.y, "left_stick", current, diag_l)
	cross_gate(nr.x, nr.y, "right_stick", current, diag_r)

	local llen = ((lx / MAX_AXIS) ^ 2 + (ly / MAX_AXIS) ^ 2) ^ 0.5
	local rlen = ((rx / MAX_AXIS) ^ 2 + (ry / MAX_AXIS) ^ 2) ^ 0.5
	if nl.x == 0 and nl.y == 0 then llen = 0 end
	if nr.x == 0 and nr.y == 0 then rlen = 0 end
	if llen > 0 and llen < lr then current["left_ring_inner"] = true end
	if llen > lr then current["left_ring_outer"] = true end
	if rlen > 0 and rlen < rr then current["right_ring_inner"] = true end
	if rlen > rr then current["right_ring_outer"] = true end

	local prev = stick_state[which] or {}
	stick_state[which] = current

	local pressed, released = {}, {}
	for k, _ in pairs(current) do
		if not prev[k] then pressed[#pressed + 1] = k end
	end
	for k, _ in pairs(prev) do
		if not current[k] then released[#released + 1] = k end
	end

	-- trigger processing
	local thresh = trigger_threshold
	local t = trigger_state[which] or { lt = false, rt = false }
	local now = _now()
	local tl = trigger_last_time[which] or {}
	tl.lt = tl.lt or 0
	tl.rt = tl.rt or 0

	if lt > thresh and not t.lt and (now - tl.lt) > 0.05 then
		t.lt = true; tl.lt = now
		pressed[#pressed + 1] = "left_trigger"
	elseif lt <= thresh and t.lt and (now - tl.lt) > 0.05 then
		t.lt = false; tl.lt = now
		released[#released + 1] = "left_trigger"
	end
	if rt > thresh and not t.rt and (now - tl.rt) > 0.05 then
		t.rt = true; tl.rt = now
		pressed[#pressed + 1] = "right_trigger"
	elseif rt <= thresh and t.rt and (now - tl.rt) > 0.05 then
		t.rt = false; tl.rt = now
		released[#released + 1] = "right_trigger"
	end
	trigger_state[which] = t
	trigger_last_time[which] = tl

	return { pressed = pressed, released = released }
end

local prev_touchpad_zones = {}

function process_touchpad(id)
	local fingers = _touchpad_fingers
	if not fingers then return { pressed = {}, released = {} } end

	local left, right = false, false
	for _, f in pairs(fingers) do
		if f.x < 0.5 then left = true else right = true end
	end
	local any = left or right

	local prev = prev_touchpad_zones[id] or {}
	local pressed, released = {}, {}

	if any and not prev.any then pressed[#pressed + 1] = "touchpad_touch" end
	if not any and prev.any then released[#released + 1] = "touchpad_touch" end

	if left and not prev.left then pressed[#pressed + 1] = "touchpad_touch_left" end
	if not left and prev.left then released[#released + 1] = "touchpad_touch_left" end

	if right and not prev.right then pressed[#pressed + 1] = "touchpad_touch_right" end
	if not right and prev.right then released[#released + 1] = "touchpad_touch_right" end

	prev_touchpad_zones[id] = { any = any, left = left, right = right }
	return { pressed = pressed, released = released }
end

function cleanup_controller(which)
	stick_state[which] = nil
	trigger_state[which] = nil
	trigger_last_time[which] = nil
	prev_touchpad_zones[which] = nil
end
