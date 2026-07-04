# Configuration & CLI

## Loading a config

```bash
moonglide [config.lua]
```

- **No arguments** — starts with defaults, no bindings loaded
- **`config.lua`** — load the given Lua config file on startup

The entire script runs on startup. Bindings, gyro config, and globals are all set via Lua globals and function calls.

## REPL (read-eval-print loop)

While Moonglide is running, type Lua expressions at the terminal. Each expression is evaluated immediately.

```lua
> left_stick_inner_deadzone = 0.25
> ok
> bind.press(con.guide, key.esc)
> ok
```

After each REPL command, the following globals are re-read and applied immediately:

- `log_level`
- `trigger_threshold`
- `instant_press_time`
- `double_press_window`
- `left_stick_inner_deadzone`, `left_stick_outer_deadzone`
- `right_stick_inner_deadzone`, `right_stick_outer_deadzone`
- `left_ring_position`, `right_ring_position`
- Any `bind.*` calls
- `reset()`, `include()`

Other changes require a full config reload.

### REPL commands

| Command | Effect |
|---------|--------|
| `include("path")` | Load additional config file |
| `reset()` | Clear all bindings, release held keys |
| `gyro_calibrate_start()` | Start gyro calibration |
| `gyro_calibrate_stop()` | Stop gyro calibration |

## `include()` — loading additional files

```lua
include("./config/extra.lua")                -- relative to CWD
include("~/.config/moonglide/game.lua")      -- home directory
include("/absolute/path.lua")
```

Loaded files are processed the same as the main config — all `bind.*` calls register bindings.

## `reset()` — clearing bindings

```lua
reset()         -- removes all bindings, releases all held keys
```

Useful from the REPL to reload config:

```lua
> reset()
> ok
> include("./new_config.lua")
> ok
```

Or inside a binding as a panic button:

```lua
bind.press(con.guide, function()
    reset()
    print("all bindings cleared")
end)
```

## Timing globals

Set these in your config or from the REPL:

| Global | Default | Description |
|--------|---------|-------------|
| `hold_press_time` | 400 | Default ms delay for `bind.hold` (overridable per-binding with `{delay=N}`). Read at config load only, not re-read from REPL. |
| `instant_press_time` | 40 | How long `instant(key)` holds the key before releasing (ms) |
| `double_press_window` | 200 | Default ms window for `bind.double_press` (overridable with `{window=N}`) |

### `wait(seconds)`

Yield a binding's coroutine for `seconds` (non-blocking — other bindings still fire during the wait). Useful for timed sequences.

```lua
bind.press(con.y, function()
    press(key.f)
    wait(0.5)
    instant(key.e)
end)
```

## `_current_btn`

Inside binding callbacks, the read-only variable `_current_btn` holds the button name that triggered the binding:

```lua
bind.press(con.a, function()
    print("pressed: " .. _current_btn)  -- prints "pressed: a"
end)
```

## Logging

| Global | Default | Description |
|--------|---------|-------------|
| `log_level` | 0 | 0 = off, 1 = controller buttons, triggers, touchpad |

Use `log(level, "msg")` inside binding functions for custom log messages:

```lua
bind.press(con.guide, function()
    log(1, "guide button pressed")
end)
```

## Exit

Press `Escape` to quit Moonglide.

## Output styling

Console output uses ANSI escape codes:

| Style | Used for |
|-------|----------|
| Bold green | Config loaded, calibration events, controller found |
| Bold yellow | Warnings (no config, config errors) |
| Bold red | Errors (SDL/init failures, REPL errors) |
| Dim | Instruction text, log messages, calibration progress |
| Green | REPL `> ok` |
| Yellow | Controller disconnected |
