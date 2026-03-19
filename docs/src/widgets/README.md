# Widgets

Widgets are the building blocks of your `waypane` UI. Each widget represents a single element, such as a text label, a clickable button, or a container for other widgets.

## Base Widgets

- **[Label](./label.md)**: A simple widget that displays a text label.
- **[Button](./button.md)**: A clickable button widget.
- **[Container](./container.md)**: A container widget that can hold multiple child widgets, arranged either horizontally or vertically.
- **[Icon](./icon.md)**: A widget that displays a GTK icon.
- **[Image](./image.md)**: A widget that displays an image from a file path.
- **[ProgressBar](./progress_bar.md)**: A widget that displays a progress bar.
- **[Slider](./slider.md)**: A widget that allows users to select a value from a range by sliding a handle.
- **[Stack](./stack.md)**: A container widget that shows one child at a time with optional transitions.

## [Common Properties](./common.md)

All widgets share a common set of properties for alignment, visibility, expansion, and more. Understanding these common properties will help you effectively layout and style your UI.

## Creating a Widget Tree

Widgets are defined declaratively in your Lua configuration. You can nest widgets to create complex layouts.

```lua
local my_ui = Container({
  orientation = "vertical",
  spacing = 10,
  children = {
    Label({ text = "Title" }),
    Button({
      child = Label({ text = "Click Me" }),
      on_click = function() print("Button clicked!") end,
    }),
  },
})
```
