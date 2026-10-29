# Moonglide

JoyShockMapper-inspired controller remapper in Rust, configured via Lua, using SDL2.
Supports gyro + mouse/keyboard output.

## Design Principles

These rules exist because breaking them caused real rework. If a change tensions against one of them, stop and ask.

- **Simple over clever.** Smallest code that works. No caches, dirty flags, event channels, or abstractions without a measured reason. Deleting code is the preferred fix.
- **Everything happens on the 1000 Hz frame.** Inputs are polled, every Lua stage runs, and all shared state is published once per frame (`process_sticks`, `process_gyro`, `on_btn_down/up`, `on_update`, `_touchpad_*` globals). Device events that SDL only pushes (e.g. touchpad) are collected into state, then published per frame like everything else — no lazy updates, no gating.
- **Performance means guarding the loop, not micro-optimizing.** Fresh small tables at 1000 Hz are effectively free; don't trade clarity or safety for invisible allocation wins. Real savings come from skipping work wholesale (e.g. gyro fusion runs only for `player`/`world` spaces).
- **Snapshot semantics.** Functions return fresh values. Never hand out tables or objects that will mutate later — consumers must be able to store a result and read it a frame later.
- **Lua owns the remapping.** Rust polls devices and shuttles data; all binding and processing logic lives in `src/lua/*.lua`. Every pipeline stage is an overridable global — wrap-and-extend (`local builtin = process_gyro`) must always work. New knobs are globals read by Lua per frame, never Rust-side config structs.
- **Defaults preserve existing behavior.** New shapes are opt-in (e.g. a scalar inner deadzone stays JSM-style circular; `{h, v}` opts into cross/square). JoyShockMapper is the reference for feel — diverge deliberately, and document the divergence.
- **Plugins are plain `require()`.** A plugin is a normal module with require-time side effects; the config dir is already on `package.path`. `reset()`/`reload()` drop `package.loaded` (modules re-execute fresh) and fire `on_reset(fn)` hooks after core + DSL restore. That is the entire plugin API — no framework, no loader, no manifest.
- **Verify before claiming done.** Behavior-affecting changes get proven equivalent or correct (harness diffs, smoke tests) plus `cargo test`, before commit.
- **Housekeeping:** never commit without explicit consent; keep AGENTS.md and `docs/` true to the code in the same change.

## Build & Run

```bash
nix develop                     # dev shell (SDL2 + Lua + Rust deps, Linux x86_64)
cargo build --release           # Linux (uinput + LuaJIT)
cargo run --release                          # no config (defaults only)
cargo run --release -- ./config.lua          # custom config
nix build .#moonglide           # nix package (lean SDL3: no GL/audio/video)
nix bundle --bundler . .#moonglide -o .t && cp -L .t moonglide.AppImage && rm .t
                                                # self-contained AppImage as a REAL file at
                                                # root (in nix develop: moonglide-bundle)
cp -f "$(nix build --no-link --print-out-paths .#moonglide)/bin/moonglide" moonglide
                                                # real binary at repo root (runs via local nix store)
```

