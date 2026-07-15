-- gyro processing: sensor fusion, calibration, gyro spaces
-- called from Rust via on_sensor_event and process_gyro

local RAD_TO_DEG = 180 / math.pi

-- fusion constants
local HALF_TIME = 0.25
local STILL_THRESH = 0.01
local SHAKY_THRESH = 0.4
local WORLD_SIDE_REDUCTION = 0.125

-- quaternion math (internal, no table allocs)
local function qmul(w1, x1, y1, z1, w2, x2, y2, z2)
	return w1*w2 - x1*x2 - y1*y2 - z1*z2,
	       w1*x2 + x1*w2 + y1*z2 - z1*y2,
	       w1*y2 - x1*z2 + y1*w2 + z1*x2,
	       w1*z2 + x1*y2 - y1*x2 + z1*w2
end

local function qaxis_angle(angle, x, y, z)
	local ha = angle * 0.5
	local s = math.sin(ha)
	local len = math.sqrt(x*x + y*y + z*z)
	if len > 0 then s = s / len end
	return math.cos(ha), x*s, y*s, z*s
end

local function qrot_vec(vx, vy, vz, qw, qx, qy, qz)
	local vw, vx2, vy2, vz2 = qmul(qw, qx, qy, qz, 0, vx, vy, vz)
	local _, rx, ry, rz = qmul(vw, vx2, vy2, vz2, qw, -qx, -qy, -qz)
	return rx, ry, rz
end

-- vector math (internal)
local function vlen(x, y, z)
	return math.sqrt(x*x + y*y + z*z)
end

local function vnorm(x, y, z)
	local len = vlen(x, y, z)
	if len > 0 then return x/len, y/len, z/len end
	return x, y, z
end

local function vdot(x1, y1, z1, x2, y2, z2)
	return x1*x2 + y1*y2 + z1*z2
end

local function vcross(x1, y1, z1, x2, y2, z2)
	return y1*z2 - z1*y2, z1*x2 - x1*z2, x1*y2 - y1*x2
end

-- gyro state
local gyro_state = {
	active = false,
	bias_x = 0, bias_y = 0, bias_z = 0,
	sens_h = 1, sens_v = 1,
	calibration = 45.454,
	in_game_sens = 1,
	accum_x = 0, accum_y = 0,
	cal_samples = {},
	calibrating = false,
	hold_button = nil,

	-- sensor fusion state
	quat = { w = 1, x = 0, y = 0, z = 0 },
	grav = { x = 0, y = 0, z = 0 },
	shakiness = 0,
	smooth_accel = { x = 0, y = 0, z = 0 },
	sensor_last_time = nil,

	-- calibrated gyro (set by on_sensor_event, read by process_gyro)
	cal_gx = 0, cal_gy = 0, cal_gz = 0,

	-- flag for gravity auto-init
	grav_initialized = false,

	-- gyro deadzone in deg/s
	deadzone = 0,

	-- gyro space
	space = "local_yaw",

	-- acceleration curve function (takes speed_dps, returns multiplier)
	accel_fn = nil,
}

-- init gravity/orientation globals from module load
_gravity.x, _gravity.y, _gravity.z = 0, 0, 0
_orientation.w, _orientation.x, _orientation.y, _orientation.z = 1, 0, 0, 0

local function parse_sens(val)
	if type(val) == "table" then
		local h = val[1] or 1
		local v = val[2] or h
		return h, v
	end
	return val, val
end

function gyro(tbl)
	local s = tbl.sensitivity or tbl.gyro_sens
	if s then
		gyro_state.sens_h, gyro_state.sens_v = parse_sens(s)
	end
	gyro_state.calibration = tbl.calibration or gyro_state.calibration
	gyro_state.in_game_sens = tbl.in_game_sens or gyro_state.in_game_sens
	gyro_state.deadzone = tbl.deadzone or gyro_state.deadzone
	if tbl.space == "local_yaw" or tbl.space == "local_roll" or tbl.space == "player" or tbl.space == "world" then
		gyro_state.space = tbl.space
	end
	if type(tbl.acceleration) == "function" then
		gyro_state.accel_fn = tbl.acceleration
	else
		gyro_state.accel_fn = nil
	end
