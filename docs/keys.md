# Key & Button Reference

Use these with the built-in tables: `con` for controller, `key` for keyboard, `mouse` for mouse buttons.

```lua
bind.press(con.a, key.space)
bind.tap(con.x, mouse.left)
```

## Controller buttons

Standard SDL gamepad button names (via `con` table):

| Field | Button | Notes |
|-------|--------|-------|
| `con.a` | `a` | |
| `con.b` | `b` | |
| `con.x` | `x` | |
| `con.y` | `y` | |
| `con.dpad_up` | `dpad_up` | |
| `con.dpad_down` | `dpad_down` | |
| `con.dpad_left` | `dpad_left` | |
| `con.dpad_right` | `dpad_right` | |
| `con.left_shoulder` | `left_shoulder` | LB |
| `con.right_shoulder` | `right_shoulder` | RB |
| `con.left_stick` | `left_stick` | Stick click (L3) |
| `con.right_stick` | `right_stick` | Stick click (R3) |
| `con.start` | `start` | |
| `con.back` | `back` | Select |
| `con.guide` | `guide` | Home/PS/Xbox button |
| `con.left_trigger` | `left_trigger` | Analog trigger |
| `con.right_trigger` | `right_trigger` | Analog trigger |
| `con.touchpad_click` | `touchpad_click` | Physical touchpad press |
| `con.touchpad_touch` | `touchpad_touch` | Finger on touchpad surface |
| `con.misc_1` | `misc_1` | Miscellaneous button |
| `con.paddle_1` | `paddle_1` | Rear paddle 1 |
| `con.paddle_2` | `paddle_2` | Rear paddle 2 |
| `con.paddle_3` | `paddle_3` | Rear paddle 3 |
| `con.paddle_4` | `paddle_4` | Rear paddle 4 |

Unrecognized SDL button indices produce `unknown_N` names (use as string literals).

## Stick virtual buttons (via `con`)

| Field | Description |
|-------|-------------|
| `con.left_stick_up` | Left stick cross-gate direction |
| `con.left_stick_down` | |
| `con.left_stick_left` | |
| `con.left_stick_right` | |
| `con.right_stick_up` | Right stick cross-gate direction |
| `con.right_stick_down` | |
| `con.right_stick_left` | |
| `con.right_stick_right` | |
| `con.left_ring_inner` | Left stick between deadzone and ring position |
| `con.left_ring_outer` | Left stick above ring position |
| `con.right_ring_inner` | Right stick between deadzone and ring position |
| `con.right_ring_outer` | Right stick above ring position |

## Mouse buttons (via `mouse`)

| Field | Description |
|-------|-------------|
| `mouse.left` | Left click |
| `mouse.right` | Right click |
| `mouse.middle` | Middle click |

## Keyboard keys (via `key`)

### Modifiers

| Field | Key |
|-------|-----|
| `key.left_control` | Left Ctrl |
| `key.left_shift` | Left Shift |
| `key.left_alt` | Left Alt |
| `key.left_meta` | Left Meta/Super/Windows |
| `key.right_control` | Right Ctrl |
| `key.right_shift` | Right Shift |
| `key.right_alt` | Right Alt (AltGr) |
| `key.right_meta` | Right Meta/Super |

### Navigation & editing

| Field | Key |
|-------|-----|
| `key.space` | Space |
| `key.enter` | Return |
| `key.tab` | Tab |
| `key.esc` | Escape |
| `key.backspace` | Backspace |
| `key.delete` | Delete |
| `key.insert` | Insert |
| `key.home` | Home |
| `key.end` | End |
| `key.page_up` | Page Up |
| `key.page_down` | Page Down |
| `key.caps_lock` | Caps Lock |
| `key.num_lock` | Num Lock |
| `key.scroll_lock` | Scroll Lock |
| `key.sysrq` | Print Screen |
| `key.minus` | - |
| `key.equal` | = |
| `key.leftbrace` | [ |
| `key.rightbrace` | ] |
| `key.semicolon` | ; |
| `key.apostrophe` | ' |
| `key.grave` | ` |
| `key.backslash` | \\ |
| `key.comma` | , |
| `key.dot` | . |
| `key.slash` | / |

### Arrow keys

`key.up`, `key.down`, `key.left`, `key.right`

### Function keys

`key.f1` through `key.f12`

### Letters

`key.a`–`key.z`

### Digits (spelled out)

`key.zero`–`key.nine` (since `key.0` is Lua syntax error — digits use spelled-out names)

## String literals are not accepted

The old `"string"` syntax is **not supported** — use the table syntax (`con.a`, `key.space`, `mouse.left`) everywhere. This gives you autocomplete in editors with LuaLS support.
