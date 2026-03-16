pub mod monitors;
pub mod utils;
pub mod windows;
pub mod workspaces;

use async_channel::{Receiver, Sender, TrySendError};
use hyprland::{event_listener::EventListener, shared::WorkspaceType};
use mlua::{IntoLua, Lua, Value as LuaValue};
use monitors::{ActiveMonitor, get_monitors};
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};
use waypane_macros::LuaModule;
use windows::{Window, get_active_window, kill_active_win, toggle_floating, toggle_fs};
use workspaces::{
    Workspace, get_workspaces, mv_active_to_ws, mv_active_to_ws_silent, switch_prev_ws, switch_ws,
    switch_ws_named, switch_ws_rel, toggle_special_ws,
};

const EVENT_QUEUE_CAPACITY: usize = 100;

fn enqueue_event(
    sender: &Sender<(String, HyprlandEvent)>,
    queue: &Receiver<(String, HyprlandEvent)>,
    dropped_events: &AtomicUsize,
    signal: &str,
    event: (String, HyprlandEvent),
) {
    match sender.try_send(event) {
        Ok(()) => {}
        Err(TrySendError::Full(event)) => {
            // Keep latest state relevant by dropping the oldest queued event under pressure.
            let _ = queue.try_recv();
            let _ = sender.try_send(event);

            let dropped = dropped_events.fetch_add(1, Ordering::Relaxed) + 1;
            if dropped == 1 || dropped.is_multiple_of(100) {
                tracing::warn!(
                    "Hyprland event queue full (capacity: {}), dropped oldest events total: {}",
                    EVENT_QUEUE_CAPACITY,
                    dropped
                );
            }
        }
        Err(TrySendError::Closed(_)) => {
            tracing::warn!("Failed to send {} event: receiver closed", signal);
        }
    }
}

/// The `hyprland` module, which provides functions for querying Hyprland state and dispatching
/// commands, as well as forwarding events from the Hyprland IPC listener.
///
/// ### Signals
/// The following signals are emitted on the `waypane` signal bus:
/// - `hyprland::workspace_changed` : Emitted when the user switches to a different workspace.
///   Data: `HyprlandWorkspace`
/// - `hyprland::workspace_added` : Emitted when a new workspace is created.
///   Data: `HyprlandWorkspace`
/// - `hyprland::workspace_deleted` : Emitted when a workspace is destroyed.
///   Data: `HyprlandWorkspace`
/// - `hyprland::workspace_moved` : Emitted when a workspace is moved to another monitor.
///   Data: `HyprlandWorkspace`
/// - `hyprland::workspace_renamed` : Emitted when a workspace is given a new name.
///   Data: `HyprlandWorkspace`
/// - `hyprland::active_window` : Emitted when the focused window changes.
///   Data: `HyprlandWindow`
/// - `hyprland::fullscreen_changed` : Emitted when the active window's fullscreen state toggles.
///   Data: `boolean`
/// - `hyprland::active_monitor_changed` : Emitted when focus moves to a different monitor.
///   Data: `HyprlandActiveMonitor`
#[allow(dead_code)]
#[derive(LuaModule)]
#[lua_module(parent = "waypane")]
struct Hyprland;

/// Events emitted by the Hyprland IPC listener.
///
/// Each variant maps to Lua signal(s) dispatched as `hyprland::<event_name>`.
pub enum HyprlandEvent {
    /// Emitted for workspace lifecycle and focus events.
    ///
    /// Signals:
    /// - `hyprland::workspace_changed` : the user switched to a different workspace
    /// - `hyprland::workspace_added`   : a new workspace was created
    /// - `hyprland::workspace_deleted` : a workspace was destroyed
    /// - `hyprland::workspace_moved`   : a workspace was moved to another monitor
    /// - `hyprland::workspace_renamed` : a workspace was given a new name
    Workspace(Workspace),

    /// Emitted when the focused window changes.
    ///
    /// Signal: `hyprland::active_window`
    ActiveWindowChanged(Window),

    /// Emitted when the fullscreen state of the active window toggles.
    ///
    /// Signal: `hyprland::fullscreen_changed`
    FullscreenStateChanged(bool),

    /// Emitted when keyboard focus moves to a different monitor.
    ///
    /// Signal: `hyprland::active_monitor_changed`
    ActiveMonitorChanged(ActiveMonitor),
}

impl IntoLua for HyprlandEvent {
    fn into_lua(self, lua: &Lua) -> mlua::Result<LuaValue> {
        match self {
            HyprlandEvent::Workspace(event) => event.into_lua(lua),
            HyprlandEvent::ActiveWindowChanged(event) => event.into_lua(lua),
            HyprlandEvent::FullscreenStateChanged(state) => Ok(LuaValue::Boolean(state)),
            HyprlandEvent::ActiveMonitorChanged(event) => event.into_lua(lua),
        }
    }
}

/// Helper function to parse workspace names from Hyprland events.
fn parse_ws(name: WorkspaceType) -> String {
    match name {
        WorkspaceType::Regular(name) => name,
        WorkspaceType::Special(Some(name)) => name,
        WorkspaceType::Special(None) => "special".to_string(),
    }
}