end

function gyro_get_bias()
	return gyro_state.bias_x, gyro_state.bias_y, gyro_state.bias_z
end

function gyro_get_state()
	return gyro_state.active, gyro_state.space, gyro_state.deadzone
end

function gyro_enable()
	gyro_state.hold_button = nil
	gyro_state.active = true
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
end

function gyro_calibrate_start()
	gyro_state.cal_samples = {}
	gyro_state.calibrating = true
	_info("gyro calibration started — collecting samples")
end

function gyro_calibrate_stop()
	if not gyro_state.calibrating or #gyro_state.cal_samples == 0 then
		log(1, "gyro calibration: no samples collected (no gyro events received)")
		gyro_state.calibrating = false
		return
	end
	local n = #gyro_state.cal_samples
	local sum_x, sum_y, sum_z = 0, 0, 0
	for _, s in ipairs(gyro_state.cal_samples) do
		sum_x = sum_x + s.x
		sum_y = sum_y + s.y
		sum_z = sum_z + s.z
	end
	gyro_state.bias_x = sum_x / n
	gyro_state.bias_y = sum_y / n
	gyro_state.bias_z = sum_z / n
	gyro_state.cal_samples = {}
	gyro_state.calibrating = false
	_info(string.format("gyro calibration complete (%d samples)", n))
end

-- ── fusion helpers ──

local function apply_gyro_rotation(cgx, cgy, cgz, dt)
	local q = gyro_state.quat
	local angle_speed = vlen(cgx, cgy, cgz) * math.pi / 180
	local angle = angle_speed * dt
	local rot_w, rot_x, rot_y, rot_z
	if angle > 0 then
		local rx, ry, rz = cgx / angle_speed, cgy / angle_speed, cgz / angle_speed
		rot_w, rot_x, rot_y, rot_z = qaxis_angle(angle, rx, ry, rz)
		q.w, q.x, q.y, q.z = qmul(q.w, q.x, q.y, q.z, rot_w, rot_x, rot_y, rot_z)
	else
		rot_w, rot_x, rot_y, rot_z = 1, 0, 0, 0
	end
	return angle_speed, rot_w, rot_x, rot_y, rot_z
end

local function smooth_accel_and_shakiness(ax, ay, az, ri_w, ri_x, ri_y, ri_z, dt)
	local sax, say, saz = qrot_vec(gyro_state.smooth_accel.x, gyro_state.smooth_accel.y, gyro_state.smooth_accel.z, ri_w, ri_x, ri_y, ri_z)
	local sf = HALF_TIME > 0 and 2^(-dt / HALF_TIME) or 0
	gyro_state.shakiness = gyro_state.shakiness * sf
	gyro_state.shakiness = math.max(gyro_state.shakiness, vlen(ax - sax, ay - say, az - saz))
	gyro_state.smooth_accel.x = sax + (ax - sax) * (1 - sf)
	gyro_state.smooth_accel.y = say + (ay - say) * (1 - sf)
	gyro_state.smooth_accel.z = saz + (az - saz) * (1 - sf)
end

