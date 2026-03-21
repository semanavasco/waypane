# Hyprland Module

The `hyprland` module provides deep integration with the Hyprland window manager. It allows you to query the state of workspaces, windows, and monitors, as well as dispatch commands and react to compositor events via signals.

## Usage

The Hyprland module is available under the `waypane.hyprland` table.

## Types

The Hyprland module defines several types that are used across its functions and signals.

### `HyprlandWorkspace`

Basic workspace identification, used in signal event data.

| Field  | Type     | Description                              |
| ------ | -------- | ---------------------------------------- |
| `id`   | `number` | The unique identifier for the workspace. |
| `name` | `string` | The name of the workspace.               |

### `HyprlandWorkspaceInfo`

Detailed workspace information, returned by `getWorkspaces()`.

| Field               | Type                | Description                                             |
| ------------------- | ------------------- | ------------------------------------------------------- |
| `workspace`         | `HyprlandWorkspace` | Basic workspace identification (id and name).           |
| `monitor`           | `string`            | The name of the monitor this workspace is on.           |
| `windows`           | `number`            | The number of windows currently on this workspace.      |
| `last_window_title` | `string`            | The title of the last focused window on this workspace. |
| `fullscreen`        | `boolean`           | Whether this workspace is currently in fullscreen mode. |
| `monitor_id`        | `number?`           | The unique identifier of the monitor, if available.     |

### `HyprlandWindow`

Basic window information, used in the `hyprland::active_window_changed` signal data.

| Field   | Type     | Description              |
| ------- | -------- | ------------------------ |
| `title` | `string` | The title of the window. |
| `class` | `string` | The class of the window. |

### `HyprlandActiveWindowInfo`

Detailed window information, returned by `getActiveWindow()`.

| Field           | Type                | Description                                           |
| --------------- | ------------------- | ----------------------------------------------------- |
| `address`       | `string`            | The unique hex address of the window.                 |
| `title`         | `string`            | The title of the active window.                       |
| `initial_title` | `string`            | The initial title of the window when it was created.  |
| `class`         | `string`            | The class of the active window.                       |
| `initial_class` | `string`            | The initial class of the window when it was created.  |
| `pid`           | `number`            | The process ID of the active window.                  |
| `monitor`       | `number?`           | The ID of the monitor the window is on, if available. |
| `workspace`     | `HyprlandWorkspace` | The workspace the window is on.                       |
| `width`         | `number`            | The width of the window in pixels.                    |
| `height`        | `number`            | The height of the window in pixels.                   |
| `x`             | `number`            | The x-coordinate of the window's top-left corner.     |
| `y`             | `number`            | The y-coordinate of the window's top-left corner.     |
| `floating`      | `boolean`           | Whether the window is currently floating.             |
| `fullscreen`    | `boolean`           | Whether the window is in fullscreen mode.             |

### `HyprlandMonitorInfo`

Monitor information, returned by `getMonitors()`.

| Field              | Type                | Description                                         |
| ------------------ | ------------------- | --------------------------------------------------- |
| `id`               | `number`            | The unique identifier for the monitor.              |
| `name`             | `string`            | The name of the monitor, as configured in Hyprland. |
| `focused`          | `boolean`           | Whether this monitor is currently focused.          |
| `width`            | `number`            | The width of the monitor in pixels.                 |
| `height`           | `number`            | The height of the monitor in pixels.                |
| `x`                | `number`            | The x-coordinate of the monitor's top-left corner.  |
| `y`                | `number`            | The y-coordinate of the monitor's top-left corner.  |
| `refresh_rate`     | `number`            | The refresh rate of the monitor in Hz.              |
| `scale`            | `number`            | The UI scale factor for the monitor.                |
| `active_workspace` | `HyprlandWorkspace` | The currently active workspace on this monitor.     |

### `HyprlandActiveMonitor`

Active monitor information, used in the `hyprland::active_monitor_changed` signal data.

| Field       | Type      | Description                                             |
| ----------- | --------- | ------------------------------------------------------- |
| `monitor`   | `string`  | The name of the monitor.                                |
| `workspace` | `string?` | The name of the workspace on the monitor, if available. |

## Functions

### Querying State

- **`getWorkspaces()`** → `HyprlandWorkspaceInfo[]`: Returns a list of all current workspaces.
- **`getActiveWindow()`** → `HyprlandActiveWindowInfo | nil`: Returns information about the currently focused window, or `nil` if no window is focused.
- **`getMonitors()`** → `HyprlandMonitorInfo[]`: Returns a list of all connected monitors.

### Workspace Management

