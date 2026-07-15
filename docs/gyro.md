# Gyro

Gyro maps controller rotation to mouse movement. Uses sensor fusion (JSM-style complementary filter) for reliable gravity/orientation tracking across all four gyro spaces.

## Config

```lua
gyro {
    sensitivity = 1.0,        -- multiplier (single or {h, v})
    calibration = 45.454,     -- CS2 baseline: 360° turn at sens=1, in_game_sens matching the game
    in_game_sens = 1.0,       -- match your game's sensitivity slider
    deadzone = 0,             -- deg/s threshold below which output is suppressed
    space = "local_yaw",      -- one of: local_yaw, local_roll, player, world
    acceleration = nil,       -- function(speed_dps) → multiplier, or a curve helper
}
```

`sensitivity` (alias `gyro_sens`) accepts:
- A single number (same for both axes): `sensitivity = 1.0`
- A table for separate horizontal/vertical: `sensitivity = {1.0, 1.5}`

## Activation

Gyro is activated from bindings:

```lua
bind.press(con.l_trigger, gyro_enable)
bind.release(con.l_trigger, gyro_disable)
```

| Helper | Behavior |
|--------|----------|
| `gyro_enable()` | Enable gyro |
| `gyro_disable()` | Disable gyro, zero accumulated motion |
| `gyro_toggle()` | Toggle gyro on/off |
| `gyro_hold()` | Enable while current button held (auto-disables on release) |

### Hold example

```lua
bind.press(con.touchpad_touch, gyro_hold)
```

## Gyro Spaces

Four spaces controlling how rotation maps to mouse movement:

| Space | Horizontal (mouse X) | Vertical (mouse Y) | Yaw ignored? |
|-------|---------------------|-------------------|--------------|
| `local_yaw` (default) | controller yaw (gyro Y) | controller pitch (gyro X) | no |
| `local_roll` | controller roll (gyro Z) | controller pitch (gyro X) | yes |
| `player` | world-horizontal yaw (projected via gravity) | controller pitch (gyro X) | no |
| `world` | world-vertical rotation (gravity-axis) | world-horizontal rotation | no |

**`player`** and **`world`** use gravity (auto-initialized from the first accelerometer reading) to project gyro rotation onto world axes. Works in any controller orientation.

> Note: `player` and `world` are **experimental** — the gravity projection may behave unexpectedly in some orientations. `local_yaw` and `local_roll` are confirmed working correctly.

## Calibration

Captures gyro resting bias and subtracts it from readings:

```lua
bind.press(con.b, function()
    print("calibrating in 1s — hold controller still")
    wait(1)
    gyro_calibrate_start()
    wait(2)
    gyro_calibrate_stop()
    print("calibration done")
end)
```

- `gyro_calibrate_start()` — begin collecting bias samples
- `gyro_calibrate_stop()` — compute per-axis bias (X, Y, Z), subtract from readings

## Per-Frame Globals

Updated every sensor event (~2000 Hz combined gyro + accel):

| Global | Content |
|--------|---------|
| `_gyro_raw` | Latest raw gyro `{x, y, z}` (rad/s, before bias subtraction) |
| `_accel_raw` | Latest raw accelerometer `{x, y, z}` (m/s²) |
| `_gravity` | Current estimated gravity direction `{x, y, z}` (normalized) |
| `_orientation` | Current orientation quaternion `{x, y, z, w}` |

## Sensor Event Callback

`on_sensor_event(gx, gy, gz, ax, ay, az, dt, is_gyro)` is called on every sensor event (gyro AND accel). Can be overridden for custom calibration, fusion, or logging. The default implementation handles bias, JSM-style complementary filter, gravity tracking, and orientation.

## Acceleration Curves

`gyro { acceleration = fn }` — a user-defined function that scales gyro output based on raw controller rotational speed (deg/s). The function takes one argument (`speed_dps`) and returns a multiplier applied to the per-frame pixel output.

```lua
gyro { acceleration = function(speed)
    return 1 + math.min(speed / 20, 1)     -- 1×→2× boost, plateaus at 20 dps
end }
```

### Built-in Curves: `curve.*`

| Helper | Params | Defaults | Description |
|--------|--------|----------|-------------|
| `curve.precision` | `threshold`, `min_factor` | 5, 0 | Smoothstep from `min_factor`→1×. Suppresses jitter at low speeds. |
| `curve.linear` | `threshold`, `min`, `max` | 20, 1, 2 | Linear ramp from `min`→`max`. |

Configure via table call (returns itself for use in `gyro {}`):

```lua
-- configure + use separately
curve.precision { threshold = 8, min_factor = 0.15 }
gyro { acceleration = curve.precision }

-- one-shot
gyro { acceleration = curve.linear { threshold = 30, min = 1, max = 3 } }

-- compose with your own logic
gyro { acceleration = function(s)
    return curve.precision(s) * curve.linear(s)
end }
```

The `acceleration` function runs after sensitivity and deadzone but before sub-pixel accumulation. `curve.precision` and `curve.linear` read their state from a shared table that persists across frames — configuration persists until changed or `reset()`.

## Deadzone

`gyro { deadzone = 2 }` — JSM-style `GYRO_CUTOFF_SPEED`. When the 2D output velocity magnitude (after space transform) is below the threshold in deg/s, per-frame output is zeroed. Below threshold → zero output; above → full output. Hard gate, no ramp. Works per-frame on velocity (not accumulator), so no jump when crossing back out.