local function correct_gravity(ax, ay, az, anx, any, anz, accel_mag, ri_w, ri_x, ri_y, ri_z, angle_speed, dt)
	local gv = gyro_state.grav
	gv.x, gv.y, gv.z = qrot_vec(gv.x, gv.y, gv.z, ri_w, ri_x, ri_y, ri_z)
	local target_x, target_y, target_z = -anx * accel_mag, -any * accel_mag, -anz * accel_mag
	local gta_x, gta_y, gta_z = target_x - gv.x, target_y - gv.y, target_z - gv.z
	local gta_len = vlen(gta_x, gta_y, gta_z)
	if gta_len > 0 then
		local gdx, gdy, gdz = gta_x / gta_len, gta_y / gta_len, gta_z / gta_len
		local corr_speed
		if gyro_state.shakiness < STILL_THRESH then
			corr_speed = 1.0
		elseif gyro_state.shakiness > SHAKY_THRESH then
			corr_speed = 0.1
		else
			local t = (gyro_state.shakiness - STILL_THRESH) / (SHAKY_THRESH - STILL_THRESH)
			corr_speed = 1.0 + (0.1 - 1.0) * t
		end
		corr_speed = math.min(corr_speed, math.max(angle_speed * 0.1, 0.01))
		local delta = corr_speed * dt
		if delta < gta_len then
			gv.x = gv.x + gdx * delta
			gv.y = gv.y + gdy * delta
			gv.z = gv.z + gdz * delta
		else
			gv.x, gv.y, gv.z = target_x, target_y, target_z
		end
	end
end

local function correct_quaternion_tilt()
	local q = gyro_state.quat
	local gv = gyro_state.grav
	do
		local len = math.sqrt(q.w*q.w + q.x*q.x + q.y*q.y + q.z*q.z)
		local gw, gqx, gqy, gqz = q.w/len, q.x/len, q.y/len, q.z/len
		local gdir_x, gdir_y, gdir_z = qrot_vec(gv.x, gv.y, gv.z, gw, gqx, gqy, gqz)
		local _, gux, guy, guz = vnorm(gdir_x, gdir_y, gdir_z)
		local err_angle = math.acos(math.max(-1, math.min(1, vdot(0, -1, 0, gux, guy, guz))))
		local flat_x, flat_y, flat_z = vcross(gdir_x, gdir_y, gdir_z, 0, -1, 0)
		local flat_len = vlen(flat_x, flat_y, flat_z)
		if flat_len > 1e-6 and err_angle > 1e-6 then
			local fix_w, fix_x, fix_y, fix_z = qaxis_angle(err_angle, flat_x / flat_len, flat_y / flat_len, flat_z / flat_len)
			q.w, q.x, q.y, q.z = qmul(fix_w, fix_x, fix_y, fix_z, q.w, q.x, q.y, q.z)
		end
	end
	do
		local len = math.sqrt(q.w*q.w + q.x*q.x + q.y*q.y + q.z*q.z)
		q.w, q.x, q.y, q.z = q.w/len, q.x/len, q.y/len, q.z/len
	end
end

-- ── space transforms ──

local function space_local_yaw(cgx, cgy, cgz)
	return -cgy * RAD_TO_DEG, -cgx * RAD_TO_DEG
end

local function space_local_roll(cgx, cgy, cgz)
	return -cgz * RAD_TO_DEG, -cgx * RAD_TO_DEG
end

local function space_player(cgx, cgy, cgz, gv)
	local world_yaw = -(gv.y * cgy + gv.z * cgz)
	local sign = world_yaw < 0 and -1 or 1
	local yaw_rate = sign * math.min(math.abs(world_yaw) * 1.41, math.sqrt(cgy * cgy + cgz * cgz))
	return -yaw_rate * RAD_TO_DEG, -cgx * RAD_TO_DEG
end

local function space_world(cgx, cgy, cgz, gv)
	local deg_x_s = -(gv.x * cgx + gv.y * cgy + gv.z * cgz) * RAD_TO_DEG
	local gdpx = gv.x
	local pax, pay, paz = 1 - gv.x * gdpx, -gv.y * gdpx, -gv.z * gdpx
	local pa_len2 = pax * pax + pay * pay + paz * paz
	local deg_y_s = 0
	if pa_len2 > 0 then
		local pa_len = math.sqrt(pa_len2)
		pax, pay, paz = pax / pa_len, pay / pa_len, paz / pa_len
		local side_max = math.max(math.abs(gv.y), math.abs(gv.z))
		local side_reduction = side_max <= WORLD_SIDE_REDUCTION and 0 or math.min((side_max - WORLD_SIDE_REDUCTION) / WORLD_SIDE_REDUCTION, 1)
		deg_y_s = -side_reduction * (pax * cgx + pay * cgy + paz * cgz) * RAD_TO_DEG
	end
	return deg_x_s, deg_y_s
