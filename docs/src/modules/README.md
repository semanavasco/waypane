# Modules

`waypane` can be extended with modules that provide integration with external tools and services. These modules typically expose a set of Lua functions and signals.

## Built-in Modules

- **[Backlight](./backlight.md)**: Reactive access to the current screen brightness as a `State`.
- **[Hyprland](./hyprland.md)**: Deep integration with the Hyprland window manager, allowing you to react to workspace changes, window focus, and more.

## Using Modules

Modules are available as sub-tables of the global `waypane` table.

```lua
-- Using the Backlight module
local brightness = waypane.backlight.level()

-- Using the Hyprland module
local workspaces = waypane.hyprland.getWorkspaces()
```

## Community Modules

In the future, `waypane` may support a plugin system or a community-driven set of modules. If you're interested in developing your own module, check out the source code of the Hyprland module as a reference.
