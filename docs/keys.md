# Key & Button Reference

## Controller buttons

Standard SDL gamepad button names:

| Button | Notes |
|--------|-------|
| `a` | |
| `b` | |
| `x` | |
| `y` | |
| `dpad_up` | |
| `dpad_down` | |
| `dpad_left` | |
| `dpad_right` | |
| `left_shoulder` | LB |
| `right_shoulder` | RB |
| `left_stick` | Stick click (L3) |
| `right_stick` | Stick click (R3) |
| `start` | |
| `back` | Select |
| `guide` | Home/PS/Xbox button |
| `left_trigger` | Analog trigger |
| `right_trigger` | Analog trigger |
| `touchpad_click` | Physical touchpad press |
| `touchpad_touch` | Finger on touchpad surface |
| `misc_1` | Miscellaneous button |
| `paddle_1` | Rear paddle 1 |
| `paddle_2` | Rear paddle 2 |
| `paddle_3` | Rear paddle 3 |
| `paddle_4` | Rear paddle 4 |

Unrecognized SDL button indices produce `unknown_N` names (e.g. `unknown_20`).

## Stick virtual buttons

| Button | Description |
|--------|-------------|
| `left_stick_up` | Left stick cross-gate direction |
| `left_stick_down` | |
| `left_stick_left` | |
| `left_stick_right` | |
| `right_stick_up` | Right stick cross-gate direction |
| `right_stick_down` | |
| `right_stick_left` | |
| `right_stick_right` | |
| `left_ring_inner` | Left stick between deadzone and ring position |
| `left_ring_outer` | Left stick above ring position |
| `right_ring_inner` | Right stick between deadzone and ring position |
| `right_ring_outer` | Right stick above ring position |

## Mouse buttons

| Name | Description |
|------|-------------|
| `left_mouse` | Left click |
| `right_mouse` | Right click |
| `middle_mouse` | Middle click |

## Keyboard keys

### Modifiers

| Name | Key |
|------|-----|
| `left_control` | Left Ctrl |
| `left_shift` | Left Shift |
| `left_alt` | Left Alt |
| `left_meta` | Left Meta/Super/Windows |
| `right_control` | Right Ctrl |
| `right_shift` | Right Shift |
| `right_alt` | Right Alt (AltGr) |
| `right_meta` | Right Meta/Super |

### Navigation & editing

| Name | Key |
|------|-----|
| `space` | Space |
| `enter` | Return |
| `tab` | Tab |
| `esc` | Escape |
| `backspace` | Backspace |
| `delete` | Delete |
| `insert` | Insert |
| `home` | Home |
| `end` | End |
| `page_up` | Page Up |
| `page_down` | Page Down |
| `caps_lock` | Caps Lock |
| `num_lock` | Num Lock |
| `scroll_lock` | Scroll Lock |
| `sysrq` | Print Screen |
| `minus` | - |
| `equal` | = |
| `leftbrace` | [ |
| `rightbrace` | ] |
| `semicolon` | ; |
| `apostrophe` | ' |
| `grave` | ` |
| `backslash` | \\ |
| `comma` | , |
| `dot` | . |
| `slash` | / |

### Arrow keys

`up`, `down`, `left`, `right`

### Function keys

`f1` through `f12`

### Letters and digits

Single character strings: `"a"`–`"z"`, `"0"`–`"9"`
