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
├── main.rs            — SDL init, event loop, Lua config load, Lua gyro invocation
├── config.rs          — loads Lua scripts (tables/bindings/sticks/gyro/events)
├── api.rs             — Lua ↔ Rust glue functions (_press_key, _is_held, log, etc.)
├── mapping.rs         — Mapper: button state → active keyboard/mouse output
├── controller.rs      — SDL controller open/close/sensor events → ControllerEvent
├── lua_coroutines.rs  — PendingThread, execute/poll Lua coroutines
├── output_devices.rs  — OutputDevices (mouse+kbd)
├── style.rs           — ANSI escape helpers for console output
├── output/
│   ├── mouse.rs       — uinput mouse (relative move, buttons)
│   └── keyboard.rs    — uinput keyboard (key map, mouse button helpers)
└── lua/
    ├── tables.lua     — con/key/mouse typed ref tables
    ├── bindings.lua   — bind.* DSL, press/release/instant/toggle/held/wait
    ├── sticks.lua     — process_sticks (deadzone, cross-gate, ring, triggers)
    ├── gyro.lua       — process_gyro (RWS math, bias, calibration, enable/disable)
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
                  └──→ process_gyro (Lua) → dev.mouse        │
                                                               ▼
                   ┌──────────────────────────────────────────┐
                   │  handle_btn_down / handle_btn_up         │
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
                   │  • poll Lua coroutines    │
                   │  • defer_map timers       │
                   │  • process_stick_buttons  │
                   │  • process_hold_turbo     │
                   │  • process_turbo          │
                   │  • process_instant_releases│
                   │  • drain_actions → dev    │
                   │  • dev.synchronize_all    │
                   └──────────────────────────┘
