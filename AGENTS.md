# Moonglide

JoyShockMapper-inspired controller remapper in Rust, configured via Lua, using SDL2.
Supports gyro + mouse/keyboard output.

## Build & Run

```bash
nix develop                     # dev shell (SDL2 + Lua + Rust deps, Linux x86_64)
cargo build --release           # Linux (uinput + LuaJIT)
cargo run --release                          # no config (defaults only)
cargo run --release -- ./config.lua          # custom config
```

**Windows:** `cargo build --release` (SendInput + bundled SDL2 + Lua 5.4).
Requires VS Build Tools and CMake. See [docs/getting-started.md](docs/getting-started.md).

## Testing

```bash
cargo test                    # all tests
cargo test <name>             # single test by name
```

## Architecture

```
src/
├── main.rs            — SDL init, event loop, Lua config load, gyro/accel/sensor fusion
├── config.rs          — loads Lua scripts (tables/bindings/sticks/gyro/events)
├── api.rs             — Lua ↔ Rust glue functions (_press_key, _is_held, log, etc.)
├── mapping.rs         — Mapper: button state → active keyboard/mouse output
├── controller.rs      — SDL controller open/close/sensor events → ControllerEvent
├── lua_coroutines.rs  — PendingThread, execute/poll Lua coroutines
├── output_devices.rs  — OutputDevices (mouse+kbd)
├── frame_pacer.rs     — FramePacer (drift-compensated sleep at target FPS)
├── style.rs           — ANSI escape helpers for console output
├── output/
│   ├── mouse.rs       — uinput mouse (relative move, buttons)
│   └── keyboard.rs    — uinput keyboard (key map, mouse button helpers)
└── lua/
    ├── tables.lua     — con/key/mouse typed ref tables
    ├── bindings.lua   — bind.* DSL, press/release/instant/toggle/held/wait
    ├── sticks.lua     — process_sticks (deadzone, cross-gate, ring, triggers)
    ├── gyro.lua       — process_gyro (spaces, fusion, deadzone, calibration), on_sensor_event (fusion, gravity auto-init, bias)
    └── events.lua     — on_btn_down/up/update callbacks (chords, DP, modeshift, etc.)
```

### Key Dependencies

- `sdl2` (`use-pkgconfig`, `hidapi`) — controller input, gyro sensors
- `mlua` (`luajit`) — Lua scripting runtime
- `uinput` — all output (mouse + keyboard)

## Data Flow

```
                  ┌─ Lua REPL (terminal input thread)
                  │
SDL events → ControllerManager → ControllerEvent enum → match in main loop
                  │                                          │
            AxisMotion ──→ process_sticks (Lua) ────────────→ handle_btn_down/up
                  │                                          │
                  ├──→ on_sensor_event (Lua → fusion + globals)
                  │         │
                  │         └──→ process_gyro (Lua) → dev.mouse
                  │                                          │
                  └── Accelerometer ─→ on_sensor_event (Lua)  │
                                                               ▼
                   ┌──────────────────────────────────────────┐
                   │  on_btn_down / on_btn_up (Lua)           │
                   │  • Chord check → fire chord or continue  │
                   │  • Double-press check                    │
                   │  • Modeshift check → consume or continue │
                   │  • Normal press binding                  │
                   │  • Retroactive modeshift consumption      │
                   └──────────────────────────────────────────┘
                                 │
                                 ▼
                   ┌──────────────────────────┐
                   │  Per-frame (after events) │
                   │  • process_sticks (Lua)   │
                   │  • on_update (Lua)        │
                   │  • poll Lua coroutines    │
                   │  • drain_actions → dev    │
                   │  • dev.synchronize_all    │
                   └──────────────────────────┘
```

Main loop runs at ~1000 Hz with drift-compensated frame pacing (FramePacer). The Lua VM runs on the main thread; only the REPL thread is separate. The `mapper` mutex is locked briefly for button/key state queries, never held across Lua calls.

## Lua DSL Registration (config.rs)

`setup_dsl()` concatenates and executes the 5 Lua source files (`tables.lua`, `bindings.lua`, `sticks.lua`, `gyro.lua`, `events.lua`) into the Lua VM as a single chunk. This defines:

