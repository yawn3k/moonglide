# Analog Inputs

## Sticks

Analog sticks can produce virtual directional buttons and ring buttons based on deflection.

## Stick direction buttons

Virtual buttons that fire when the stick is pushed in a direction through a cross-gate:

| Button | Direction |
|--------|-----------|
| `con.left_stick_up` | Left stick pushed up |
| `con.left_stick_down` | Left stick pushed down |
| `con.left_stick_left` | Left stick pushed left |
| `con.left_stick_right` | Left stick pushed right |
| `con.right_stick_up` | Right stick pushed up |
| `con.right_stick_down` | Right stick pushed down |
| `con.right_stick_left` | Right stick pushed left |
| `con.right_stick_right` | Right stick pushed right |

```lua
bind.press(con.left_stick_up, key.w)
bind.press(con.left_stick_down, key.s)
bind.press(con.left_stick_left, key.a)
bind.press(con.left_stick_right, key.d)
```

### Cross-gate detection

Direction is determined by JoyShockMapper's cross-gate method:

| Condition | Result |
|-----------|--------|
| `x < -0.5 \|y\|` | Left |
| `x > 0.5 \|y\|` | Right |
| `y < -0.5 \|x\|` | Down |
| `y > 0.5 \|x\|` | Up |

Diagonals activate two adjacent directions. To suppress diagonal bindings, use chords with an empty callback:

```lua
bind.chord({con.left_stick_up, con.left_stick_right}, "")
bind.chord({con.left_stick_up, con.left_stick_left}, "")
bind.chord({con.left_stick_down, con.left_stick_right}, "")
bind.chord({con.left_stick_down, con.left_stick_left}, "")
```

## Deadzones

Deadzones control the analog-to-digital threshold for stick direction buttons.

| Global | Default | Description |
|--------|---------|-------------|
| `left_stick_inner_deadzone` | 0.15 | Fraction of full deflection (0–1). Stick below this produces no direction. |
| `left_stick_outer_deadzone` | 1.0 | Fraction of full deflection (0–1). Stick at or above this produces full direction. |
| `right_stick_inner_deadzone` | 0.15 | Same for right stick |
| `right_stick_outer_deadzone` | 1.0 | Same for right stick |

### Processing

1. If magnitude ≤ inner deadzone → x/y zeroed, no direction
2. If magnitude between inner and outer → linearly rescaled from 0–1 over the band
3. If magnitude ≥ outer → normalized to unit length, full direction

```lua
left_stick_inner_deadzone = 0.3   -- ignore stick until 30%
left_stick_outer_deadzone = 1.0   -- max direction at full tilt
```

## Ring buttons

Ring buttons are virtual buttons that fire based on stick deflection within the deadzone band.

| Button | Description |
|--------|-------------|
| `con.left_ring_inner` | Left stick between deadzone and position threshold |
| `con.left_ring_outer` | Left stick above position threshold |
| `con.right_ring_inner` | Right stick between deadzone and position threshold |
| `con.right_ring_outer` | Right stick above position threshold |

| Global | Default | Description |
|--------|---------|-------------|
| `left_ring_position` | 0.8 | Fraction of deadzone-processed magnitude (0–1). Inner ring below this, outer ring above. |
| `right_ring_position` | 0.8 | Same for right stick |

```lua
left_ring_position = 0.8
bind.press(con.left_ring_outer, key.r)
bind.press(con.left_ring_inner, key.left_shift)
```

The ring uses deadzone-processed stick magnitude (0–1 range):

- **Inner** ring: active when `0 < processed_magnitude < position`
- **Outer** ring: active when `processed_magnitude > position`

## Triggers

Analog triggers can act as digital buttons via the trigger threshold.

### Trigger threshold

| Global | Default | Description |
|--------|---------|-------------|
| `trigger_threshold` | 3000 | Axis value (0–32767) that must be exceeded to count as "pressed" |

```lua
trigger_threshold = 5000   -- require deeper press
```

### Usage

Trigger names: `con.left_trigger`, `con.right_trigger`

Triggers generate press/release events like physical buttons (debounced at 50 ms):

```lua
bind.press(con.left_trigger, function()
    press(key.left_control)
end)
bind.release(con.left_trigger, function()
    instant(key.four)
end)
```

With `trigger_threshold = 3000`, the trigger must be pressed past ~9% (3000/32767) to activate. Increase the threshold for a stiffer activation point.

## Touchpad

The DualSense touchpad is polled every frame. Position data is exposed via Lua globals, and zone buttons fire through the normal `handle_btn_down/up` pipeline.

### Position globals

Updated every frame:

| Global | Type | Range | Description |
|--------|------|-------|-------------|
| `_touchpad_touching` | bool | — | Any finger on touchpad |
| `_touchpad_x` | float | 0–1 | Primary finger X position (0=left, 1=right) |
| `_touchpad_y` | float | 0–1 | Primary finger Y position (0=top, 1=bottom) |
| `_touchpad_pressure` | float | 0–1 | Primary finger pressure |
| `_touchpad_fingers` | table | — | `{ [finger_id] = { x, y, pressure } }` for multitouch |

### Zone buttons

| Button | Condition |
|--------|-----------|
| `con.touchpad_touch` | Any finger on any zone |
| `con.touchpad_touch_left` | Any finger with x < 0.5 |
| `con.touchpad_touch_right` | Any finger with x ≥ 0.5 |

```lua
bind.press(con.touchpad_touch_right, gyro_hold)
```

### Overriding `process_touchpad()`

The default implementation checks both halves of the touchpad and emits press/release events for the three zone buttons. Override it for custom zone layouts:

```lua
local builtin_touchpad = process_touchpad
function process_touchpad(id)
    local result = builtin_touchpad(id)
    -- add custom logic (e.g. trackpad clicks)
    return result
end
```
