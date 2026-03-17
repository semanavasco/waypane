use super::workspaces::Workspace;
use hyprland::{
    data::Monitors,
    shared::{HyprData, HyprDataVec},
};
use mlua::{IntoLua, Lua, Value as LuaValue};
use waypane_macros::{LuaClass, lua_func};

/// Basic information about an active monitor, including its name and the name of the active
/// workspace.
#[derive(LuaClass)]
#[lua_class(name = "HyprlandActiveMonitor")]
pub struct ActiveMonitor {
    /// The name of the monitor.
    pub monitor: String,
    /// The name of the workspace on the monitor, if available.
    pub workspace: Option<String>,
}

impl IntoLua for ActiveMonitor {
    fn into_lua(self, lua: &Lua) -> mlua::Result<LuaValue> {
        let table = lua.create_table()?;
        table.set("monitor", self.monitor)?;
        if let Some(ws) = self.workspace {
            table.set("workspace", ws)?;
        }
        Ok(LuaValue::Table(table))
    }
}

/// Information about a monitor in Hyprland, including its ID, name, resolution,
/// position, and currently active workspace.
#[derive(LuaClass)]
#[lua_class(name = "HyprlandMonitorInfo")]
pub struct MonitorInfo {
    /// The unique identifier for the monitor.
    pub id: i128,
    /// The name of the monitor, as configured in Hyprland.
    pub name: String,
    /// Whether this monitor is currently focused (i.e., has the active workspace).
    pub focused: bool,
    /// The width of the monitor in pixels.
    pub width: i32,
    /// The height of the monitor in pixels.
    pub height: i32,
    /// The x-coordinate of the monitor's top-left corner.
    pub x: i32,
    /// The y-coordinate of the monitor's top-left corner.
    pub y: i32,
    /// The refresh rate of the monitor in Hz.
    pub refresh_rate: f32,
    /// The UI scale factor for the monitor.
    pub scale: f32,
    /// Basic information about the currently active workspace on this monitor.
    pub active_workspace: Workspace,
}

impl IntoLua for MonitorInfo {
    fn into_lua(self, lua: &Lua) -> mlua::Result<LuaValue> {
        let table = lua.create_table()?;
        table.set("id", self.id)?;
        table.set("name", self.name)?;
        table.set("focused", self.focused)?;
        table.set("width", self.width)?;
        table.set("height", self.height)?;
        table.set("x", self.x)?;
        table.set("y", self.y)?;
        table.set("refresh_rate", self.refresh_rate)?;
        table.set("scale", self.scale)?;
        table.set("active_workspace", self.active_workspace.into_lua(lua)?)?;
        Ok(LuaValue::Table(table))
    }
}

/// Returns a list of Hyprland monitors, including their IDs, names, and focus status.
#[lua_func(name = "getMonitors", module = "waypane.hyprland", skip = "lua")]
#[ret(doc = "monitors A list of information about all connected monitors.")]
pub fn get_monitors() -> mlua::Result<Vec<MonitorInfo>> {
    let monitors = Monitors::get()
        .map_err(|e| mlua::Error::external(format!("Failed to get monitors: {}", e)))?
        .to_vec();

    let monitor_infos: Vec<MonitorInfo> = monitors
        .into_iter()
        .map(|m| MonitorInfo {
            id: m.id,
            name: m.name,
            focused: m.focused,
            width: m.width as i32,
            height: m.height as i32,
            x: m.x,
            y: m.y,
            refresh_rate: m.refresh_rate,
            scale: m.scale,
            active_workspace: Workspace {
                id: m.active_workspace.id,
                name: m.active_workspace.name.clone(),
            },
        })
        .collect();

    Ok(monitor_infos)
}
