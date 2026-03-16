# Changelog

All notable changes to this project will be documented in this file.

## [0.0.1-alpha.1] - 2026-03-16

### Added

- First public alpha release of `waypane` as an early proof of concept.
- Lua-driven GTK4 widget toolkit foundations (Shell, windows, state, timers, signals).
- Core UI widgets: `Button`, `Container`, `Icon`, `Image`, `Label`, `ProgressBar`, and `Slider`.
- Optional Hyprland integration (workspaces, active window, dispatchers) behind the `hyprland` feature flag.
- Lua stub generation command (`waypane gen-stubs`) and starter examples.
- Basic logging with configurable log levels.
- Comprehensive [documentation](https://semanavasco.com/waypane) for installation, usage, and API reference.

### Known Limitations

- **Experimental Memory Lifecycle:** Because this alpha is focused on testing the core GTK/Lua reactivity engine, automatic garbage collection for dynamically created widgets (e.g., auto-destroying states, timers, signals when a widget unloads, etc) is not yet fully implemented. While perfectly fine for testing sessions, highly dynamic setups may experience poor memory management over extended periods and are not yet recommended for 24/7, daily-driver use.
- **Compositor Support:** Currently only provides native IPC modules for Hyprland.

### Compatibility & Stability

- **Volatile API:** As an early alpha release, the Lua API surface (widget properties, module names, and reactivity mechanics) is subject to heavy breaking changes in upcoming minor versions while the architecture stabilizes. If you build a config, please pin your crate version!
