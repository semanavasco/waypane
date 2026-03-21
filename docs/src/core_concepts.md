# Core Concepts

waypane is built around a few fundamental concepts that work together to create a dynamic desktop environment.

## The Shell

The `Shell` is the top-level object in your configuration. It manages global settings like the application title and any custom CSS styles you want to apply. It also acts as a container for your window definitions.

A typical configuration returns a `Shell` object at the end of the file.

```lua
local shell = waypane.shell({
  title = "My Desktop",
  style = "style.css",
})

-- Define windows here

return shell
```

## Windows

A `Window` represents a single UI element on your desktop, such as a status bar, a dashboard, or a standalone widget. Windows are defined using the `shell:window` method and specify their layer, placement, and layout.

Windows can be configured to appear on specific monitors or all of them. Their layout is defined by providing a main Widget, or a function that returns one.

## Widgets

`Widgets` are the building blocks of your UI. waypane provides a variety of built-in widgets like `Label`, `Button`, `Container`, `Icon`, and more. Widgets are declarative, meaning you describe their state and properties, and waypane handles the rendering and updates.

Many widget properties can be **reactive**, meaning they automatically update when the underlying data changes.

## Reactive State

`State` is the parent handle type for waypane's reactive bindings. By using `waypane.state()`, you create a `MutableState` (a subtype of `State`) that widgets can subscribe to. When the state changes, any widget property bound to that state will automatically refresh.

```lua
local time = waypane.state(os.date("%H:%M"))

-- Later, when `time`'s internal value changes, the label will update automatically
local my_label = Label({
  text = time,
})
```

## Signals and Events

`Signals` are a way to react to asynchronous events, such as workspace changes in Hyprland or custom events emitted by your own scripts. You can use `waypane.onSignal()` to register callbacks for specific signals, allowing your UI to respond to the desktop environment in real-time.