end

local space_handlers = {
	player = space_player,
	world = space_world,
	local_yaw = space_local_yaw,
	local_roll = space_local_roll,
}

-- JSM-style complementary filter: maintain gravity + orientation from gyro + accel
local function update_fusion(cgx, cgy, cgz, ax, ay, az, dt)
	local angle_speed, rot_w, rot_x, rot_y, rot_z = apply_gyro_rotation(cgx, cgy, cgz, dt)
	local ri_w, ri_x, ri_y, ri_z = rot_w, -rot_x, -rot_y, -rot_z

	local accel_mag = vlen(ax, ay, az)
	if accel_mag > 0 then
		local anx, any, anz = vnorm(ax, ay, az)

		smooth_accel_and_shakiness(ax, ay, az, ri_w, ri_x, ri_y, ri_z, dt)
		correct_gravity(ax, ay, az, anx, any, anz, accel_mag, ri_w, ri_x, ri_y, ri_z, angle_speed, dt)
		correct_quaternion_tilt()
	else
		local gv = gyro_state.grav
		gv.x, gv.y, gv.z = qrot_vec(gv.x, gv.y, gv.z, ri_w, ri_x, ri_y, ri_z)
	end

	-- normalize quaternion every frame to prevent drift
	local q = gyro_state.quat
	local len = math.sqrt(q.w*q.w + q.x*q.x + q.y*q.y + q.z*q.z)
	q.w, q.x, q.y, q.z = q.w/len, q.x/len, q.y/len, q.z/len
end