- `con`/`key`/`mouse` typed ref tables (`tables.lua`)
- `bind.*` DSL, user helpers (`press`, `release`, `instant`, `toggle`, `held`, `wait`) (`bindings.lua`)
- `process_sticks()` — deadzone, cross-gate, ring, trigger processing (`sticks.lua`)
- `process_steicks()` — deadzone, cross-gate, ring, trigger processing (`sticks.lua`)
- `process_gyro()` — four gyro spaces, deadzone, calibration, enable/disable, acceleration curves (`gyro.lua`)
- `on_sensor_event()` — fusion, gravity auto-init, bias subtraction (`gyro.lua`)
- `make_curve()` — factory for acceleration curve helpers (`gyro.lua`)
- `curve.precision` / `curve.linear` — built-in acceleration curves (`gyro.lua`)
- `on_btn_down/up/update` — event dispatching (chords, modeshift, DP, hold timers) (`events.lua`)

`init_bare()` sets `package.path` to `./?.lua`. `load(path)` prepends the config file's directory to `package.path`, then executes the file. No Rust-side struct conversion — all binding logic stays in Lua.

## Config Management

Available from the REPL or from any config:

| Function | Description |
|----------|-------------|
| `reset()` | Clear all bindings, gyro state, stick state, config globals, and release all held keys. Re-runs the DSL to reset all Lua-side state. |
| `reload()` | Same as `reset()`, then re-loads the CLI config file from disk. No-op if no config was given at launch. |

```lua
> reset()
> dofile("path/to/new-config.lua")

> reload()  -- re-reads the CLI config from disk
```

All five functions are globals — your config can override any of them by defining a new function with the same name. Wrap a built-in by capturing it in a local first: `local builtin = process_gyro` then define your own `process_gyro(...)` that calls it. To restore defaults, reassign from the saved local. See the source files in `src/lua/` for each function's exact contract.

Typed ref actions (e.g. `bind.press(con.a, key.space)`) are auto-wrapped as Lua functions via `extract_action()` / `extract_instant_action()` in `bindings.lua`:
- `press` / `hold` / `chord` → wrapper calls `press(key.X)`
- `release` / `tap` / `double_press` → wrapper calls `instant(key.X)`

See [docs/bindings.md](docs/bindings.md) for the full Lua API reference.

## Button Handling

### `handle_btn_down` (main.rs)

Called when a physical button is pressed, a touchpad touch occurs, or a trigger crosses the threshold.

