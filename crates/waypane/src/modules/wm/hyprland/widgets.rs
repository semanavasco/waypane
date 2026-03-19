use super::{
    monitors::get_monitors,
    windows::get_active_window,
    workspaces::{Workspace, WorkspaceInfo, get_workspaces, switch_ws},
};
use crate::{
    dynamic::signals::SIGNAL_BUS,
    lua::types::Orientation,
    widgets::{Properties, Widget},
};
use anyhow::Result;
use gtk4::{
    Box as GtkBox, Button as GtkButton, Label as GtkLabel,
    prelude::{BoxExt, ButtonExt, WidgetExt},
};
use mlua::{FromLua, Lua, Table as LuaTable, Value as LuaValue};
use std::{cell::RefCell, rc::Rc};
use waypane_macros::{LuaClass, WidgetBuilder};

/// A label widget that displays the title of the currently active window in Hyprland.
#[derive(LuaClass, WidgetBuilder)]
#[lua_class(name = "HyprlandActiveWindowLabelWidget")]
struct HyprlandActiveWindowLabel {
    #[lua_attr(parent)]
    pub properties: Properties,
}

impl Widget for HyprlandActiveWindowLabel {
    fn build(&self) -> Result<gtk4::Widget> {
        let active_window = get_active_window()?.map(|w| w.title);

        let label = GtkLabel::new(active_window.as_deref());
        self.properties.apply(&label)?;

        let label_clone = label.clone();
        let callback = Rc::new(move |data: LuaValue| {
            if let LuaValue::Table(t) = data
                && let Ok(title) = t.get::<String>("title")
            {
                label_clone.set_text(&title);
            }
        });

        let sub_id = SIGNAL_BUS.with(|bus| {
            bus.borrow_mut()
                .subscribe("hyprland::active_window_changed", callback)
        });

        label.connect_destroy(move |_| {
            SIGNAL_BUS.with(|bus| {
                bus.borrow_mut()
                    .unsubscribe("hyprland::active_window_changed", sub_id)
            });
        });

        Ok(label.into())
    }
}

impl FromLua for HyprlandActiveWindowLabel {
    fn from_lua(value: LuaValue, _: &Lua) -> mlua::Result<Self> {
        let table = match &value {
            LuaValue::Table(t) => t,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Label".to_string(),
                    message: Some("Expected a table".to_string()),
                });
            }
        };

        Ok(HyprlandActiveWindowLabel {
            properties: Properties::parse(table)?,
        })
    }
}

/// A container widget that displays a list of workspace buttons in Hyprland.
#[derive(LuaClass, WidgetBuilder)]
#[lua_class(name = "HyprlandWsContainerWidget")]
struct HyprlandWsContainer {
    #[lua_attr(parent)]
    pub properties: Properties,
    /// The orientation of the container.
    pub orientation: Orientation,
    /// The spacing between children in the container, in pixels.
    #[lua_attr(default = 0)]
    pub spacing: i32,
    /// An optional monitor name to filter workspaces by.
    pub monitor: Option<String>,
    /// Optional widget properties to apply to active workspace buttons.
    /// **(DO NOT PASS A WIDGET DIRECTLY, ONLY ITS COMMON PROPERTIES)**
    pub active_properties: Option<Rc<Properties>>,
    /// Optional widget properties to apply to inactive workspace buttons.
    /// **(DO NOT PASS A WIDGET DIRECTLY, ONLY ITS COMMON PROPERTIES)**
    pub inactive_properties: Option<Rc<Properties>>,
    /// A list of workspace IDs that should always be shown, even if they have no windows.
    pub persistent_workspaces: Option<Vec<i32>>,
    /// Whether to hide workspaces that have no windows and are not active.
    #[lua_attr(default = false)]
    pub hide_empty: bool,
}

