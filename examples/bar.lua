-- Example status bar using waypane

-- state ---------------------------------------------------------------------

local function create_state(monitor)
  local active_workspace = waypane.state(1)
  local active_window_title = waypane.state("")

  local function update_active_workspace()
    local monitors = waypane.hyprland.getMonitors() or {}
    for _, monitor_info in ipairs(monitors) do
      if monitor_info.focused then
        active_workspace:set(monitor_info.active_workspace.id)
        return
      end
    end
  end

  local function load_initial_state()
    update_active_workspace()

    local window = waypane.hyprland.getActiveWindow() or {}
    if window and window.title then
      active_window_title:set(window.title)
    end
  end

  load_initial_state()

  return {
    monitor = monitor,
    active_workspace = active_workspace,
    active_window_title = active_window_title,
  },
    update_active_workspace
end

-- widgets -------------------------------------------------------------------

local function workspace_button(id, is_active)
  return Button({
    child = Label({
      text = tostring(id),
      class_list = { is_active and "ws-active" or "ws-inactive" },
      valign = "center",
    }),
    sensitive = not is_active,
    class_list = { "ws-btn" },
    valign = "center",
    focusable = false,
    on_click = function()
      waypane.hyprland.switchWorkspace(id)
    end,
  })
end

local function workspaces_widget(state, update_active_workspace)
  local children_state = waypane.state({})

  local function rebuild_workspaces()
    update_active_workspace()

    local workspaces = waypane.hyprland.getWorkspaces() or {}
    table.sort(workspaces, function(a, b)
      return a.id < b.id
    end)

    local btns = {}
    for _, ws_info in ipairs(workspaces) do
      local on_my_monitor = not state.monitor or ws_info.monitor == state.monitor.name

      if on_my_monitor then
        local id = ws_info.id
        if type(id) == "number" and id > 0 then
          table.insert(btns, workspace_button(id, id == state.active_workspace:get()))
        end
      end
    end
    children_state:set(btns)
  end

  -- Rebuild whenever workspace events fire
  waypane.onSignal({
    "hyprland::workspace_changed",
    "hyprland::workspace_added",
    "hyprland::workspace_deleted",
    "hyprland::workspace_moved",
    "hyprland::workspace_renamed",
    "hyprland::active_monitor_changed",
  }, function()
    rebuild_workspaces()
  end)

  -- Initial build
  rebuild_workspaces()

  return Container({
    id = "workspaces",
    orientation = "horizontal",
    spacing = 4,
    valign = "center",
    children = children_state,
    on_scroll = function(_, dy)
      if dy < 0 then
        waypane.hyprland.switchWorkspaceRelative(-1)
      else
        waypane.hyprland.switchWorkspaceRelative(1)
      end
    end,
  })
end

local function title_widget(state)
  waypane.onSignal("hyprland::active_window_changed", function(window)
    if window and window.title then
      state.active_window_title:set(window.title)
    end
  end)

  return Label({
    text = state.active_window_title,
    id = "window-title",
    valign = "center",
  })
end

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
    local state, update_active_workspace = create_state(monitor)

    return Container({
      id = "bar",
      orientation = "horizontal",
      spacing = 8,
      valign = "center",
      children = {
        workspaces_widget(state, update_active_workspace),
        title_widget(state),
        spacer(),
        clock_widget(),
        date_widget(),
      },
    })
  end,
})

return shell
