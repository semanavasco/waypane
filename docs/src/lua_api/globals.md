# Global Functions

`waypane` provides a set of global functions that are primarily used to construct widgets. These functions are available directly in your Lua configuration file without any prefixes.

## Widget Constructors

Each widget constructor takes a single table as an argument, which defines its properties and behavior.

### `Label(props)`

Creates a new `Label` widget for displaying text.
[See Label for more details](../widgets/label.md).

### `Button(props)`

Creates a clickable `Button` widget.
[See Button for more details](../widgets/button.md).

### `Container(props)`

Creates a `Container` widget for organizing other widgets.
[See Container for more details](../widgets/container.md).

### `Icon(props)`

Creates an `Icon` widget for displaying GTK icons.
[See Icon for more details](../widgets/icon.md).

### `Image(props)`

Creates an `Image` widget for displaying images from file paths.
[See Image for more details](../widgets/image.md).

### `ProgressBar(props)`

Creates a `ProgressBar` widget for visualizing progress.
[See ProgressBar for more details](../widgets/progress_bar.md).

### `Slider(props)`

Creates a `Slider` widget for selecting a value from a range.
[See Slider for more details](../widgets/slider.md).

### `Stack(props)`

Creates a `Stack` widget for holding multiple children and showing one at a time.
[See Stack for more details](../widgets/stack.md).

## Usage Example

```lua
local my_label = Label({
  text = "Hello, waypane!",
  halign = "center",
})

local my_button = Button({
  child = my_label,
  on_click = function()
    print("Button clicked!")
  end,
})
```
