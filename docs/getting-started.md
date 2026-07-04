# Getting Started

## Quick start

Create a `config.lua`:

```lua
-- Stick directions as WASD
bind.press(con.left_stick_up, key.w)
bind.press(con.left_stick_down, key.s)
bind.press(con.left_stick_left, key.a)
bind.press(con.left_stick_right, key.d)

-- Face buttons
bind.press(con.a, key.space)
bind.tap(con.x, mouse.left)
bind.press(con.b, function()
    press(key.left_control)
    press(key.left_shift)
end)

-- Right stick as arrow keys
bind.press(con.right_stick_up, key.up)
bind.press(con.right_stick_down, key.down)
bind.press(con.right_stick_left, key.left)
bind.press(con.right_stick_right, key.right)
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
