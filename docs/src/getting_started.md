# Getting Started

This guide will help you set up and run your first waypane configuration.

`waypane` is available on crates.io as an early alpha (`0.0.1-alpha.1`).

## Prerequisites

Before building waypane, ensure you have the following installed:

- **Rust**: The Rust toolchain (use [rustup](https://rustup.rs/) to install).
- **GTK4**: The development libraries for GTK4.
- **gtk4-layer-shell**: The development libraries for gtk4-layer-shell.
- **Lua**: The Lua interpreter and development libraries (`lua5.4`).
- **Wayland**: A Wayland compositor (Hyprland is recommended for full feature support).

> [!NOTE]
> The exact package names may vary depending on your Linux distribution. Please refer to your distribution's package manager for the correct names.

### Building

Start by cloning the repository and navigating into it:

```bash
git clone https://github.com/semanavasco/waypane.git
```

```bash
cd waypane
```

Then, build the project using Cargo:

```bash
cargo build --release
```

Or with the **-\-features** flag to enable Hyprland integration:

```bash
cargo build --release --features hyprland
```

### Installation

You can install waypane system-wide from crates.io:

```bash
cargo install waypane --version 0.0.1-alpha.1 --features hyprland
```

Or install the base crate without any module integration:

```bash
cargo install waypane --version 0.0.1-alpha.1
```

## Running your First Widget

waypane requires a Lua configuration file to define its UI and behavior. You can run it by passing the path to your config:

```bash
waypane run examples/bar.lua
```

You can also set the log level using the `-l` or `--log-level` flag (defaults to `info`):

```bash
waypane run examples/bar.lua --log-level debug
```

Available log levels are: `error`, `warn`, `info`, `debug`, and `trace`. Pass nonsense value to disable logging entirely (e.g. `none`).

## IDE Integration (Recommended)

To get autocompletion and type hints in your Lua editor, generate a stubs file:

```bash
waypane gen-stubs > stubs.lua
```

Place this file in your project directory. If you're using the [Lua Language Server](https://luals.github.io/), it will automatically pick up the definitions.

> [!NOTE]
> You may need an additional `.luarc.json` file to supress warnings about undefined globals. Here's an example:
>
> ```json
> {
>   "diagnostics": {
>     "globals": ["Label", "Container", "Button", ..., "waypane"]
>   }
> }
> ```

## Basic Config Structure

A minimal waypane configuration file (`config.lua`) looks like this:

```lua
local shell = waypane.shell({
  title = "My Bar",
})

shell:window("main-window", {
  layer = "top",
  anchors = { top = true, left = true, right = true },

  layout = Label({
    text = "Hello, waypane!",
  })
})

return shell
```

Save this to a file and run it with waypane to see your first widget in action!
