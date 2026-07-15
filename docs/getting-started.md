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

Reload or switch configs without restarting:

```lua
> reload()                    -- re-read CLI config from disk
> reset()                     -- clear everything to defaults
> dofile("./other.lua")       -- load a different config
```

## Building from source

```bash
nix develop                     # enter dev shell
cargo build --release           # build
cargo run --release             # run with defaults
cargo run --release -- ./config.lua  # run with config
```

Requires SDL2, LuaJIT, udev, and pkg-config. The `nix develop` command provides all dependencies on Linux x86_64.

## Building on Windows

**Prerequisites:**
- Rust (from [rustup.rs](https://rustup.rs))
- Visual Studio 2022 Build Tools with "Desktop development with C++"
- CMake 3.20+ (`winget install CMake` or from [cmake.org](https://cmake.org))

```bash
cargo build --release
cargo run --release
cargo run --release -- ./config.lua
```

> If you get a CMake compatibility error, make sure `.cargo/config.toml` exists
> with `[env] CMAKE_POLICY_VERSION_MINIMUM = "3.5"` — this file is checked into
> the repo and handled automatically.

On Windows, Moonglide uses `SendInput` for mouse and keyboard output (no driver
needed) and compiles Lua 5.4 from source. No system-wide dependencies beyond
the prerequisites above.

## Planned Features

- Analog stick to mouse
- Virtual controller
- REPL improvements
- Full touchapd remapping and support
- Flick stick
- Compensate for windows pointer speed