Releases are manual (JSM-style, no CI): tag locally, bundle, upload with
`gh release create vX.Y.Z out.AppImage --generate-notes`. The Windows exe must
be built on a real Windows machine (`release-windows.bat` at the repo root);
there is no cross-compilation.

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
├── main.rs            — SDL init, event loop, Lua config load, per-controller poll loop
├── config.rs          — loads Lua scripts (tables/bindings/sticks/gyro/events)
├── api.rs             — Lua ↔ Rust glue functions (_press_key, _is_held, log, etc.)
├── mapping.rs         — Mapper: button/axis state → active keyboard/mouse output
├── controller.rs      — SDL controller open/close, poll_buttons/poll_axes/poll_sensors/poll_touchpad
├── lua_coroutines.rs  — PendingThread, execute/poll Lua coroutines
├── output_devices.rs  — OutputDevices (mouse+kbd), apply + synchronize
├── frame_pacer.rs     — FramePacer (drift-compensated sleep at target FPS)
├── style.rs           — ANSI escape helpers for console output
├── output/
│   ├── mouse_linux.rs    — uinput mouse (relative move, buttons)
│   ├── mouse_windows.rs  — SendInput mouse (relative move, buttons)
│   ├── keyboard_linux.rs — uinput keyboard (key map from match fn)
│   └── keyboard_windows.rs — SendInput keyboard (key map from match fn)
└── lua/
    ├── tables.lua     — con/key/mouse typed ref tables (incl. touchpad_touch_left/right)
    ├── bindings.lua   — bind.* DSL, press/release/instant/toggle/held/wait
    ├── sticks.lua     — process_sticks (deadzone, cross-gate, ring, triggers), process_touchpad, cleanup_controller
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
SDL events → ControllerEvent (connect/disconnect only) → match in main loop
                                                                     │
                                                  Per-controller polling loop:
                                                  │
                                                  ├── poll_buttons → handle_btn_down/up
                                                  │         │
                                                  │         └── on_btn_down/up (Lua)
                                                  │              • Chord check → fire or continue
                                                  │              • Double-press check
                                                  │              • Modeshift check → consume
                                                  │              • Normal press
                                                  │              • Retroactive consumption
                                                  │
                                                  ├── poll_axes → axis_state[controller]
                                                  │
                                                  ├── poll_sensors → on_sensor_event (Lua)
                                                  │         │
                                                  │         └── process_gyro (Lua) → dev.mouse
                                                  │
                                                  └── poll_touchpad → _touchpad_* globals
                                                                     │
                                                                     ▼
                              ┌───────────────────────────────────────────┐
                              │  process_sticks (Lua → pressed/released   │
                              │    events → btn_down/btn_up)              │
                              ├───────────────────────────────────────────┤
                              │  process_touchpad (Lua → pressed/released │
                              │    events → btn_down/btn_up)              │
                              ├───────────────────────────────────────────┤
                              │  on_update (Lua coroutine)                │
                              ├───────────────────────────────────────────┤
                              │  poll pending Lua coroutines              │
                              ├───────────────────────────────────────────┤
                              │  drain_actions → dev.apply                │
                              │  → dev.synchronize_all                     │
                              └───────────────────────────────────────────┘
```

Main loop runs at ~1000 Hz with drift-compensated frame pacing (FramePacer). The Lua VM runs on the main thread; only the REPL thread is separate. The `mapper` mutex is locked briefly for button/key state queries, never held across Lua calls.

## Lua DSL Registration (config.rs)

`setup_dsl()` concatenates and executes the 5 Lua source files (`tables.lua`, `bindings.lua`, `sticks.lua`, `gyro.lua`, `events.lua`) into the Lua VM as a single chunk. This defines:

- `con`/`key`/`mouse` typed ref tables (`tables.lua`)
- `bind.*` DSL, user helpers (`press`, `release`, `instant`, `toggle`, `held`, `wait`) (`bindings.lua`)
- `process_sticks()` — deadzone, cross-gate, ring, trigger processing (`sticks.lua`)
- `process_touchpad()` — touchpad zone button detection (`sticks.lua`)
- `cleanup_controller()` — stick/trigger/touchpad state cleanup on disconnect (`sticks.lua`)
- `process_gyro()` — four gyro spaces, deadzone, calibration, enable/disable, acceleration curves (`gyro.lua`)
- `on_sensor_event()` — fusion, gravity auto-init, bias subtraction (`gyro.lua`)
- `make_curve()` — factory for acceleration curve helpers (`gyro.lua`)
- `curve.precision` / `curve.linear` — built-in acceleration curves (`gyro.lua`)
- `on_btn_down/up/update` — event dispatching (chords, modeshift, DP, hold timers) (`events.lua`)
- `on_reset(fn)` / `_run_reset_hooks()` — plugin reset hooks, run after `reset()`/`reload()` restores core + DSL (`events.lua`); `_reset_internals()` also clears `package.loaded` so `require()` re-executes plugins

`init_bare()` sets `package.path` to `./?.lua`. `load(path)` prepends the config file's directory to `package.path`, then executes the file. No Rust-side struct conversion — all binding logic stays in Lua.

## Config Management

Available from the REPL or from any config:

| Function | Description |
|----------|-------------|
| `reset()` | Clear all bindings, gyro state, stick state, config globals, and release all held keys. Re-runs the DSL to reset all Lua-side state. Gyro calibration bias (`_gyro_bias_x/y/z`) is preserved. |
| `reload()` | Same as `reset()`, then re-loads the CLI config file from disk. No-op if no config was given at launch. |

```lua
> reset()
> dofile("path/to/new-config.lua")

