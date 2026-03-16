use hyprland::{
    data::Workspaces,
    dispatch::{DispatchType, WorkspaceIdentifierWithSpecial},
    shared::{HyprData, HyprDataVec},
};
use mlua::{IntoLua, Lua, Value as LuaValue};
use waypane_macros::{LuaClass, lua_func};

use super::utils::call_dispatch;

/// Basic information about a workspace, including its ID and name.
#[derive(LuaClass)]
#[lua_class(name = "HyprlandWorkspace")]
pub struct Workspace {
    /// The unique identifier for the workspace.
    pub id: i32,
    /// The name of the workspace.
    pub name: String,
}

impl IntoLua for Workspace {
    fn into_lua(self, lua: &Lua) -> mlua::Result<LuaValue> {
        let table = lua.create_table()?;
        table.set("id", self.id)?;
        table.set("name", self.name)?;
        Ok(LuaValue::Table(table))
    }
}

/// Detailed information about a workspace in Hyprland, including its ID, name,
/// associated monitor, number of windows, last focused window title, and whether it is fullscreen.
#[derive(LuaClass)]
#[lua_class(name = "HyprlandWorkspaceInfo")]
pub struct WorkspaceInfo {
    #[lua_attr(parent)]
    workspace: Workspace,
    /// The name of the monitor this workspace is on.
    monitor: String,
    /// The number of windows currently on this workspace.
    windows: u16,
    /// The title of the last focused window on this workspace.
    last_window_title: String,
    /// Whether this workspace is currently in fullscreen mode.
    fullscreen: bool,
    /// The unique identifier of the monitor this workspace is on.
    monitor_id: Option<i128>,
}

impl IntoLua for WorkspaceInfo {
    fn into_lua(self, lua: &Lua) -> mlua::Result<LuaValue> {
        let table = lua.create_table()?;
        table.set("id", self.workspace.id)?;
        table.set("name", self.workspace.name)?;
        table.set("monitor", self.monitor)?;
        table.set("windows", self.windows)?;
        table.set("last_window_title", self.last_window_title)?;
        table.set("fullscreen", self.fullscreen)?;
        table.set("monitor_id", self.monitor_id)?;
        Ok(LuaValue::Table(table))
    }
}

/// Returns a list of all current workspaces in Hyprland, including their IDs, names,
/// associated monitors, and status information.
#[lua_func(name = "getWorkspaces", module = "waypane.hyprland", skip = "lua")]
#[ret(doc = "workspaces A list of information about all current workspaces.")]
pub fn get_workspaces() -> mlua::Result<Vec<WorkspaceInfo>> {
    let workspaces = Workspaces::get()
        .map_err(|e| mlua::Error::external(format!("Failed to get workspaces: {}", e)))?
        .to_vec();

    let ws_infos: Vec<WorkspaceInfo> = workspaces
        .into_iter()
        .map(|ws| WorkspaceInfo {
            workspace: Workspace {
                id: ws.id,
                name: ws.name,
            },
            monitor: ws.monitor,
            windows: ws.windows,
            last_window_title: ws.last_window_title,
            fullscreen: ws.fullscreen,
            monitor_id: ws.monitor_id,
        })
        .collect();

    Ok(ws_infos)
}

/// Switches the focus to the workspace with the given numerical ID.
#[lua_func(name = "switchWorkspace", module = "waypane.hyprland")]
#[arg(
    name = "workspace_id",
    doc = "The numerical ID of the workspace to switch to."
)]
pub fn switch_ws(workspace_id: i32) -> mlua::Result<()> {
    call_dispatch(
        DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(workspace_id)),
        "switch workspace",
    )
}

/// Switches the focus to a workspace relative to the current one (e.g., +1 or -1).
#[lua_func(name = "switchWorkspaceRelative", module = "waypane.hyprland")]
#[arg(
    name = "offset",
    doc = "The relative offset from the current workspace (e.g., 1 for next, -1 for previous)."
)]
pub fn switch_ws_rel(offset: i32) -> mlua::Result<()> {
    call_dispatch(
        DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Relative(offset)),
        "switch workspace",
    )
}

/// Switches the focus to the workspace with the given name.
#[lua_func(name = "switchWorkspaceNamed", module = "waypane.hyprland")]
#[arg(
    name = "workspace_name",
    doc = "The name of the workspace to switch to."
)]
pub fn switch_ws_named(workspace_name: String) -> mlua::Result<()> {
    call_dispatch(
        DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Name(&workspace_name)),
        "switch workspace",
    )
}

/// Switches the focus back to the previously active workspace.
#[lua_func(name = "switchToPreviousWorkspace", module = "waypane.hyprland")]
pub fn switch_prev_ws() -> mlua::Result<()> {
    call_dispatch(
        DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Previous),
        "switch workspace",
    )
}

/// Moves the currently active window to the workspace with the given numerical ID.
#[lua_func(name = "moveActiveToWorkspace", module = "waypane.hyprland")]
#[arg(
    name = "workspace_id",
    doc = "The numerical ID of the workspace to move the window to."
)]
pub fn mv_active_to_ws(workspace_id: i32) -> mlua::Result<()> {
    call_dispatch(
        DispatchType::MoveToWorkspace(WorkspaceIdentifierWithSpecial::Id(workspace_id), None),
        "move active window to workspace",
    )
}

/// Moves the currently active window to the workspace with the given numerical ID without
/// switching focus to that workspace.
#[lua_func(name = "moveActiveToWorkspaceSilent", module = "waypane.hyprland")]
#[arg(
    name = "workspace_id",
    doc = "The numerical ID of the workspace to move the window to."
)]
pub fn mv_active_to_ws_silent(workspace_id: i32) -> mlua::Result<()> {
    call_dispatch(
        DispatchType::MoveToWorkspaceSilent(WorkspaceIdentifierWithSpecial::Id(workspace_id), None),
        "move active window to workspace silently",
    )
}

/// Toggles a special workspace (scratchpad) with the given name. If no name is provided,
/// the default special workspace is used.
#[lua_func(name = "toggleSpecialWorkspace", module = "waypane.hyprland")]
#[arg(
    name = "workspace_name",
    doc = "Optional name of the special workspace to toggle. If nil, the default special workspace is used."
)]
pub fn toggle_special_ws(workspace_name: Option<String>) -> mlua::Result<()> {
    call_dispatch(
        DispatchType::ToggleSpecialWorkspace(workspace_name),
        "toggle special workspace",
    )
}
