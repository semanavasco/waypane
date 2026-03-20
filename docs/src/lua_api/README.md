# Lua API

`waypane` exposes a Lua API that allows you to define your desktop environment's structure, behavior, and appearance. The API is designed to be intuitive and powerful, leveraging Lua's flexibility to provide a seamless development experience.

This section covers the core components of the Lua API:

- **[Shell & Windows](./shell_windows.md)**: Defining the overall structure of your desktop environment, including how to create windows and manage their layout.
- **[Global Functions](./globals.md)**: Functions available in the global Lua scope, primarily for widget creation.
- **[State Management](./state.md)**: How to create and use reactive state to build dynamic UIs.
- **[Timers](./timers.md)**: Working with intervals and timeouts for time-based updates.
- **[Signals](./signals.md)**: Subscribing to and emitting events to react to the desktop environment.
- **[Shell Commands](./commands.md)**: Executing and polling external shell commands.

## The `waypane` Table

Most of the core functionality is organized under the global `waypane` table. This includes functions for creating the shell, managing state, and interacting with modules like Hyprland.

```lua
-- Example usage
local shell = waypane.shell({ title = "My Bar" })
local my_state = waypane.state(0)
```

## Type Hints & Autocompletion

To get the most out of the Lua API, it is highly recommended to use the `gen-stubs` command to generate type definitions for your IDE. This will provide you with autocompletion, type checking, and inline documentation as you write your scripts.

```bash
waypane gen-stubs > stubs.lua
```