impl Widget for HyprlandWsContainer {
    fn build(&self) -> Result<gtk4::Widget> {
        let container = GtkBox::new(self.orientation.into(), self.spacing);
        self.properties.apply(&container)?;

        let container_clone = container.clone();
        let monitor_filter = self.monitor.clone();
        let active_props = self.active_properties.clone();
        let inactive_props = self.inactive_properties.clone();
        let persistent_workspaces = self.persistent_workspaces.clone();
        let hide_empty = self.hide_empty;

        let callback = Rc::new(move |_| {
            let Ok(mut wss) = get_workspaces() else {
                return;
            };

            let Ok(monitors) = get_monitors() else {
                return;
            };

            // Identify active workspaces across all monitors
            let active_workspace_ids: Vec<i32> =
                monitors.iter().map(|m| m.active_workspace.id).collect();

            // Clear existing children
            while let Some(child) = container_clone.first_child() {
                container_clone.remove(&child);
            }

            // If we have persistent workspaces, ensure they are in the list
            if let Some(persistent) = &persistent_workspaces {
                for &id in persistent {
                    if !wss.iter().any(|ws| ws.workspace.id == id) {
                        wss.push(WorkspaceInfo {
                            workspace: Workspace {
                                id,
                                name: id.to_string(),
                            },
                            monitor: String::new(),
                            windows: 0,
                            last_window_title: String::new(),
                            fullscreen: false,
                            monitor_id: None,
                        });
                    }
                }
            }

            // Filter by monitor if specified
            if let Some(monitor) = &monitor_filter {
                wss.retain(|ws| ws.monitor == *monitor || ws.monitor.is_empty());
            }

            // Handle hide_empty
            if hide_empty {
                wss.retain(|ws| {
                    ws.windows > 0
                        || active_workspace_ids.contains(&ws.workspace.id)
                        || persistent_workspaces
                            .as_ref()
                            .map_or(false, |p| p.contains(&ws.workspace.id))
                });
            }

            wss.sort_by(|a, b| a.workspace.id.cmp(&b.workspace.id));

            for ws in wss {
                let ws_id = ws.workspace.id;
                let is_active = active_workspace_ids.contains(&ws_id);

                // If monitor filter, check if is active workspace on that monitor
                let should_highlight = if let Some(monitor) = &monitor_filter {
                    monitors
                        .iter()
                        .any(|m| m.name == *monitor && m.active_workspace.id == ws_id)
                } else {
                    is_active
                };

                let button = GtkButton::with_label(&ws.workspace.name);

                // Apply properties based on active status
                if should_highlight {
                    if let Some(props) = &active_props {
                        if let Err(e) = props.apply(&button) {
                            tracing::error!(
                                "Couldn't apply active properties to workspace button: {e}"
                            );
                        }
                    } else if let Some(props) = &inactive_props {
                        let _ = props.apply(&button);
                    }
                } else if let Some(props) = &inactive_props {
                    if let Err(e) = props.apply(&button) {
                        tracing::error!(
                            "Couldn't apply inactive properties to workspace button: {e}"
                        );
                    }
                }

                container_clone.append(&button);

                button.connect_clicked(move |_| {
                    if let Err(e) = switch_ws(ws_id) {
                        tracing::error!("Couldn't switch workspace: {e}");
                    }
                });
            }
        });

        callback(LuaValue::Nil);

        let signals = [
            "hyprland::workspace_changed",
            "hyprland::workspace_added",
            "hyprland::workspace_deleted",
            "hyprland::workspace_moved",
            "hyprland::workspace_renamed",
            "hyprland::active_monitor_changed",
        ];

        let sub_ids = Rc::new(RefCell::new(Vec::new()));
        for signal in signals {
            let cb = callback.clone();
            let sub_id = SIGNAL_BUS.with(|bus| bus.borrow_mut().subscribe(signal, cb));
            sub_ids.borrow_mut().push((signal, sub_id));
        }

        let sub_ids_cleanup = sub_ids.clone();
        container.connect_destroy(move |_| {
            SIGNAL_BUS.with(|bus| {
                let mut bus = bus.borrow_mut();
                for (signal, id) in sub_ids_cleanup.borrow().iter() {
                    bus.unsubscribe(signal, *id);
                }
            });
        });

        Ok(container.into())
    }
}

impl FromLua for HyprlandWsContainer {
    fn from_lua(value: LuaValue, _: &Lua) -> mlua::Result<Self> {
        let table = match &value {
            LuaValue::Table(t) => t,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Container".to_string(),
                    message: Some("Expected a table".to_string()),
                });
            }
        };

        Ok(HyprlandWsContainer {
            properties: Properties::parse(table)?,
            orientation: table.get("orientation")?,
            spacing: table.get::<Option<i32>>("spacing")?.unwrap_or(0),
            monitor: table.get("monitor")?,
            active_properties: if let Some(props) =
                table.get::<Option<LuaTable>>("active_properties")?
            {
                Some(Rc::new(Properties::parse(&props)?))
            } else {
                None
            },
            inactive_properties: if let Some(props) =
                table.get::<Option<LuaTable>>("inactive_properties")?
            {
                Some(Rc::new(Properties::parse(&props)?))
            } else {
                None
            },
            persistent_workspaces: table.get("persistent_workspaces")?,
            hide_empty: table.get::<Option<bool>>("hide_empty")?.unwrap_or(false),
        })
    }
}
