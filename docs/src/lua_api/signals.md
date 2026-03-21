# Signals

`waypane` uses a signal-based event system that allows your Lua scripts to react to events from the desktop environment (like Hyprland workspace changes) or to communicate between different parts of your configuration.

## `waypane.onSignal(signals, callback)`

Listens for one or more signals and calls the provided callback when they are emitted.

- **signals** (`string | string[]`): The name or names of the signals to listen for.
- **callback** (`function(data)`): The function to execute when a signal is emitted. Receives any data passed with the signal.
- **Returns**: A `CancelHandle`.

**Example:**

```lua
local handle = waypane.onSignal("hyprland::workspace_changed", function(workspace)
  print("Active workspace changed to: " .. workspace.name)
end)

-- Stop listening later:
handle:cancel()

-- You can also listen for multiple signals at once
waypane.onSignal({ "event_a", "event_b" }, function(data)
  print("Signal emitted: " .. tostring(data))
end)
```

## `waypane.emitSignal(signal, data)`

Emits a custom signal with the given name and an optional data payload.

- **signal** (`string`): The name of the signal to emit.
- **data** (`any`): Optional data to include with the signal. Can be any Lua value.

> [!IMPORTANT]
> Signals in the `::` namespace are reserved for native module events (e.g., `hyprland::workspace_changed`) and cannot be emitted from Lua. Attempting to do so will result in an error.

**Example:**

```lua
-- Emit a custom signal
waypane.emitSignal("my_event", { message = "Hello from Lua!" })

-- Elsewhere in your script...
waypane.onSignal("my_event", function(payload)
  print(payload.message)
end)
```

## CancelHandle

The `CancelHandle` returned by `onSignal` has a single method:

- **`:cancel()`**: Unsubscribes the callback from all signals it was listening to.

  _If already cancelled, it does nothing._

## Built-in Signals

Modules (like Hyprland), may emit signals automatically. For a full list of these, see the [Hyprland Module](../modules/hyprland.md) documentation.
