# waypane

waypane is a Wayland toolkit made in Rust for building custom desktop widgets and bars in Lua. It provides a simple API to create and manage GTK-based widgets, handle events, and interact with the Wayland compositor.

Documentation including **installation instructions**, **API reference**, etc, can be found [here](https://semanavasco.com/waypane).

## Installation

`waypane` is published as an early alpha release (`0.0.1-alpha.1`).

Install from crates.io:

```bash
cargo install waypane --version 0.0.1-alpha.1
```

Or, with a module integration (e.g., Hyprland):

```bash
cargo install waypane --version 0.0.1-alpha.1 --features hyprland
```

## Usage

Run with a configuration file:

```bash
waypane run examples/bar.lua
```

Generate Lua stubs for better IDE integration (e.g., with Lua Language Server):

```bash
waypane gen-stubs > stubs.lua
```

## Examples

The [`examples/`](./examples) directory contains a few examples of what you can build with waypane:

- `bar.lua`: A complete status bar featuring workspaces, window titles, and a clock.
- `clock.lua`: A simple background clock widget.

## Feedback

This is one of my first Rust projects and I'm actively learning! I'm open to suggestions, code reviews, and constructive criticism. Feel free to open issues. I'd appreciate if you'd let me fix them rather than opening PRs with written solutions. Thank you!

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
