# Styling

`waypane` uses standard CSS for styling its widgets. This provides a powerful and flexible way to customize the appearance of your desktop environment.

> [!NOTE]
> You can use the `--watch-css` flag to automatically reload the CSS file when it changes, allowing for rapid development and testing of your styles.

## Applying CSS

You can apply CSS by providing a path to a CSS file when creating your shell object. This stylesheet will be applied to all windows.

```lua
local shell = waypane.shell({
  title = "My Bar",
  style = "style.css",
})
```

## Targeting Widgets

Widgets can be targeted in CSS using their IDs or classes.

- **ID**: Target a specific widget with its unique `id` property using `#id`.
- **Class**: Target one or more widgets with their `class_list` property using `.class`.

### Example

In your Lua config:

```lua
local my_label = Label({
  id = "status-label",
  class_list = { "important", "alert" },
  text = "Status: OK",
})
```

In your CSS file:

```css
#status-label {
  color: #fff;
  background-color: #333;
  padding: 5px;
  border-radius: 5px;
}

.important {
  font-weight: bold;
}

.alert {
  color: #ff0000;
}
```

## Supported Properties

Standard CSS properties supported by GTK4 can be used. This includes:

- `color`, `background-color`
- `font-family`, `font-size`, `font-weight`
- `margin`, `padding`
- `border`, `border-radius`
- `min-width`, `min-height`

  ...

## CSS Selectors

You can use complex CSS selectors to target widgets based on their hierarchy.

```css
/* Target all labels inside a container with the class 'my-container' */
.my-container label {
  color: #aaa;
}
```

## Resources

For more information on the specific CSS properties supported by GTK4, refer to the [GTK4 CSS Properties documentation](https://docs.gtk.org/gtk4/css-properties.html).
