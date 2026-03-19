# Common Widget Properties

Every widget in `waypane` shares a set of common properties that you can use to customize its appearance, layout, and behavior. These properties are always available when you're defining a widget.

## Basic Properties

| Property     | Type                          | Description                                   |
| ------------ | ----------------------------- | --------------------------------------------- |
| `id`         | `string \| State<string>`     | Widget ID, used for CSS styling and querying. |
| `class_list` | `string[] \| State<string[]>` | List of CSS classes applied to the widget.    |

## Layout Properties

| Property         | Type                                                            | Default | Description                                                    |
| ---------------- | --------------------------------------------------------------- | ------- | -------------------------------------------------------------- |
| `halign`         | `"start" \| "center" \| "end" \| "fill" \| "baseline" \| State` |         | Horizontal alignment for the widget.                           |
| `valign`         | `"start" \| "center" \| "end" \| "fill" \| "baseline" \| State` |         | Vertical alignment for the widget.                             |
| `hexpand`        | `boolean \| State<boolean>`                                     | `false` | Whether the widget expands to fill available horizontal space. |
| `vexpand`        | `boolean \| State<boolean>`                                     | `false` | Whether the widget expands to fill available vertical space.   |
| `margins`        | [`Margins`](../lua_api/shell_windows.md#margins)` \| State`     |         | Margins around the widget.                                     |
| `width_request`  | `number \| State<number>`                                       | `-1`    | Width request for the widget.                                  |
| `height_request` | `number \| State<number>`                                       | `-1`    | Height request for the widget.                                 |

## Behavior Properties

| Property    | Type                        | Default | Description                                                                         |
| ----------- | --------------------------- | ------- | ----------------------------------------------------------------------------------- |
| `visible`   | `boolean \| State<boolean>` | `true`  | Whether the widget is visible.                                                      |
| `focusable` | `boolean \| State<boolean>` | `false` | Whether the widget can receive keyboard focus.                                      |
| `tooltip`   | `string \| State<string>`   |         | Tooltip markup text for the widget.                                                 |
| `sensitive` | `boolean \| State<boolean>` | `true`  | Whether the widget should be sensitive to user input.                               |
| `on_scroll` | `function(dx, dy)`          |         | Function to execute when scrolling over the widget. Receives (dx, dy) as arguments. |

## CSS Styling

All widgets can be styled using standard CSS. You can target them by their ID (`#my-widget`) or their classes (`.my-class`).

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
}

.important {
  font-weight: bold;
}

.alert {
  color: #ff0000;
}
```
