local function make_ref(src, field, val)
	return { __kind = "ref", src = src, field = field, val = val }
end

local con_names = {
	"a", "b", "x", "y",
	"dpad_up", "dpad_down", "dpad_left", "dpad_right",
	"left_shoulder", "right_shoulder",
	"left_stick", "right_stick",
	"start", "back", "guide",
	"left_trigger", "right_trigger",
	"touchpad_click", "touchpad_touch",
	"misc_1",
	"paddle_1", "paddle_2", "paddle_3", "paddle_4",
	"left_stick_up", "left_stick_down", "left_stick_left", "left_stick_right",
	"right_stick_up", "right_stick_down", "right_stick_left", "right_stick_right",
	"left_ring_inner", "left_ring_outer", "right_ring_inner", "right_ring_outer",
}
con = setmetatable({}, { __index = function(_, k)
	error("unknown button '" .. tostring(k) .. "' — check spelling", 2)
end })
for _, n in ipairs(con_names) do con[n] = make_ref("con", n, n) end

key = setmetatable({}, { __index = function(_, k)
	error("unknown key '" .. tostring(k) .. "' — check spelling", 2)
end })
local key_names = {
	"esc",
	"1", "2", "3", "4", "5", "6", "7", "8", "9", "0",
	"minus", "equal", "backspace", "tab",
	"q", "w", "e", "r", "t", "y", "u", "i", "o", "p",
	"leftbrace", "rightbrace", "enter",
	"left_control",
	"a", "s", "d", "f", "g", "h", "j", "k", "l",
	"semicolon", "apostrophe", "grave",
	"left_shift", "backslash",
	"z", "x", "c", "v", "b", "n", "m",
	"comma", "dot", "slash",
	"right_shift", "left_alt", "space",
	"caps_lock",
	"f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12",
	"num_lock", "scroll_lock",
	"right_control", "sysrq", "right_alt",
	"home", "up", "page_up", "left", "right", "end", "down", "page_down",
	"insert", "delete",
	"left_meta", "right_meta",
	"zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
}
local digit_overrides = {
	zero = "0", one = "1", two = "2", three = "3", four = "4",
	five = "5", six = "6", seven = "7", eight = "8", nine = "9",
}
for _, n in ipairs(key_names) do
	local val = digit_overrides[n] or n
	key[n] = make_ref("key", n, val)
end

mouse = setmetatable({}, { __index = function(_, k)
	error("unknown mouse button '" .. tostring(k) .. "' — check spelling", 2)
end })
local mouse_names = { "left", "right", "middle", "wheel_up", "wheel_down" }
local mouse_overrides = { left = "left_mouse", right = "right_mouse", middle = "middle_mouse" }
for _, n in ipairs(mouse_names) do
	local val = mouse_overrides[n] or n
	mouse[n] = make_ref("mouse", n, val)
end
