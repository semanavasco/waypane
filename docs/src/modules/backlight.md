# Backlight Module

The `backlight` module provides reactive access to the current screen backlight level.

It is exposed under `waypane.backlight`.

## Function

- **`level()`** -> `State`: Returns a read-only reactive `State` containing the current backlight percentage (`0` to `100`).

## Behavior

- Reads from Linux sysfs backlight interfaces (`/sys/class/backlight/*`).
- Automatically updates when brightness changes.
- If no supported backlight device is found, it still returns a valid `State` initialized to `0`.

## Example

```lua
local brightness = waypane.backlight.level()

local widget = Container({
  orientation = "horizontal",
  children = {
    ProgressBar({ fraction = brightness:as(function(v) return v / 100 end) }),
    Label({ text = brightness:as(function(v) return string.format("%d%%", v) end) }),
  },
})
```
