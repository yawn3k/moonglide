local RAD_TO_DEG = 180 / math.pi

local gyro_state = {
	active = false,
	bias_x = 0, bias_y = 0,
	sens_h = 1, sens_v = 1,
	calibration = 45.454,
	in_game_sens = 1,
	accum_x = 0, accum_y = 0,
	last_time = nil,
	cal_samples = {},
	calibrating = false,
	hold_button = nil,
}

local function parse_sens(val)
	if type(val) == "table" then
		local h = val[1] or 1
		local v = val[2] or h
		return h, v
	end
	return val, val
end

function gyro(tbl)
	local s = tbl.sensitivity or tbl.gyro_sens or 1
	gyro_state.sens_h, gyro_state.sens_v = parse_sens(s)
	gyro_state.calibration = tbl.calibration or gyro_state.calibration
	gyro_state.in_game_sens = tbl.in_game_sens or gyro_state.in_game_sens
end

function gyro_enable()
	gyro_state.hold_button = nil
	gyro_state.active = true
	gyro_state.last_time = _now()
end

function gyro_disable()
	gyro_state.hold_button = nil
	gyro_state.active = false
	gyro_state.accum_x = 0
	gyro_state.accum_y = 0
end

function gyro_toggle()
	if gyro_state.active then gyro_disable() else gyro_enable() end
end

function gyro_hold()
	gyro_state.hold_button = _current_btn
	gyro_state.active = true
	gyro_state.last_time = _now()
end

function gyro_calibrate_start()
	gyro_state.cal_samples = {}
	gyro_state.calibrating = true
	log(1, "gyro calibration started — collecting samples")
end

function gyro_calibrate_stop()
	if not gyro_state.calibrating or #gyro_state.cal_samples == 0 then
		log(1, "gyro calibration: no samples collected (no gyro events received)")
		gyro_state.calibrating = false
		return
	end
	local n = #gyro_state.cal_samples
	local sum_x, sum_y = 0, 0
	for _, s in ipairs(gyro_state.cal_samples) do
		sum_x = sum_x + s.x
		sum_y = sum_y + s.y
	end
	gyro_state.bias_x = sum_x / n
	gyro_state.bias_y = sum_y / n
	gyro_state.cal_samples = {}
	gyro_state.calibrating = false
	log(1, string.format("gyro calibration complete (%d samples)", n))
end

function process_gyro(gx, gy, gz)
	if gyro_state.calibrating then
		gyro_state.cal_samples[#gyro_state.cal_samples + 1] = { x = gx, y = gy, z = gz }
		if #gyro_state.cal_samples % 100 == 0 then
			log(2, string.format("calibrating... %d samples collected", #gyro_state.cal_samples))
		end
		return {}
	end
	if not gyro_state.active then return {} end

	local now = _now()
	local dt = gyro_state.last_time and math.min(now - gyro_state.last_time, 0.1) or 0
	gyro_state.last_time = now

	local rx = gx - gyro_state.bias_x
	local ry = gy - gyro_state.bias_y

	local pitch_deg = rx * RAD_TO_DEG * dt
	local yaw_deg   = ry * RAD_TO_DEG * dt

	local dx = -yaw_deg * gyro_state.calibration * gyro_state.sens_h / gyro_state.in_game_sens
	local dy = -pitch_deg * gyro_state.calibration * gyro_state.sens_v / gyro_state.in_game_sens

	gyro_state.accum_x = gyro_state.accum_x + dx
	gyro_state.accum_y = gyro_state.accum_y + dy

	local out_x = math.floor(gyro_state.accum_x)
	local out_y = math.floor(gyro_state.accum_y)
	if out_x ~= 0 or out_y ~= 0 then
		gyro_state.accum_x = gyro_state.accum_x - out_x
		gyro_state.accum_y = gyro_state.accum_y - out_y
		return { dx = out_x, dy = out_y }
	end
	return {}
end

function gyro_reset()
	gyro_state.bias_x = 0
	gyro_state.bias_y = 0
	gyro_state.calibrating = false
	gyro_state.cal_samples = {}
	gyro_state.hold_button = nil
	gyro_state.active = false
	gyro_state.accum_x = 0
	gyro_state.accum_y = 0
end
