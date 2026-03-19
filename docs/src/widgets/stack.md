# Stack

A container widget that can hold multiple child widgets, but only shows one at a time. It allows for animated transitions when switching between pages.

## Properties

In addition to the [Common Properties](./common.md), `Stack` has the following properties:

| Property              | Type                                               | Default      | Description                                                                                    |
| --------------------- | -------------------------------------------------- | ------------ | ---------------------------------------------------------------------------------------------- |
| `pages`               | [`StackPage[]`](#stackpage)                        | **required** | A list of pages contained in the stack.                                                        |
| `visible_page`        | `string \| State<string>`                          | **required** | The name of the currently visible page.                                                        |
| `transition_type`     | `StackTransitionType\| State<StackTransitionType>` | `"none"`     | The type of animation used when switching between pages. [See below](#transition-types).       |
| `transition_duration` | `number \| State<number>`                          | `200`        | The duration of the transition animation in milliseconds.                                      |
| `interpolate_size`    | `boolean \| State<boolean>`                        | `false`      | Whether the stack should interpolate its size when switching between pages of different sizes. |

### StackPage

A table representing a single page in the stack.

| Field    | Type     | Description                                       |
| -------- | -------- | ------------------------------------------------- |
| `name`   | `string` | The unique name of the page, used to identify it. |
| `widget` | `Widget` | The widget to display when this page is active.   |

### Stack Transition Types

The following transition types are available as strings:

- `"none"` (default)
- `"crossfade"`
- `"slide-right"`, `"slide-left"`, `"slide-up"`, `"slide-down"`
- `"slide-left-right"`, `"slide-up-down"`
- `"over-up"`, `"over-down"`, `"over-left"`, `"over-right"`
- `"under-up"`, `"under-down"`, `"under-left"`, `"under-right"`
- `"over-up-down"`, `"over-left-right"`
- `"rotate-left"`, `"rotate-right"`, `"rotate-left-right"`

## Examples

### Basic Tab-like Switcher

```lua
local active_page = waypane.state("page1")

local my_stack = Stack({
  visible_page = active_page,
  transition_type = "slide-left-right",
  transition_duration = 300,
  pages = {
    {
      name = "page1",
      widget = Label({ text = "This is Page 1" }),
    },
    {
      name = "page2",
      widget = Label({ text = "Welcome to Page 2" }),
    },
  },
})

local controls = Container({
  orientation = "horizontal",
  spacing = 10,
  children = {
    Button({
      child = Label({ text = "Go to Page 1" }),
      on_click = function() active_page:set("page1") end,
    }),
    Button({
      child = Label({ text = "Go to Page 2" }),
      on_click = function() active_page:set("page2") end,
    }),
  }
})
```
