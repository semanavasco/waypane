# State Management

Reactive state is at the heart of `waypane`. It allows you to bind dynamic data points directly to your widgets so they automatically update themselves whenever the underlying data changes.

## The Base Type: `State` (Read-Only)

At its core, all reactive data in `waypane` is represented by a `State` handle. The base `State` type is **read-only** (`:get()`, `:as()`). It is commonly used for data managed by the system or a background module (like your battery level, current workspace, or screen brightness).

You can read the current value of any `State` at any time using the `:get()` method:

```lua
-- Example: Backlight module returns a read-only State for hardware brightness
local brightness = waypane.backlight.level()

-- Read the current hardware value
print(brightness:get()) -- Output: e.g., 50
```

### Binding State to Widgets

The true power of a `State` handle is that you can pass it directly into widget properties instead of static values. When the state changes in the background, the widget property will automatically update without any extra code.

```lua
-- Example: Using a read-only state from backlight
local brightness = waypane.backlight.level()

-- A simple progress bar that automatically tracks your screen brightness
local slider = ProgressBar({
  fraction = brightness,
})
```

### Transforming State

Often, raw state data needs to be formatted before it is displayed. You can create a "derived" state using the `:as()` method. This method takes a function that receives the raw state value and returns the formatted result.

```lua
-- Example: Using a read-only state from backlight
local brightness = waypane.backlight.level()

local label = Label({
  -- Transform the raw integer (50) into a formatted string ("Brightness: 50%")
  text = brightness:as(function(val)
    return string.format("Brightness: %d%%", val)
  end)
})
```

## Creating Custom Data: `MutableState`

While hardware and system modules provide read-only `State` handles, you will often want to create your own reactive variables (like a custom counter, a toggle switch, or parsed script output).

You can create a new, writable state object using the `waypane.state()` function. This returns a `MutableState`.

```lua
-- Create a new mutable state with an initial value of 0
local count = waypane.state(0)
```

Because `MutableState` is just an extension of `State`, it inherits `:get()` and `:as()`, and can be bound to widgets exactly like a read-only state. However, it also gains the `:set()` method, allowing your Lua scripts to update the value and trigger UI redraws.

```lua
local count = waypane.state(0)

local my_label = Label({
  text = count:as(function(c) return "Count: " .. tostring(c) end)
})

-- Later, in a button click or a waypane.setInterval timer...
count:set(count:get() + 1) -- This automatically updates the label's text!
```

> [!NOTE]
> Any widget property that accepts a `State` will seamlessly accept a `MutableState`.

## Advanced: Dynamic Children

Containers can also use reactive state for their `children` property. This is a powerful way to create dynamic lists or grids of widgets that can grow or shrink based on external data.

```lua
-- We use a MutableState here so we can add items to it later
local items = waypane.state({ "A", "B", "C" })

local my_container = Container({
  -- The :as() transform maps the data array into an array of UI Widgets
  children = items:as(function(list)
    local labels = {}
    for _, item in ipairs(list) do
      table.insert(labels, Label({ text = item }))
    end
    return labels
  end),
})

-- Adding an item to the list will automatically rebuild the container's children
local current = items:get()
table.insert(current, "D")
items:set(current)
```

## Combining Multiple States

Sometimes a widget property needs to react to changes from multiple independent states. You can use `waypane.combine({ states... })` to create a new read-only state that aggregates multiple inputs into a single array.

The combined state will update whenever **any** of its input states change.

```lua
local level = waypane.battery.level()
local status = waypane.battery.status()

-- Combine both into a single state handle
local combined = waypane.combine({ level, status })

-- The :as() transform receives an array containing the current values of { level, status }
local label = Label({
  text = combined:as(function(vals)
    local l, s = vals[1], vals[2]
    return string.format("Battery: %d%% (%s)", l, s)
  end)
})
```