1. **Chord check** — if the current button + held buttons match a chord definition, the chord fires. All chorded buttons are consumed (their individual press bindings are skipped).
2. **Double-press check** — if the button was pressed within `window_ms`, fire the double-press binding instead, consume the press.
3. **Modeshift check** — if the button has a modeshift whose modifiers are all held, fire the modeshift callback and mark the button as consumed (suppresses `bind.release` on release).
4. **Normal press** — if nothing consumed it, fire `bind.press` bindings for this button.
5. **Retroactive consumption** — after all the above, check if the current button acts as a modeshift **modifier** for any already-held button. If so, retroactively mark that held button as consumed. This handles the case where the modifier is pressed *after* the action button (e.g., pressing left_trigger, then right_trigger — right_trigger's modeshift consumes left_trigger retroactively).

### `handle_btn_up` (main.rs)

Called on button release.

1. **Consumed path** — if the button was marked consumed by a modeshift/double-press/chord, release its mapped keys via `button_up()` and skip all release bindings.
2. **Normal path** — check for tap bindings (held < 180ms), fire `bind.release` bindings, release mapped keys via `button_up()`.

## Mapper Internals (mapping.rs)

| Field | Purpose |
|-------|---------|
| `held_buttons` | Currently pressed buttons (HashSet) |
| `held_keys` | Currently held output keys (dedup for action_queue) |
| `action_queue` | Pending press/release actions to drain each frame |

Mapper is a thin state machine — press tracking (`is_held`, `held_buttons`), output key dedup (`press_key`/`release_key` check `held_keys` before queuing), and action queuing (`drain_actions`). All binding logic (chords, modeshifts, DP, consumption, key release dedup across held buttons) lives in Lua's `events.lua`.

## Stick Processing

See [docs/sticks.md](docs/sticks.md) for stick button names, deadzones, cross-gate detection, ring position, and trigger threshold configuration.

Internally: `process_sticks()` is called every frame in the main loop. It builds the `current` set of directions (cross-gate + ring), compares against `prev`, fires press events for new directions, checks chords/modeshifts, and fires releases for directions no longer in `current`.

## Gyro Processing

See [docs/gyro.md](docs/gyro.md) for gyro modes, sensitivity, calibration, and activation settings.

Internally:
- **Four spaces**: `local_yaw` (default, yaw→X pitch→Y), `local_roll` (roll→X pitch→Y), `player` (world-horizontal yaw + local pitch via JSM), `world` (both axes world-relative via JSM)
- **Sensor fusion**: JSM-style complementary filter — gyro rotation → quaternion → accel smoothing + shakiness → gravity correction → quaternion tilt correction
- **on_sensor_event()** called on every gyro AND accel event (~2000 Hz combined): updates `_gravity`/`_orientation`/`_gyro_raw`/`_accel_raw` globals, handles calibration samples (gated on `is_gyro`), runs bias subtraction, drives fusion
- **Gyro deadzone**: configurable in deg/s (`GYRO_CUTOFF_SPEED`), suppresses output below threshold on per-frame velocity (2D magnitude after space transform)
- **Acceleration curves**: `gyro { acceleration = fn }` — user-defined function that takes raw speed (deg/s) and returns a multiplier; applied after sensitivity/deadzone, before sub-pixel accumulation
- `curve.precision { threshold, min_factor }` — smoothstep 0→1×, configurable via table call
- `curve.linear { threshold, min, max }` — linear ramp min→max, configurable via table call
- Both curves share state across frames; reset via `reset()`
- Bias subtraction: `value - bias` for X, Y, Z axes
- Calibration: `gyro_calibrate_start()` collects samples, `gyro_calibrate_stop()` computes per-axis bias (now 3 axes including Z)
- Gravity auto-initialized from first valid accelerometer reading — no hardcoded orientation guess
- Activation: `gyro_enable()`/`gyro_disable()`/`gyro_toggle()`/`gyro_hold()` called from bindings

> Note: `player` and `world` spaces are **experimental** — `local_yaw` and `local_roll` are fully stable.

See `src/lua/gyro.lua` for implementation.

## Triggers

See [docs/sticks.md](docs/sticks.md#triggers) for trigger usage and configuration.

Internally: analog triggers (SDL axes 105/106) are handled by `TriggerTracker` in `sticks.lua`. State tracked per controller instance, crosses threshold → generates `ButtonDown`/`ButtonUp` events → routed through `handle_btn_down`/`handle_btn_up`. Debounced at 50ms.

## Output Devices (output_devices.rs)

- `OutputDevices` wraps optional `VirtualMouse` + `VirtualKeyboard`
- `apply(key, press)` dispatches keyboard keys and mouse buttons (`left_mouse`, `right_mouse`, `middle_mouse`)
- `synchronize_all()` calls uinput sync on both devices each frame

## Stick Statics (Global Config)

Config globals read by Lua each frame (no Rust atomics — just Lua globals):

| Lua Global | Default | Docs |
|---|---|---|
| `log_level` | 0 | — |
| `trigger_threshold` | 3000 | [sticks.md](docs/sticks.md) |
| `instant_press_time` | 40 | — |
| `hold_press_time` | 400 | [bindings.md](docs/bindings.md) |
| `double_press_window` | 200 | [bindings.md](docs/bindings.md) |
| `left_stick_inner_deadzone` | 0.15 | [sticks.md](docs/sticks.md) |
| `left_stick_outer_deadzone` | 1.0 | [sticks.md](docs/sticks.md) |
| `right_stick_inner_deadzone` | 0.15 | [sticks.md](docs/sticks.md) |
| `right_stick_outer_deadzone` | 1.0 | [sticks.md](docs/sticks.md) |
| `left_ring_position` | 0.8 | [sticks.md](docs/sticks.md) |
| `right_ring_position` | 0.8 | [sticks.md](docs/sticks.md) |

`log_level` is the only Rust-side global stored in an `AtomicU8`, re-read from Lua on REPL commands. All others are read by Lua directly each frame — no Rust involvement.

## Gyro Config (gyro {}) Fields

Set via the `gyro(tbl)` function in config:

| Field | Default | Description |
|---|---|---|---|
| `sensitivity` / `gyro_sens` | 1.0 | Multiplier (single or `{h, v}`) |
| `calibration` | 45.454 | RWS factor (CS2 baseline) |
| `in_game_sens` | 1.0 | Game's mouse sensitivity value |
| `deadzone` | 0 | Gyro cutoff speed in deg/s (JSM `GYRO_CUTOFF_SPEED`) |
| `space` | `"local_yaw"` | One of: `local_yaw`, `local_roll`, `player`, `world` |
| `acceleration` | `nil` | Function `(speed_dps) → multiplier` or curve helper reference |