> reload()  -- re-reads the CLI config from disk
```

All these functions are globals — your config can override any of them by defining a new function with the same name. Wrap a built-in by capturing it in a local first: `local builtin = process_gyro` then define your own `process_gyro(...)` that calls it. To restore defaults, reassign from the saved local. See the source files in `src/lua/` for each function's exact contract.

Typed ref actions (e.g. `bind.press(con.a, key.space)`) are auto-wrapped as Lua functions via `wrap_action(action, mode)` in `bindings.lua`:
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
- **Sensor fusion**: JSM-style complementary filter — gyro rotation → quaternion → accel smoothing + shakiness → gravity correction → quaternion tilt correction. Runs **only for `player`/`world` spaces** (internal units deg/s + g, matching GamepadMotionHelpers); `local_yaw`/`local_roll` bypass fusion entirely
- **on_sensor_event()** called once per frame (~1000 Hz) with the latest gyro + accel sample: updates `_gyro_raw`/`_accel_raw` globals, handles calibration samples (gated on `is_gyro`); runs bias subtraction and drives fusion (fusion spaces only, which also sync `_gravity`/`_orientation`)
- **Gyro deadzone**: configurable in deg/s (`GYRO_CUTOFF_SPEED`), suppresses output below threshold on per-frame velocity (2D magnitude after space transform)
- **Acceleration curves**: `gyro { acceleration = fn }` — user-defined function that takes bias-corrected speed (deg/s) and returns a multiplier; applied after sensitivity/deadzone, before sub-pixel accumulation
- `curve.precision { threshold, min_factor }` — smoothstep 0→1×, configurable via table call
- `curve.linear { threshold, min, max }` — linear ramp min→max, configurable via table call
- Both curves share state across frames; reset via `reset()`
- Bias subtraction: `value - bias` for X, Y, Z axes
- Calibration: `gyro_calibrate_start()` collects samples, `gyro_calibrate_stop()` computes per-axis bias (now 3 axes including Z)
- Calibration bias stored in `_gyro_bias_x/y/z` globals — persists across `reset()` and `reload()`
- Gravity auto-initialized from first valid accelerometer reading — no hardcoded orientation guess
- Activation: `gyro_enable()`/`gyro_disable()`/`gyro_toggle()`/`gyro_hold()` called from bindings

> Note: `player` and `world` spaces are **experimental** — `local_yaw` and `local_roll` are fully stable.

See `src/lua/gyro.lua` for implementation.

## Touchpad Processing

`process_touchpad(id)` is called every frame in the main loop for each connected controller. It reads the `_touchpad_fingers` global (set by Rust from SDL poll data) and compares current zone state against the previous frame to emit press/release events for touchpad zone buttons:

| Lua Global | Type | Range | Description |
|------------|------|-------|-------------|
| `_touchpad_touching` | bool | — | Any finger on touchpad |
| `_touchpad_x` | float | 0–1 | Primary finger X position (0=left, 1=right) |
| `_touchpad_y` | float | 0–1 | Primary finger Y position (0=top, 1=bottom) |
| `_touchpad_pressure` | float | 0–1 | Primary finger pressure |
| `_touchpad_fingers` | table | — | `{ [finger_id] = { x, y, pressure } }` for multitouch |

Zone buttons available in the `con` table:

| Button | Condition |
|--------|-----------|
| `con.touchpad_touch` | Any finger on any zone |
| `con.touchpad_touch_left` | Any finger with x < 0.5 |
| `con.touchpad_touch_right` | Any finger with x ≥ 0.5 |

Internally: `process_touchpad()` checks both zones across all fingers, compares against `prev_touchpad_zones[id]`, and emits press/release events routed through the same `handle_btn_down/up` pipeline as physical buttons.

`cleanup_controller(id)` clears touchpad zone state (plus stick and trigger state) on disconnect. It is called from Rust's disconnect handler and is also overridable.

See `src/lua/sticks.lua` for implementation.

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
| `left_stick_inner_deadzone` | 0.15 | [sticks.md](docs/sticks.md) — scalar (circular) or `{h, v}` (cross/square) |
| `left_stick_outer_deadzone` | 1.0 | [sticks.md](docs/sticks.md) |
| `right_stick_inner_deadzone` | 0.15 | [sticks.md](docs/sticks.md) — scalar (circular) or `{h, v}` (cross/square) |
| `right_stick_outer_deadzone` | 1.0 | [sticks.md](docs/sticks.md) |
| `left_stick_diagonal_angle` | 22.5 | [sticks.md](docs/sticks.md) — degrees; lower = steeper tilt for diagonals |
| `right_stick_diagonal_angle` | 22.5 | [sticks.md](docs/sticks.md) |
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
