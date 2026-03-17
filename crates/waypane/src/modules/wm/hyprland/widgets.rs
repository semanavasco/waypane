use super::windows::get_active_window;
use crate::{
    dynamic::signals::SIGNAL_BUS,
    widgets::{Properties, Widget},
};
use anyhow::Result;
use gtk4::{Label as GtkLabel, prelude::WidgetExt};
use mlua::{FromLua, Lua, Value as LuaValue};
use std::rc::Rc;
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
