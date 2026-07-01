# Getting Started

## Quick start

Create a `config.lua`:

```lua
-- Stick directions as WASD
bind.press("left_stick_up", "w")
bind.press("left_stick_down", "s")
bind.press("left_stick_left", "a")
bind.press("left_stick_right", "d")

-- Face buttons
bind.press("a", "space")
bind.tap("x", "left_mouse")
bind.press("b", function()
    press("left_control")
    press("left_shift")
end)

-- Right stick as arrow keys
bind.press("right_stick_up", "up")
bind.press("right_stick_down", "down")
bind.press("right_stick_left", "left")
bind.press("right_stick_right", "right")
```

Run Moonglide with your config:

```bash
moonglide ./config.lua
```

Or without a config (defaults only, no bindings):

```bash
moonglide
```

While it's running, type Lua commands in the terminal (REPL) to adjust on the fly:

```lua
> left_stick_inner_deadzone = 0.25
> ok
```

Press **Escape** to quit.

## Building from source

```bash
nix develop                     # enter dev shell
cargo build --release           # build
cargo run --release             # run with defaults
cargo run --release -- ./config.lua  # run with config
```

Requires SDL2, LuaJIT, udev, and pkg-config. The `nix develop` command provides all dependencies on Linux x86_64.
