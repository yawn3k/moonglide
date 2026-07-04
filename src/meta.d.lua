---@meta moonglide

--- Controller buttons — physical buttons, stick directions, ring zones.
---@class con_table
---@field a string
---@field b string
---@field x string
---@field y string
---@field dpad_up string
---@field dpad_down string
---@field dpad_left string
---@field dpad_right string
---@field left_shoulder string
---@field right_shoulder string
---@field left_stick string
---@field right_stick string
---@field start string
---@field back string
---@field guide string
---@field left_trigger string
---@field right_trigger string
---@field touchpad_click string
---@field touchpad_touch string
---@field misc_1 string
---@field paddle_1 string
---@field paddle_2 string
---@field paddle_3 string
---@field paddle_4 string
---@field left_stick_up string
---@field left_stick_down string
---@field left_stick_left string
---@field left_stick_right string
---@field right_stick_up string
---@field right_stick_down string
---@field right_stick_left string
---@field right_stick_right string
---@field left_ring_inner string
---@field left_ring_outer string
---@field right_ring_inner string
---@field right_ring_outer string
con = {}

--- Keyboard keys — letters, modifiers, navigation, function keys.
---@class key_table
---@field esc string
---@field ['1'] string
---@field ['2'] string
---@field ['3'] string
---@field ['4'] string
---@field ['5'] string
---@field ['6'] string
---@field ['7'] string
---@field ['8'] string
---@field ['9'] string
---@field ['0'] string
---@field minus string
---@field equal string
---@field backspace string
---@field tab string
---@field q string
---@field w string
---@field e string
---@field r string
---@field t string
---@field y string
---@field u string
---@field i string
---@field o string
---@field p string
---@field leftbrace string
---@field rightbrace string
---@field enter string
---@field left_control string
---@field s string
---@field d string
---@field f string
---@field g string
---@field h string
---@field j string
---@field k string
---@field l string
---@field semicolon string
---@field apostrophe string
---@field grave string
---@field left_shift string
---@field backslash string
---@field z string
---@field c string
---@field v string
---@field b string
---@field n string
---@field m string
---@field comma string
---@field dot string
---@field slash string
---@field right_shift string
---@field left_alt string
---@field space string
---@field caps_lock string
---@field f1 string
---@field f2 string
---@field f3 string
---@field f4 string
---@field f5 string
---@field f6 string
---@field f7 string
---@field f8 string
---@field f9 string
---@field f10 string
---@field f11 string
---@field f12 string
---@field num_lock string
---@field scroll_lock string
---@field right_control string
---@field sysrq string
---@field right_alt string
---@field home string
---@field up string
---@field page_up string
---@field left string
---@field right string
---@field end string
---@field down string
---@field page_down string
---@field insert string
---@field delete string
---@field left_meta string
---@field right_meta string
key = {}

--- Mouse buttons.
---@class mouse_table
---@field left string
---@field right string
---@field middle string
mouse = {}

--- Gyro configuration.
---@class gyro_config
---@field mode? string "off" | "toggle" | "hold_enable" | "hold_disable" | "always_on"
---@field button? string
---@field sensitivity? number|number[]
---@field gyro_sens? number|number[]
---@field calibration? number
---@field in_game_sens? number

--- @param cfg gyro_config
function gyro(cfg) end

--- Start gyro calibration (collect bias samples).
function gyro_calibrate_start() end
--- Stop gyro calibration and apply bias.
function gyro_calibrate_stop() end

--- Load additional config file.
---@param path string
function include(path) end

--- Clear all bindings and release held keys.
function reset() end

--- Yield for N seconds (non-blocking).
---@param seconds number
function wait(seconds) end

--- Log a message at the given level.
---@param level integer
---@param msg string
function log(level, msg) end

--- Hold a key down while the binding button is held.
---@param key string
function press(key) end

--- Tap a key (press then release).
---@param key string
---@param opts? {press_time?: integer}
function instant(key, opts) end

--- Release a previously pressed key.
---@param key string
function release(key) end

--- Toggle a key between held and released.
---@param key string
function toggle(key) end

--- Rapid-pulse a key at ~100ms while the binding button is held.
---@param key string
function turbo(key) end

--- The button that triggered the current binding callback.
---@type string
_current_btn = ""

--- @class binding_opts
--- @field delay? integer ms delay for hold bindings

--- @class double_press_opts
--- @field window? integer ms window for double-press

--- @class bind_table
--- @field press fun(button: string, action: string|fun())
--- @field tap fun(button: string, action: string|fun())
--- @field hold fun(button: string, action: string|fun(), opts?: binding_opts)
--- @field release fun(button: string, action: string|fun())
--- @field turbo fun(button: string, action: string|fun())
--- @field chord fun(buttons: string[], action: string|fun())
--- @field double_press fun(button: string, action: string|fun(), opts?: double_press_opts)
--- @field modeshift fun(modifiers: string[], button: string, action: string|fun())
bind = {}

---@type integer
log_level = 0
---@type integer
trigger_threshold = 3000
---@type integer
hold_press_time = 400
---@type integer
instant_press_time = 40
---@type integer
double_press_window = 200
---@type number
left_stick_inner_deadzone = 0.15
---@type number
left_stick_outer_deadzone = 1.0
---@type number
right_stick_inner_deadzone = 0.15
---@type number
right_stick_outer_deadzone = 1.0
---@type number
left_ring_position = 0.8
---@type number
right_ring_position = 0.8