-- called by Rust on every sensor event (gyro AND accel)
function on_sensor_event(gx, gy, gz, ax, ay, az, dt, is_gyro)
	_gyro_raw.x, _gyro_raw.y, _gyro_raw.z = gx, gy, gz
	_accel_raw.x, _accel_raw.y, _accel_raw.z = ax, ay, az

	-- auto-init gravity from first valid accel reading
	if not gyro_state.grav_initialized then
		local amag = vlen(ax, ay, az)
		if amag > 0.1 then
			gyro_state.grav.x = -ax / amag
			gyro_state.grav.y = -ay / amag
			gyro_state.grav.z = -az / amag
			_gravity.x, _gravity.y, _gravity.z = gyro_state.grav.x, gyro_state.grav.y, gyro_state.grav.z
			gyro_state.grav_initialized = true
		end
	end

	if gyro_state.calibrating and is_gyro then
		gyro_state.cal_samples[#gyro_state.cal_samples + 1] = { x = gx, y = gy, z = gz }
		if #gyro_state.cal_samples % 100 == 0 then
			log(1, string.format("calibrating... %d samples collected", #gyro_state.cal_samples))
		end
		return
	end

	local cgx = gx - gyro_state.bias_x
	local cgy = gy - gyro_state.bias_y
	local cgz = gz - gyro_state.bias_z
	gyro_state.cal_gx, gyro_state.cal_gy, gyro_state.cal_gz = cgx, cgy, cgz

	update_fusion(cgx, cgy, cgz, ax, ay, az, dt)

	_gravity.x, _gravity.y, _gravity.z = gyro_state.grav.x, gyro_state.grav.y, gyro_state.grav.z
	_orientation.x, _orientation.y, _orientation.z, _orientation.w = gyro_state.quat.x, gyro_state.quat.y, gyro_state.quat.z, gyro_state.quat.w
end

-- called by Rust once per frame
function process_gyro(gx, gy, gz, dt)
	-- gyro hold check (was in on_update; moved here to keep gyro concerns together)
	if gyro_state.hold_button and not _is_held(gyro_state.hold_button) then
		gyro_state.active = false
		gyro_state.hold_button = nil
	end

	if gyro_state.calibrating then return {} end
	if not gyro_state.active then return {} end

	local cgx, cgy, cgz = gyro_state.cal_gx, gyro_state.cal_gy, gyro_state.cal_gz
	local gv = _gravity

	local deg_x_s, deg_y_s
	local space_fn = space_handlers[gyro_state.space] or space_local_yaw
	deg_x_s, deg_y_s = space_fn(cgx, cgy, cgz, gv)

	local dz = gyro_state.deadzone
	if dz and dz > 0 then
		local mag = math.sqrt(deg_x_s * deg_x_s + deg_y_s * deg_y_s)
		if mag < dz then
			deg_x_s, deg_y_s = 0, 0
		end
	end

	local conv = gyro_state.calibration / gyro_state.in_game_sens
	local dx = deg_x_s * dt * conv * gyro_state.sens_h
	local dy = deg_y_s * dt * conv * gyro_state.sens_v

	if gyro_state.accel_fn then
		local speed_dps = vlen(gx, gy, gz) * RAD_TO_DEG
		local factor = gyro_state.accel_fn(speed_dps)
		dx = dx * (factor or 1)
		dy = dy * (factor or 1)
	end

	gyro_state.accum_x = gyro_state.accum_x + dx
	gyro_state.accum_y = gyro_state.accum_y + dy

	local out_x = gyro_state.accum_x >= 0 and math.floor(gyro_state.accum_x) or math.ceil(gyro_state.accum_x)
	local out_y = gyro_state.accum_y >= 0 and math.floor(gyro_state.accum_y) or math.ceil(gyro_state.accum_y)
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
	gyro_state.bias_z = 0
	gyro_state.calibrating = false
	gyro_state.cal_samples = {}
	gyro_state.hold_button = nil
	gyro_state.active = false
	gyro_state.accum_x = 0
	gyro_state.accum_y = 0
	gyro_state.quat = { w = 1, x = 0, y = 0, z = 0 }
	gyro_state.grav = { x = 0, y = 0, z = 0 }
	gyro_state.grav_initialized = false
	gyro_state.shakiness = 0
	gyro_state.smooth_accel = { x = 0, y = 0, z = 0 }
	gyro_state.sensor_last_time = nil
	gyro_state.deadzone = 0
	gyro_state.space = "local_yaw"
	gyro_state.cal_gx = 0
	gyro_state.cal_gy = 0
	gyro_state.cal_gz = 0
	gyro_state.accel_fn = nil

	_gravity.x, _gravity.y, _gravity.z = 0, 0, 0
	_orientation.w, _orientation.x, _orientation.y, _orientation.z = 1, 0, 0, 0
end

local function make_curve(defaults, fn)
	local state = {}
	for k, v in pairs(defaults) do state[k] = v end
	local func
	func = function(arg)
		if type(arg) == "table" then
			for k, v in pairs(arg) do state[k] = v end
			return func
		end
		return fn(state, arg)
	end
	return func
end

curve = curve or {}
setmetatable(curve, { __index = function(_, k) error("unknown curve: " .. tostring(k), 2) end })

curve.precision = make_curve({ threshold = 5, min_factor = 0 }, function(s, speed)
	if s.threshold <= 0 then return 1 end
	local t = math.min(speed / s.threshold, 1)
	return s.min_factor + (1 - s.min_factor) * t * t * (3 - 2 * t)
end)

curve.linear = make_curve({ threshold = 20, min = 1, max = 2 }, function(s, speed)
	if s.threshold <= 0 then return s.max end
	local t = math.min(speed / s.threshold, 1)
	return s.min + (s.max - s.min) * t
end)
