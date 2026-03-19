use super::{utils::call_dispatch, workspaces::Workspace};
use hyprland::{
    data::{Client, FullscreenMode},
    dispatch::{DispatchType, FullscreenType},
    shared::HyprDataActiveOptional,
};
use mlua::{IntoLua, Lua, Value as LuaValue};
use waypane_macros::{LuaClass, lua_func};

/// Basic information about a window, including its title and class.
#[derive(LuaClass)]
#[lua_class(name = "HyprlandWindow")]
pub struct Window {
    /// The title of the window.
    pub title: String,
    /// The class of the window.
    pub class: String,
}

impl IntoLua for Window {
    fn into_lua(self, lua: &Lua) -> mlua::Result<LuaValue> {
        let table = lua.create_table()?;
        table.set("title", self.title)?;
        table.set("class", self.class)?;
        Ok(LuaValue::Table(table))
    }
}

/// Information about the currently active window in Hyprland, including its title, class, PID,
/// monitor, workspace, position, size, and other states.
#[derive(LuaClass)]
#[lua_class(name = "HyprlandActiveWindowInfo")]
pub struct ActiveWindowInfo {
    /// The unique hex address of the window.
    pub address: String,
    /// The title of the active window.
    pub title: String,
    /// The initial title of the window when it was first created.
    pub initial_title: String,
    /// The class of the active window.
    pub class: String,
    /// The initial class of the window when it was first created.
    pub initial_class: String,
    /// The process ID of the active window.
    pub pid: i32,
    /// The ID of the monitor the active window is on, if available.
    pub monitor: Option<i128>,
    /// Basic information about the workspace the active window is on.
    pub workspace: Workspace,
    /// The width of the window in pixels.
    pub width: i16,
    /// The height of the window in pixels.
    pub height: i16,
    /// The x-coordinate of the window's top-left corner.
    pub x: i16,
    /// The y-coordinate of the window's top-left corner.
    pub y: i16,
    /// Whether the window is currently floating.
    pub floating: bool,
    /// Whether the window is currently in fullscreen mode.
    pub fullscreen: bool,
}

impl IntoLua for ActiveWindowInfo {
    fn into_lua(self, lua: &Lua) -> mlua::Result<LuaValue> {
        let table = lua.create_table()?;
        table.set("address", self.address)?;
        table.set("title", self.title)?;
        table.set("initial_title", self.initial_title)?;
        table.set("class", self.class)?;
        table.set("initial_class", self.initial_class)?;
        table.set("pid", self.pid)?;
        table.set("monitor", self.monitor)?;

        let workspace = self.workspace.into_lua(lua)?;
        table.set("workspace", workspace)?;

        table.set("width", self.width)?;
        table.set("height", self.height)?;

        table.set("x", self.x)?;
        table.set("y", self.y)?;

        table.set("floating", self.floating)?;
        table.set("fullscreen", self.fullscreen)?;

        Ok(LuaValue::Table(table))
    }
}

/// Returns information about the currently active window in Hyprland, including its title, class,
/// PID, monitor, workspace, position, and size. If there is no active window, a nil value is
/// returned.
#[lua_func(name = "getActiveWindow", module = "waypane.hyprland", skip = "lua")]
#[ret(
    doc = "active_window A table containing information about the currently active window, or nil if there is no active window.",
    ty = "HyprlandActiveWindowInfo | nil"
)]
pub fn get_active_window() -> mlua::Result<Option<ActiveWindowInfo>> {
    let active_window = Client::get_active()
        .map_err(|e| mlua::Error::external(format!("Failed to get active window: {}", e)))?;

    let window = active_window.as_ref().map(|w| ActiveWindowInfo {
        address: w.address.to_string(),
        title: w.title.clone(),
        initial_title: w.initial_title.clone(),
        class: w.class.clone(),
        initial_class: w.initial_class.clone(),
        pid: w.pid,
        monitor: w.monitor,
        workspace: Workspace {
            id: w.workspace.id,
            name: w.workspace.name.clone(),
        },
        x: w.at.0,
        y: w.at.1,
        width: w.size.0,
        height: w.size.1,
        floating: w.floating,
        fullscreen: w.fullscreen != FullscreenMode::None,
    });

    Ok(window)
}

/// Toggles the floating state of the currently active window in Hyprland.
#[lua_func(name = "toggleFloating", module = "waypane.hyprland")]
pub fn toggle_floating() -> mlua::Result<()> {
    call_dispatch(DispatchType::ToggleFloating(None), "toggle floating")
}

/// Toggles the fullscreen state of the currently active window in Hyprland.
#[lua_func(name = "toggleFullscreen", module = "waypane.hyprland")]
pub fn toggle_fs() -> mlua::Result<()> {
    call_dispatch(
        DispatchType::ToggleFullscreen(FullscreenType::NoParam),
        "toggle fullscreen",
    )
}

/// Closes the currently active window in Hyprland.
#[lua_func(name = "killActiveWindow", module = "waypane.hyprland")]
pub fn kill_active_win() -> mlua::Result<()> {
    call_dispatch(DispatchType::KillActiveWindow, "kill active window")
}
