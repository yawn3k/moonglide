# Bindings & Actions

Bindings map controller buttons to actions. Each binding has a **button name**, an **event type** (when it fires), and an **action** (function or string shorthand).

## Binding syntax

```lua
bind.event(button, action, opts?)
```

- `button` — a button name string (see [keys.md](keys.md))
- `action` — a function or a string (auto-wrapped, see below)
- `opts` — optional table with per-binding settings

## Event types

### `bind.press(button, action)`

Fires when the button is pressed down. If the action uses `press("key")`, that key stays held until the button is released.

```lua
bind.press("a", "space")              -- string shorthand
bind.press("b", function()            -- function
    press("left_control")
    press("left_shift")
end)
```

### `bind.tap(button, action)`

Fires on quick press-release (held < 180 ms). Ignores longer holds so tap and hold can coexist on the same button.

```lua
bind.tap("x", "left_mouse")
bind.tap("y", function()
    instant("e", { press_time = 20 })
end)
```

String shorthand auto-wraps as `instant(...)`:

```lua
bind.tap("x", "e")  -- same as → function() instant("e") end
```

### `bind.hold(button, action, opts?)`

Fires after the button is held for a delay. Default delay is `hold_press_time` (400 ms). Override per-binding with `{delay = ms}`.

```lua
bind.hold("x", function()
    press("r")      -- reload
end, { delay = 800 })
```

Does **not** fire on taps (< 180 ms), so tap and hold can coexist on the same button.

String shorthand auto-wraps as `press(...)`:

```lua
bind.hold("x", "r")  -- same as → function() press("r") end
```

### `bind.release(button, action)`

Fires when the button is released.

```lua
bind.release("left_shoulder", function()
    release("q")
end)
bind.release("b", function()
    instant("left_alt")
end)
```

String shorthand auto-wraps as `instant(...)`:

```lua
bind.release("b", "left_alt")  -- same as → function() instant("left_alt") end
```

### `bind.turbo(button, action)`

Fires repeatedly at ~100 ms while the button is held.

```lua
bind.turbo("right_shoulder", function()
    instant("left_mouse")
end)
```

### `bind.chord({buttons}, action)`

Fires when **all** specified buttons are held simultaneously. Individual button press and release bindings for the chorded buttons are suppressed while the chord is active.

```lua
bind.chord({"left_shoulder", "right_shoulder"}, "f")
```

String shorthand auto-wraps as `press(...)`:

```lua
bind.chord({"left_shoulder", "right_shoulder"}, "f")  -- same as → function() press("f") end
```

### `bind.double_press(button, action, opts?)`

Fires when the button is pressed twice within the window. Default window is `double_press_window` (200 ms). Override with `{window = ms}`.

```lua
bind.double_press("b", function()
    instant("tab")
end, { window = 300 })
```

String shorthand auto-wraps as `instant(...)`:

```lua
bind.double_press("b", "tab")  -- same as → function() instant("tab") end
```

### `bind.modeshift({modifiers}, action_button, fn)`

**Modeshift** — fires `fn` when **all** `modifiers` are held and `action_button` is pressed. The press is consumed: `bind.release` for `action_button` is suppressed on release, and press bindings for `action_button` are skipped while the modeshift is active.

If the modifier is pressed *after* the action button is already held, the held button is retroactively consumed so its `bind.release` is also suppressed.

```lua
bind.modeshift({"left_trigger", "right_trigger"}, "a", function()
    press("left_control")
    instant("z")   -- ctrl+z (undo)
end)
```

String shorthand auto-wraps as `press(...)`:

```lua
bind.modeshift({"left_shoulder"}, "x", "f")  -- same as → function() press("f") end
```

## Action helpers

Helpers callable inside binding callbacks to manipulate keyboard, mouse, and controller output.

Inside any callback, the read-only variable `_current_btn` holds the button name that triggered the binding:

```lua
bind.press("a", function()
    print("triggered by: " .. _current_btn)
end)
```

### `press("key")`

Hold a key down while the binding button is held. The key is automatically released when the button comes up.

```lua
bind.press("dpad_up", function()
    press("w")
end)
```

### `instant("key", opts?)`

Tap a key — press and release after `instant_press_time` ms (default 40). Pass `{press_time = N}` for a per-key override.

```lua
bind.tap("x", function()
    instant("left_mouse", { press_time = 20 })
end)
```

### `release("key")`

Release a key that was previously pressed. Typically used in `bind.release` handlers.

```lua
bind.press("dpad_left", function()
    press("a")
end)
bind.release("dpad_left", function()
    release("a")
end)
```

### `toggle("key")`

Alternate a key between held and released on each press.

```lua
bind.tap("start", function()
    toggle("left_meta")
end)
```

### `turbo("key")`

Rapid-pulse a key at ~100 ms while the binding button is held.

```lua
bind.press("right_stick", function()
    turbo("left_mouse")
end)
```

