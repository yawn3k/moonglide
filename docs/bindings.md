# Bindings & Actions

Bindings map controller buttons to actions. Each binding has a **button name**, an **event type** (when it fires), and an **action** (function or string shorthand).

## Binding syntax

```lua
bind.event(button, action, opts?)
```

- `button` — a button name from the `con` table (see [keys.md](keys.md))
- `action` — a function, or a reference from `key`/`mouse` (auto-wrapped, see below)
- `opts` — optional table with per-binding settings

Use the built-in tables for autocomplete-friendly names:

| Table | What it holds |
|-------|--------------|
| `con` | Controller buttons, stick directions, ring zones |
| `key` | Keyboard keys |
| `mouse` | Mouse buttons |

String literals like `"a"` or `"space"` are **not** accepted — you must use the table syntax (`con.a`, `key.space`, `mouse.left`). This gives you autocomplete in editors with LuaLS support.

## Event types

### `bind.press(button, action)`

Fires when the button is pressed down. If the action uses `press(key)`, that key stays held until the button is released.

```lua
bind.press(con.a, key.space)              -- string shorthand
bind.press(con.b, function()              -- function
    press(key.left_control)
    press(key.left_shift)
end)
```

### `bind.tap(button, action)`

Fires on quick press-release (held < 180 ms). Ignores longer holds so tap and hold can coexist on the same button.

```lua
bind.tap(con.x, mouse.left)
bind.tap(con.y, function()
    instant(key.e, { press_time = 20 })
end)
```

String shorthand auto-wraps as `instant(...)`:

```lua
bind.tap(con.x, key.e)  -- same as → function() instant(key.e) end
```

### `bind.hold(button, action, opts?)`

Fires after the button is held for a delay. Default delay is `hold_press_time` (400 ms). Override per-binding with `{delay = ms}`.

```lua
bind.hold(con.x, function()
    press(key.r)      -- reload
end, { delay = 800 })
```

Does **not** fire on taps (< 180 ms), so tap and hold can coexist on the same button.

String shorthand auto-wraps as `press(...)`:

```lua
bind.hold(con.x, key.r)  -- same as → function() press(key.r) end
```

### `bind.release(button, action)`

Fires when the button is released.

```lua
bind.release(con.left_shoulder, function()
    release(key.q)
end)
bind.release(con.b, function()
    instant(key.left_alt)
end)
```

String shorthand auto-wraps as `instant(...)`:

```lua
bind.release(con.b, key.left_alt)  -- same as → function() instant(key.left_alt) end
```

### `bind.turbo(button, action)`

Fires repeatedly at ~100 ms while the button is held.

```lua
bind.turbo(con.right_shoulder, function()
    instant(mouse.left)
end)
```

### `bind.chord({buttons}, action)`

Fires when **all** specified buttons are held simultaneously. Individual button press and release bindings for the chorded buttons are suppressed while the chord is active.

```lua
bind.chord({con.left_shoulder, con.right_shoulder}, key.f)
```

String shorthand auto-wraps as `press(...)`:

```lua
bind.chord({con.left_shoulder, con.right_shoulder}, key.f)  -- same as → function() press(key.f) end
```

### `bind.double_press(button, action, opts?)`

Fires when the button is pressed twice within the window. Default window is `double_press_window` (200 ms). Override with `{window = ms}`.

```lua
bind.double_press(con.b, function()
    instant(key.tab)
end, { window = 300 })
```

String shorthand auto-wraps as `instant(...)`:

```lua
bind.double_press(con.b, key.tab)  -- same as → function() instant(key.tab) end
```

### `bind.modeshift({modifiers}, action_button, fn)`

**Modeshift** — fires `fn` when **all** `modifiers` are held and `action_button` is pressed. The press is consumed: `bind.release` for `action_button` is suppressed on release, and press bindings for `action_button` are skipped while the modeshift is active.

If the modifier is pressed *after* the action button is already held, the held button is retroactively consumed so its `bind.release` is also suppressed.

```lua
bind.modeshift({con.left_trigger, con.right_trigger}, con.a, function()
    press(key.left_control)
    instant(key.z)   -- ctrl+z (undo)
end)
```

String shorthand auto-wraps as `press(...)`:

```lua
bind.modeshift({con.left_shoulder}, con.x, key.f)  -- same as → function() press(key.f) end
```

## Action helpers

Helpers callable inside binding callbacks to manipulate keyboard and mouse output.

Inside any callback, the read-only variable `_current_btn` holds the button name that triggered the binding:

```lua
bind.press(con.a, function()
    print("triggered by: " .. _current_btn)
end)
```

### `press(key)`

Hold a key down while the binding button is held. The key is automatically released when the button comes up.

```lua
bind.press(con.dpad_up, function()
    press(key.w)
end)
```

### `instant(key, opts?)`

Tap a key — press and release after `instant_press_time` ms (default 40). Pass `{press_time = N}` for a per-key override.

```lua
bind.tap(con.x, function()
    instant(mouse.left, { press_time = 20 })
end)
```

### `release(key)`

Release a key that was previously pressed. Typically used in `bind.release` handlers.

```lua
bind.press(con.dpad_left, function()
    press(key.a)
end)
bind.release(con.dpad_left, function()
    release(key.a)
end)
```

### `toggle(key)`

Alternate a key between held and released on each press.

```lua
bind.tap(con.start, function()
    toggle(key.left_meta)
end)
```

### `turbo(key)`

Rapid-pulse a key at ~100 ms while the binding button is held.

```lua
bind.press(con.right_stick, function()
    turbo(mouse.left)
end)
```

### `held(button)`

Check if a button is currently held down. Returns `true`/`false`. Useful for conditional logic inside callbacks.

```lua
bind.press(con.touchpad_touch, function()
    if held(con.left_shoulder) then
        -- ADS + gyro: use different sensitivity
        gyro_enable()
    else
        gyro_disable()
    end
end)
```

### Calibration

```lua
bind.press(con.b, function()
    gyro_calibrate_start()
    wait(2)
    gyro_calibrate_stop()
end)
```

## Per-frame callback

Define `function update()` to run logic every frame (~240 Hz). Inside `update()`, `held()`, `press()`, `release()`, `gyro_enable()`, etc. all work with live controller state.

Keys pressed via `press()` inside `update()` are automatically released on the next frame if `press()` is not called again — no explicit `release()` needed.

```lua
function update()
    -- gyro on touchpad touch
    if held(con.touchpad_touch) then
        gyro_enable()
    else
        gyro_disable()
    end

    -- movement keys
    if held(con.left_stick_up) then press(key.w) end
    if held(con.left_stick_left) then press(key.a) end
    if held(con.left_stick_down) then press(key.s) end
    if held(con.left_stick_right) then press(key.q) end
end
```

`bind.*` and `update()` coexist — use `bind.press`/`chord`/`modeshift` for event-driven actions, `update()` for stateful conditional logic.
