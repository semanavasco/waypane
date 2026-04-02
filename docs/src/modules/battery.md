# Battery Module

The `battery` module provides reactive access to battery-related information, such as charge level, status, power draw, and more.

It is exposed under `waypane.battery`.

### `BatteryStatus`

Battery status as reported by sysfs or aggregated across multiple batteries.

| Value          | Description                                                        |
| -------------- | ------------------------------------------------------------------ |
| `charging`     | The battery is charging                                            |
| `discharging`  | The battery is discharging                                         |
| `full`         | The battery is full                                                |
| `not-charging` | The battery is not charging (e.g. battery save mode)               |
| `plugged`      | The AC adapter is online but the battery reports an unknown status |
| `unknown`      | The battery reports an unknown status                              |

## Functions

All functions in the `battery` module take an optional `name` parameter to specify which battery to query (e.g. "BAT0"). If `name` is not provided, results will reflect the aggregate data of all detected batteries.

- **`is_present(name?)`** -> `boolean`: Returns `true` if a battery device is detected on the system.
- **`level(name?)`** -> `State`: Returns a read-only reactive `State` containing the current battery percentage (`0` to `100`).
- **`status(name?)`** -> `State`: Returns a read-only reactive `State` containing the current battery status. See `BatteryStatus` for possible values.
- **`power(name?)`** -> `State`: Returns a read-only reactive `State` containing the current power draw in Watts.
- **`time_remaining(name?)`** -> `State`: Returns a read-only reactive `State` containing the estimated time remaining until full/empty in hours.
- **`cycles(name?)`** -> `State`: Returns a read-only reactive `State` containing the battery cycle count.
- **`health(name?)`** -> `State`: Returns a read-only reactive `State` containing the battery health percentage (`0` to `100`).
- **`energy(name?)`** -> `State`: Returns a read-only reactive `State` containing the current energy in Wh.
- **`voltage(name?)`** -> `State`: Returns a read-only reactive `State` containing the current voltage in Volts.

## Behavior

- Reads from Linux sysfs power supply interfaces (`/sys/class/power_supply/*`).
- Automatically updates when battery events occur (via `uevent` watcher).
- Provides a fallback periodic refresh every 30 seconds.
- Aggregates data across multiple batteries by default.

## Example

```lua
local level = waypane.battery.level()
local status = waypane.battery.status()

local function battery_widget()
  return Label({
    text = level:as(function(l)
      return string.format("󰁹 %d%%", l)
    end),
    tooltip = status,
    valign = "center",
  })
end

-- Only show the widget if a battery is present
if waypane.battery.is_present() then
  table.insert(bar_children, battery_widget())
end
```
