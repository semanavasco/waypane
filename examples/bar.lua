-- Example status bar using waypane

-- widgets -------------------------------------------------------------------

local function clock_widget()
  local time_state = waypane.state(os.date("%H:%M"))

  waypane.setInterval(function()
    time_state:set(os.date("%H:%M"))
  end, 1000)

  return Label({
    text = time_state,
    id = "clock",
    valign = "center",
  })
end

local function date_widget()
  local date_state = waypane.state(os.date("%a %d %b"))
  local tooltip_state = waypane.state("")

  local function build_calendar_tooltip()
    local now = os.date("*t")
    local year, month, today = now.year, now.month, now.day

    local tz = os.date("%Z")
    local day_of_year = os.date("%j")
    local week_num = os.date("%V")

    local info_str = string.format(
      "<span size='small'>Timezone: <span foreground='#cdd6f4'>%s</span>\n"
        .. "Day: <span foreground='#cba6f7'>%s</span>/365 | "
        .. "Week: <span foreground='#fab387'>%s</span>/52</span>\n\n",
      tz,
      day_of_year,
      week_num
    )

    local days_in_month = os.date("*t", os.time({ year = year, month = month + 1, day = 0 })).day
    local first_day_wday = tonumber(os.date("%w", os.time({ year = year, month = month, day = 1 })))
    local start_col = first_day_wday == 0 and 7 or first_day_wday

    local cal_str = "<span font_family='JetBrains Mono, monospace'>\n"
    cal_str = cal_str .. "<span foreground='#cba6f7' font_weight='bold'>Mo Tu We Th Fr Sa Su</span>\n"

    for _ = 1, start_col - 1 do
      cal_str = cal_str .. "   "
    end

    for d = 1, days_in_month do
      local day_str = string.format("%2d", d)

      if d == today then
        day_str = "<span foreground='#a6e3a1' font_weight='bold'><u>" .. day_str .. "</u></span>"
      end

      cal_str = cal_str .. day_str

      if (start_col + d - 1) % 7 == 0 and d ~= days_in_month then
        cal_str = cal_str .. "\n"
      elseif d ~= days_in_month then
        cal_str = cal_str .. " "
      end
    end

    cal_str = cal_str .. "</span>"

    return info_str .. cal_str
  end

  tooltip_state:set(build_calendar_tooltip())

  waypane.setInterval(function()
    date_state:set(os.date("%a %d %b"))
    tooltip_state:set(build_calendar_tooltip())
  end, 60000)

  return Label({
    text = date_state,
    id = "date",
    valign = "center",
    tooltip = tooltip_state,
  })
end

local function backlight_widget()
  local brightness = waypane.backlight.level()
  return Label({
    text = brightness:as(function(v)
      return string.format("󰃠 %d%%", v)
    end),
    id = "backlight",
    valign = "center",
    on_scroll = function(_, dy)
      if dy < 0 then
        waypane.exec("brightnessctl set +2%")
      elseif dy > 0 then
        waypane.exec("brightnessctl set 2%-")
      end
    end,
  })
end

local function battery_widget()
  local function icon(status, battery)
    if status == "charging" then
      return "󰂄"
    elseif status == "discharging" then
      local icons = { "󰁺", "󰁻", "󰁼", "󰁽", "󰁾", "󰁿", "󰂀", "󰂁", "󰂂", "󰁹" }
      local index = math.max(1, math.min(10, math.ceil(battery / 10)))
      return icons[index]
    else
      return "󰂎"
    end
  end

  local battery = waypane.battery.level("BAT0")
  local status = waypane.battery.status("BAT0")

  return Label({
    text = battery:as(function(v)
      return string.format("%s %d%%", icon(status:get(), v), v)
    end),
    id = "battery",
    valign = "center",
  })
end

local function spacer()
  return Container({ orientation = "horizontal", hexpand = true })
end

-- bar layout ----------------------------------------------------------------

local shell = waypane.shell({
  title = "Bar",
  style = "bar.css",
})

shell:window("main-bar", {
  layer = "top",
  exclusive_zone = true,
  anchors = { top = true, left = true, right = true },

  layout = function(monitor)
    local children = {
      HyprlandWsContainer({
        orientation = "horizontal",
        monitor = monitor.name,
        active_properties = {
          class_list = { "ws-active", "ws-btn" },
          sensitive = false,
          valign = "center",
        },
        inactive_properties = {
          class_list = { "ws-inactive", "ws-btn" },
          valign = "center",
        },
      }),
      HyprlandActiveWindowLabel({
        id = "window-title",
        valign = "center",
      }),
      spacer(),
      backlight_widget(),
    }

    if waypane.battery.is_present() then
      table.insert(children, battery_widget())
    end

    table.insert(children, clock_widget())
    table.insert(children, date_widget())

    return Container({
      id = "bar",
      orientation = "horizontal",
      spacing = 8,
      valign = "center",
      children = children,
    })
  end,
})

return shell
