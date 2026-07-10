-- stick processing
-- process_sticks is called from Rust every frame per controller
-- can be overridden by user config for custom stick handling

local MAX_AXIS = 32767

local function apply_deadzone(s, inner, outer)
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

local function cross_gate(x, y, prefix, out)
	if x == 0 and y == 0 then return end
	local angle = math.atan2(y, x) * 180 / math.pi
	if angle >= -22.5 and angle < 22.5 then
		out[prefix .. "_right"] = true
	elseif angle >= 22.5 and angle < 67.5 then
		out[prefix .. "_up"] = true
		out[prefix .. "_right"] = true
	elseif angle >= 67.5 and angle < 112.5 then
		out[prefix .. "_up"] = true
	elseif angle >= 112.5 and angle < 157.5 then
		out[prefix .. "_up"] = true
		out[prefix .. "_left"] = true
	elseif angle >= 157.5 or angle < -157.5 then
		out[prefix .. "_left"] = true
	elseif angle >= -157.5 and angle < -112.5 then
		out[prefix .. "_down"] = true
		out[prefix .. "_left"] = true
	elseif angle >= -112.5 and angle < -67.5 then
		out[prefix .. "_down"] = true
	else
		out[prefix .. "_down"] = true
		out[prefix .. "_right"] = true
	end
end

local stick_state = {}
local trigger_state = {}
local trigger_last_time = {}

function process_sticks(which, lx, ly, rx, ry, lt, rt)
	local nl = { x = lx / MAX_AXIS, y = ly / MAX_AXIS }
	local nr = { x = rx / MAX_AXIS, y = ry / MAX_AXIS }

	local li = left_stick_inner_deadzone or 0.15
	local lo = left_stick_outer_deadzone or 1.0
	local ri = right_stick_inner_deadzone or 0.15
	local ro = right_stick_outer_deadzone or 1.0
	local lr = left_ring_position or 0.8
	local rr = right_ring_position or 0.8

	apply_deadzone(nl, li, lo)
	apply_deadzone(nr, ri, ro)

	local current = {}
	cross_gate(nl.x, nl.y, "left_stick", current)
	cross_gate(nr.x, nr.y, "right_stick", current)

	local llen = (nl.x * nl.x + nl.y * nl.y) ^ 0.5
	local rlen = (nr.x * nr.x + nr.y * nr.y) ^ 0.5
	if llen > 0 and llen < lr then current["left_ring_inner"] = true end
	if llen > lr then current["left_ring_outer"] = true end
	if rlen > 0 and rlen < rr then current["right_ring_inner"] = true end
	if rlen > rr then current["right_ring_outer"] = true end

	local prev = stick_state[which] or {}
	stick_state[which] = current

	local pressed = {}
	local released = {}
	for k, _ in pairs(current) do
		if not prev[k] then pressed[#pressed + 1] = k end
	end
	for k, _ in pairs(prev) do
		if not current[k] then released[#released + 1] = k end
	end

	-- trigger processing
	local thresh = trigger_threshold or 3000
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
