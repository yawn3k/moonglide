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

Directions are computed from the deadzone-processed stick position using a cross-gate. Each quadrant is split into three zones by `*_diagonal_angle` (default 22.5°), the half-width of the diagonal band:

| Zone (angle off the nearest cardinal axis) | Directions |
|---------------------------------------------|------------|
| Below `45° − diagonal_angle` (e.g. < 22.5°) | Cardinal only (e.g. `left_stick_left`) |
| Within the diagonal band (e.g. 22.5°–67.5°) | Both adjacent (e.g. `left_stick_left` + `left_stick_up`) |
| Above `45° + diagonal_angle` (e.g. > 67.5°) | The other cardinal only (e.g. `left_stick_up`) |

Nothing fires while the stick is inside the deadzone. To suppress diagonal bindings, use chords with an empty callback:

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
| `left_stick_inner_deadzone` | 0.15 | Fraction of full deflection (0–1). Number = circular deadzone (JSM-style, applied to the stick magnitude); `{h, v}` table = per-axis cross/square. |
| `left_stick_outer_deadzone` | 1.0 | Fraction of full deflection (0–1). Stick at or above this produces full direction. Follows the inner deadzone's shape (circular for scalar, per-axis for a table). |
| `right_stick_inner_deadzone` | 0.15 | Same for right stick |
| `right_stick_outer_deadzone` | 1.0 | Same for right stick |
| `left_stick_diagonal_angle` | 22.5 | Half-angle (degrees) of the diagonal band. Lower = diagonals require a steeper tilt. |
| `right_stick_diagonal_angle` | 22.5 | Same for right stick |

### Processing

A scalar `*_inner_deadzone` uses a **circular** deadzone (JSM-style):

1. If the stick's magnitude ≤ inner threshold → zeroed (nothing fires)
2. If magnitude is between inner and outer → radially rescaled from 0–1 over the band, angle preserved
3. If magnitude ≥ outer → capped at the outer radius

A `{h, v}` table uses a **per-axis** (cross/square) deadzone:

1. Per axis: if |value| ≤ that axis's threshold → zeroed
2. Per axis: if between inner and outer → linearly rescaled from 0–1 over the band
3. Per axis: if ≥ outer → full deflection (1 or −1)

```lua
left_stick_inner_deadzone = 0.3   -- ignore stick until 30% (circular)
left_stick_outer_deadzone = 1.0   -- max direction at full tilt
```

### Cross/square deadzones

Pass a `{horizontal, vertical}` table to `*_inner_deadzone` for a per-axis (cross/square) deadzone — each axis is zeroed independently until it crosses its own threshold:

```lua
left_stick_inner_deadzone = { 0.05, 0.2 }  -- horizontal 5%, vertical 20%
```

Here a strafe with a vertical tilt below 20% snaps to pure horizontal, so UP/DOWN only fires once the vertical axis genuinely crosses 0.2. Each surviving axis is linearly rescaled from its threshold to its full-deflection threshold. A scalar (e.g. `0.15`) instead uses the circular deadzone above.

### Harder diagonals

By default a direction pair fires once the stick tilts ≥22.5° off the nearest cardinal axis. Lower `*_diagonal_angle` to require a steeper tilt before the second axis joins — useful when strafing (A/D) accidentally triggers W/S:

```lua
left_stick_diagonal_angle = 12   -- UP/DOWN only joins at ≥33° tilt
```

With `12`, a strafe tilted 26° fires pure `left_stick_left`; only deliberate 33°+ tilts produce `left_stick_left` + `left_stick_up`. The default `22.5` matches the original behavior. The angle is measured on the deadzone-processed position: the circular (scalar) deadzone preserves the stick angle, so it maps 1:1 to the raw tilt, while a per-axis table's rescale shifts angles, so the effective diagonal threshold moves slightly with the `{h, v}` values.

## Ring buttons

Ring buttons are virtual buttons based on stick deflection. The ring radius is the physical `sqrt(x² + y²)` in 0–1 measured from raw (pre-deadzone) values, so the threshold compares against the stick's actual position regardless of the deadzone shape. However, no ring button fires until the stick is pushed past the deadzone — a stick resting inside the deadzone produces no ring state.

| Button | Description |
|--------|-------------|
| `con.left_ring_inner` | Left stick past the deadzone but below the position threshold |
| `con.left_ring_outer` | Left stick past the deadzone and at or above the position threshold |
| `con.right_ring_inner` | Right stick past the deadzone but below the position threshold |
| `con.right_ring_outer` | Right stick past the deadzone and at or above the position threshold |

| Global | Default | Description |
|--------|---------|-------------|
| `left_ring_position` | 0.8 | Fraction of full deflection (0–1). Inner ring below this, outer ring above. |
| `right_ring_position` | 0.8 | Same for right stick |

```lua
left_ring_position = 0.8
bind.press(con.left_ring_outer, key.r)
bind.press(con.left_ring_inner, key.left_shift)
```

The ring is suppressed while the stick is inside the deadzone (deadzone-processed position is zero), then uses the raw radius (0–1):

- **Inner** ring: active when `past_deadzone and raw_radius < position`
- **Outer** ring: active when `past_deadzone and raw_radius > position`

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
