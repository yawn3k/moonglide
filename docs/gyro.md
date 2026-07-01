# Gyro

Gyro maps controller rotation to mouse movement.

## Config

Set a `gyro {}` block in your Lua config:

```lua
gyro {
    mode = "hold_enable",
    button = "x",
    gyro_sens = 1.0,
    calibration = 45.454,
    in_game_sens = 1.0,
}
```

## Modes

| Mode | Behavior |
|------|----------|
| `"off"` | Gyro disabled |
| `"toggle"` | Press `button` to toggle gyro on/off |
| `"hold_enable"` | Gyro active only while `button` is held |
| `"hold_disable"` | Gyro active unless `button` is held |
| `"always_on"` | Gyro always active (default when `mode` is omitted from `gyro {}`) |

If the entire `gyro {}` block is omitted, gyro is off (`GyroMode::Off`).
If `gyro {}` is present but `mode` is not specified, it defaults to `"always_on"`.

## `button`

The activation button. Use any button name (see [keys.md](keys.md)).

Required unless mode is `"off"`.

## Sensitivity

| Field | Default | Description |
|-------|---------|-------------|
| `sensitivity` | 1.0 | Gyro multiplier (alias: `gyro_sens`) |
| `gyro_sens` | 1.0 | Overrides `sensitivity` if both are set |
| `calibration` | 45.454 | Real World Calibration factor (CS2 baseline). A 360° controller rotation produces a 360° in-game turn at `gyro_sens=1` and `in_game_sens` matching the game's sensitivity slider. |
| `in_game_sens` | 1.0 | The game's mouse sensitivity value. Set to match your game's sensitivity slider. |

`gyro_sens` (or `sensitivity`) accepts either:
- A single number (same for both axes): `gyro_sens = 1.0`
- A table for separate horizontal/vertical: `gyro_sens = {1.0, 1.5}`
- A two-number string: `gyro_sens = "1.0 1.5"`

## Gyro space

Local yaw gyrospace:

- Controller yaw (z-axis rotation) → mouse X
- Controller pitch (y-axis rotation) → mouse -Y
- Roll (x-axis) is ignored

## Calibration

Calibration captures the gyro's resting bias and subtracts it from readings. Use from a binding:

```lua
bind.press("b", function()
    print("calibrating in 1s — hold controller still")
    wait(1)
    gyro_calibrate_start()
    wait(2)
    gyro_calibrate_stop()
    print("calibration done")
end)
```

The calibration helper functions:
- `gyro_calibrate_start()` — begin collecting bias samples
- `gyro_calibrate_stop()` — compute per-axis bias, subtract from readings