```

Main loop runs at ~240 Hz. The `shared_cbs` mutex is cloned before passing callbacks to Lua code to avoid deadlocks (the mutex is never held while Lua executes).

## Lua DSL Registration

In `config.rs`:

1. `setup_dsl()` registers all `bind.*` helpers on the Lua globals. Each helper (e.g. `bind.press`, `bind.chord`, `bind.modeshift`) pushes a table entry to `_bindings`, `_chords`, `_double_press`, or `_modeshifts` Lua tables.

2. `process_pending()` reads those Lua tables and converts entries into Rust `Binding`, `ChordBinding`, `DoublePressBinding`, `ModeshiftBinding` structs, storing them in the shared `Config`.

3. `init_bare()` sets up a minimal Lua state with DSL + `include()` helper. Used when no config file is given on the CLI.

4. `load()` reads a file, executes it, calls `process_pending()`, returns the final `Config`.

Typed ref actions (e.g. `bind.press(con.a, key.space)`) are auto-wrapped as Lua functions via `extract_func()`:
- `press` / `hold` / `chord` → wrapper calls `press(key.X)`
- `release` / `tap` / `double_press` → wrapper calls `instant(key.X)`

See [docs/bindings.md](docs/bindings.md) for the full Lua API reference.

## Button Handling

### `handle_btn_down` (main.rs:57)

Called when a physical button is pressed, a touchpad touch occurs, or a trigger crosses the threshold.

1. **Chord check** — if the current button + held buttons match a chord definition, the chord fires. All chorded buttons are consumed (their individual press bindings are skipped).
2. **Double-press check** — if the button was pressed within `window_ms`, fire the double-press binding instead, consume the press.
3. **Modeshift check** — if the button has a modeshift whose modifiers are all held, fire the modeshift callback and mark the button as consumed (suppresses `bind.release` on release).
4. **Normal press** — if nothing consumed it, fire `bind.press` bindings for this button.
5. **Retroactive consumption** — after all the above, check if the current button acts as a modeshift **modifier** for any already-held button. If so, retroactively mark that held button as consumed. This handles the case where the modifier is pressed *after* the action button (e.g., pressing left_trigger, then right_trigger — right_trigger's modeshift consumes left_trigger retroactively).

### `handle_btn_up` (main.rs:119)

Called on button release.

1. **Consumed path** — if the button was marked consumed by a modeshift/double-press/chord, release its mapped keys via `button_up()` and skip all release bindings.
2. **Normal path** — check for tap bindings (held < 180ms), fire `bind.release` bindings, release mapped keys via `button_up()`.

## Mapper Internals (mapping.rs)

| Field | Purpose |
|-------|---------|
| `held_buttons` | Currently pressed buttons → press time |
| `press_held` | Button → keys mapped via `press("key")` |
| `held_keys` | Set of currently held keys (deduplicated) |
| `toggled` | Keys toggled via `toggle("key")` |
| `action_queue` | Pending press/release actions to drain each frame |
| `consumed_presses` | Buttons whose press was consumed by chord/modeshift/DP |

Key design: when a button is released, `button_up()` checks if any OTHER still-held button also maps to the same key before releasing it. If both `left_shoulder` and `left_ring_outer` map to `right_mouse`, releasing one won't release the key while the other is still held.

## Stick Processing

See [docs/sticks.md](docs/sticks.md) for stick button names, deadzones, cross-gate detection, ring position, and trigger threshold configuration.

Internally: `process_stick_buttons()` is called every frame in the main loop. It builds the `current` set of directions (cross-gate + ring), compares against `prev`, fires press events for new directions, checks chords/modeshifts, and fires releases for directions no longer in `current`.

## Gyro Processing

See [docs/gyro.md](docs/gyro.md) for gyro modes, sensitivity, calibration, and activation settings.

Internally:
- **RWS** (Ratcheting Walking Sim): yaw → mouse X, pitch → mouse -Y, roll ignored
- Bias subtraction: `value - bias` for X and Y axes
- Output: `angle_deg × calibration × sensitivity / in_game_sens`
- Calibration: `gyro_calibrate_start()` collects samples, `gyro_calibrate_stop()` computes per-axis bias
- Trigger-based gyro activation uses `axis_motion()` to read raw axis value vs `trigger_threshold`, bypassing the button event system

See `src/lua/gyro.lua` for implementation.

## Triggers

See [docs/sticks.md](docs/sticks.md#triggers) for trigger usage and configuration.

Internally: analog triggers (SDL axes 105/106) are handled by `TriggerTracker` in `sticks.lua`. State tracked per controller instance, crosses threshold → generates `ButtonDown`/`ButtonUp` events → routed through `handle_btn_down`/`handle_btn_up`. Debounced at 50ms.

## Output Devices (output_devices.rs)

- `OutputDevices` wraps optional `VirtualMouse` + `VirtualKeyboard`
- `apply(key, press)` dispatches keyboard keys and mouse buttons (`left_mouse`, `right_mouse`, `middle_mouse`)
- `synchronize_all()` calls uinput sync on both devices each frame

## Stick Statics (Global Config)

Loaded from Lua globals and stored in `AtomicU16` / `AtomicU8` statics, re-read on REPL commands. Only `LOG_LEVEL` is a Rust atomic; all others are read by Lua directly each frame:

| Lua Global | Static | Default | Docs |
|---|---|---|---|
| `log_level` | `LOG_LEVEL` | 0 | — |
| `trigger_threshold` | `TRIGGER_THRESHOLD` | 3000 | [sticks.md](docs/sticks.md) |
| `instant_press_time` | `INSTANT_PRESS_TIME` | 40 | — |
| `left_stick_inner_deadzone` | `LEFT_STICK_INNER` | 0.15 | [sticks.md](docs/sticks.md) |
| `left_stick_outer_deadzone` | `LEFT_STICK_OUTER` | 1.0 | [sticks.md](docs/sticks.md) |
| `right_stick_inner_deadzone` | `RIGHT_STICK_INNER` | 0.15 | [sticks.md](docs/sticks.md) |
| `right_stick_outer_deadzone` | `RIGHT_STICK_OUTER` | 1.0 | [sticks.md](docs/sticks.md) |
| `left_ring_position` | `LEFT_RING_POSITION` | 0.8 | [sticks.md](docs/sticks.md) |
| `right_ring_position` | `RIGHT_RING_POSITION` | 0.8 | [sticks.md](docs/sticks.md) |
