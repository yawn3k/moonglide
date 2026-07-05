# Moonglide

JoyShockMapper-inspired controller remapper, configured with Lua. Gyro + mouse/keyboard output.

> **Early, AI-generated, probably buggy.** This whole thing was hacked together with LLM help and duct tape. Expect things to break. PRs and issues welcome.

## Docs

| What | Where |
|------|-------|
| Quick setup | [docs/getting-started.md](docs/getting-started.md) |
| Config, CLI & REPL | [docs/configuration.md](docs/configuration.md) |
| How bindings work | [docs/bindings.md](docs/bindings.md) |
| Gyro setup | [docs/gyro.md](docs/gyro.md) |
| Sticks, rings, triggers | [docs/sticks.md](docs/sticks.md) |
| All the button names | [docs/keys.md](docs/keys.md) |

## License

GNU General Public License v3.0. See [LICENSE](LICENSE).

## Planned Features

- **Complementary filter** — use accelerometer data to stop gyro drift when you're holding still
- **Analog stick → mouse** — map right stick as a mouse
- **Virtual controller** — re-add uinput gamepad output (had to rip it out because it broke controller detection)
- **Full hot-reload** — REPL only re-reads some globals, not everything
- **Integration tests** — we test config parsing but not much else
- **Touchpad output** — turn the DualSense touchpad into a mouse/scroll region