/// Spawns a background thread that connects to the Hyprland IPC socket and listens for compositor
/// events.
///
/// Returns a [`Receiver`] that yields `(signal_name, event)` pairs. The `signal_name` corresponds
/// to the `hyprland::<name>` signal emitted on the Lua signal bus. Callers must drive the receiver
/// on the GTK main thread and forward each event to the signal bus.
///
/// The thread runs until the Hyprland IPC socket closes or an unrecoverable error occurs, at which
/// point an error is logged and the thread exits.
pub fn start_listener() -> Receiver<(String, HyprlandEvent)> {
    let (sender, receiver) = async_channel::bounded(EVENT_QUEUE_CAPACITY);
    let queue = receiver.clone();
    let dropped_events = Arc::new(AtomicUsize::new(0));

    thread::spawn(move || {
        let mut listener = EventListener::new();

        // Helper macro to add an event handler that forwards a specific signal with the provided
        // data transformation
        macro_rules! add_event {
            ($method:ident, $signal:expr, $data:ident => $event:expr) => {
                let s = sender.clone();
                let queue = queue.clone();
                let dropped_events = dropped_events.clone();
                listener.$method(move |$data| {
                    enqueue_event(
                        &s,
                        &queue,
                        dropped_events.as_ref(),
                        $signal,
                        ($signal.to_string(), $event),
                    );
                });
            };
        }

        // Helper macro to add workspace-related event handlers
        macro_rules! add_ws_event {
            ($method:ident, $signal:expr) => {
                add_event!($method, $signal, ws =>
                    HyprlandEvent::Workspace(Workspace {
                        id: ws.id,
                        name: parse_ws(ws.name)
                    })
                );
            };
        }

        add_ws_event!(add_workspace_changed_handler, "workspace_changed");
        add_ws_event!(add_workspace_deleted_handler, "workspace_deleted");
        add_ws_event!(add_workspace_added_handler, "workspace_added");
        add_ws_event!(add_workspace_moved_handler, "workspace_moved");

        add_event!(add_workspace_renamed_handler, "workspace_renamed", ws =>
            HyprlandEvent::Workspace(Workspace { id: ws.id, name: ws.name })
        );

        // Window events
        add_event!(add_active_window_changed_handler, "active_window", ev => {
            let (title, class) = ev.map_or((String::new(), String::new()), |w| (w.title, w.class));
            HyprlandEvent::ActiveWindowChanged(Window { title, class })
        });

        add_event!(add_fullscreen_state_changed_handler, "fullscreen_changed", state =>
            HyprlandEvent::FullscreenStateChanged(state)
        );

        // Monitor events
        add_event!(add_active_monitor_changed_handler, "active_monitor_changed", ev =>
            HyprlandEvent::ActiveMonitorChanged(ActiveMonitor {
                monitor: ev.monitor_name,
                workspace: ev.workspace_name.map(parse_ws),
            })
        );

        tracing::debug!("Starting Hyprland IPC listener...");

        if let Err(e) = listener.start_listener() {
            tracing::error!("Hyprland event listener crashed: {}", e);
        }
    });

    receiver
}

/// Registers Hyprland-specific Lua functions under the `hyprland` table.
pub fn register_lua(lua: &Lua, table: &mlua::Table) -> mlua::Result<()> {
    let hyprland = lua.create_table()?;

    hyprland.set(
        "getWorkspaces",
        lua.create_function(|_, ()| get_workspaces())?,
    )?;

    hyprland.set("getMonitors", lua.create_function(|_, ()| get_monitors())?)?;

    hyprland.set(
        "getActiveWindow",
        lua.create_function(|_, ()| get_active_window())?,
    )?;

    hyprland.set(
        "switchWorkspace",
        lua.create_function(|_, ws_id| switch_ws(ws_id))?,
    )?;

    hyprland.set(
        "switchWorkspaceRelative",
        lua.create_function(|_, offset| switch_ws_rel(offset))?,
    )?;

    hyprland.set(
        "switchWorkspaceNamed",
        lua.create_function(|_, ws_name| switch_ws_named(ws_name))?,
    )?;

    hyprland.set(
        "switchToPreviousWorkspace",
        lua.create_function(|_, ()| switch_prev_ws())?,
    )?;

    hyprland.set(
        "moveActiveToWorkspace",
        lua.create_function(|_, ws_id| mv_active_to_ws(ws_id))?,
    )?;

    hyprland.set(
        "moveActiveToWorkspaceSilent",
        lua.create_function(|_, ws_id| mv_active_to_ws_silent(ws_id))?,
    )?;

    hyprland.set(
        "toggleSpecialWorkspace",
        lua.create_function(|_, ws_name| toggle_special_ws(ws_name))?,
    )?;

    hyprland.set(
        "toggleFloating",
        lua.create_function(|_, ()| toggle_floating())?,
    )?;

    hyprland.set(
        "toggleFullscreen",
        lua.create_function(|_, ()| toggle_fs())?,
    )?;

    hyprland.set(
        "killActiveWindow",
        lua.create_function(|_, ()| kill_active_win())?,
    )?;

    table.set("hyprland", hyprland)?;
    Ok(())
}