- **`switchWorkspace(id)`**: Switches focus to the workspace with the given numerical ID.
- **`switchWorkspaceRelative(offset)`**: Switches focus to a workspace relative to the current one (e.g., `1` for next, `-1` for previous).
- **`switchWorkspaceNamed(name)`**: Switches focus to the workspace with the given name.
- **`switchToPreviousWorkspace()`**: Switches focus back to the previously active workspace.
- **`toggleSpecialWorkspace(name)`**: Toggles the visibility of a special workspace (scratchpad). If `name` is `nil`, the default special workspace is used.

### Window Management

- **`moveActiveToWorkspace(id)`**: Moves the currently focused window to the workspace with the given ID and switches focus to that workspace.
- **`moveActiveToWorkspaceSilent(id)`**: Moves the focused window to the workspace with the given ID without changing the current workspace focus.
- **`toggleFloating()`**: Toggles the floating state of the active window.
- **`toggleFullscreen()`**: Toggles the fullscreen state of the active window.
- **`killActiveWindow()`**: Closes the currently focused window.

### Example: Workspace Switcher

```lua
local function workspace_button(id)
  return Button({
    child = Label({ text = tostring(id) }),
    on_click = function()
      waypane.hyprland.switchWorkspace(id)
    end,
  })
end
```

## Signals

You can subscribe to these events using `waypane.onSignal()`.

> [!NOTE]
> These signals are emitted by the Hyprland module. Since they are in the `::` namespace, they cannot be manually emitted from Lua using `waypane.emitSignal()`.

### Workspace Events

| Signal                        | Data Type           | Description                                         |
| ----------------------------- | ------------------- | --------------------------------------------------- |
| `hyprland::workspace_changed` | `HyprlandWorkspace` | Fired when focus moves to a different workspace.    |
| `hyprland::workspace_added`   | `HyprlandWorkspace` | Fired when a new workspace is created.              |
| `hyprland::workspace_deleted` | `HyprlandWorkspace` | Fired when a workspace is destroyed.                |
| `hyprland::workspace_moved`   | `HyprlandWorkspace` | Fired when a workspace is moved to another monitor. |
| `hyprland::workspace_renamed` | `HyprlandWorkspace` | Fired when a workspace is renamed.                  |

### Window & Monitor Events

| Signal                             | Data Type               | Description                                              |
| ---------------------------------- | ----------------------- | -------------------------------------------------------- |
| `hyprland::active_window_changed`  | `HyprlandWindow`        | Fired when the focused window changes.                   |
| `hyprland::fullscreen_changed`     | `boolean`               | Fired when the active window's fullscreen state toggles. |
| `hyprland::active_monitor_changed` | `HyprlandActiveMonitor` | Fired when focus moves to a different monitor.           |

## Widgets

The Hyprland module provides specialized widgets that automatically react to compositor events.

### `HyprlandActiveWindowLabelWidget`

A specialized version of the [`Label`](../widgets/label.md) widget that automatically updates its text to show the title of the currently focused window.

It supports all [common widget properties](../widgets/common.md).

#### Example

```lua
local title = HyprlandActiveWindowLabel({
  id = "window-title",
  valign = "center",
})
```

### `HyprlandWsContainerWidget`

A specialized version of the [`Container`](../widgets/container.md) widget that displays a list of Hyprland workspace buttons. It automatically updates its children whenever workspace or monitor state changes.

It supports all [common widget properties](../widgets/common.md).

#### Properties

| Property                | Type          | Description                                                                                                                        |
| ----------------------- | ------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `orientation`           | `Orientation` | The layout orientation of the buttons (`"horizontal"` or `"vertical"`).                                                            |
| `spacing`               | `number`      | The amount of space between buttons in pixels. Default is `0`.                                                                     |
| `monitor`               | `string?`     | An optional monitor name to filter workspaces by. If `nil`, workspaces from all monitors are shown.                                |
| `active_properties`     | `Widget?`     | Optional widget properties to apply to active workspace buttons. **(DO NOT PASS A WIDGET DIRECTLY, ONLY ITS COMMON PROPERTIES)**   |
| `inactive_properties`   | `Widget?`     | Optional widget properties to apply to inactive workspace buttons. **(DO NOT PASS A WIDGET DIRECTLY, ONLY ITS COMMON PROPERTIES)** |
| `persistent_workspaces` | `number[]?`   | A list of workspace IDs that should always be shown, even if they have no windows.                                                 |
| `hide_empty`            | `boolean`     | Whether to hide workspaces that have no windows and are not active. Default is `false`.                                            |

#### Example

```lua
local workspaces = HyprlandWsContainer({
  orientation = "horizontal",
  spacing = 5,
  monitor = "eDP-1", -- Only show workspaces for this monitor
  persistent_workspaces = { 1, 2, 3, 4, 5 }, -- Always show at least these
  active_properties = {
    class_list = { "ws-button", "active" },
  },
  inactive_properties = {
    class_list = { "ws-button" },
  },
})
```
