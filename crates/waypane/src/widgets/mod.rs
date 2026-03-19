mod button;
mod container;
mod icon;
mod image;
mod label;
mod progress_bar;
mod slider;
mod stack;

use crate::{
    dynamic::MaybeReactive,
    lua::{
        LUA,
        stubs::LuaType,
        types::{Alignment, Margins},
    },
};
use anyhow::Result;
use gtk4::{EventControllerScroll, EventControllerScrollFlags, glib::object::IsA, prelude::*};
use mlua::{FromLua, Lua, Value as LuaValue};
use std::{borrow::Cow, rc::Rc};
use waypane_macros::LuaClass;

/// Base trait for all UI components in waypane.
pub trait Widget {
    /// Builds the corresponding GTK widget.
    ///
    /// # Errors
    /// Returns an error if evaluating a dynamic property fails, or if the GTK widget fails to
    /// initialize for some reason.
    fn build(&self) -> Result<gtk4::Widget>;
}

impl LuaType for Box<dyn Widget> {
    fn lua_type() -> Cow<'static, str> {
        "Widget".into()
    }
}

/// Factory struct used for dynamic widget creation based on a "type" field in Lua tables.
pub struct WidgetFactory {
    pub name: &'static str,
    pub build: fn(LuaValue, &Lua) -> mlua::Result<Box<dyn Widget>>,
}
inventory::collect!(WidgetFactory);

impl FromLua for Box<dyn Widget> {
    /// Deserializes a Lua table into a specific `Widget` trait object.
    /// The table must contain a `type` field (e.g., "button", "label") to determine which widget
    /// to instantiate.
    /// The rest of the fields are passed to the specific widget's `from_lua` implementation.
    fn from_lua(value: LuaValue, lua: &Lua) -> mlua::Result<Self> {
        let table = match &value {
            LuaValue::Table(t) => t,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Widget".to_string(),
                    message: Some("Expected a table".to_string()),
                });
            }
        };

        let widget_type: String = table.get("type")?;

        let factory = inventory::iter::<WidgetFactory>
            .into_iter()
            .find(|f| f.name == widget_type)
            .ok_or_else(|| mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Widget".to_string(),
                message: Some(format!("Unknown widget type: {}", widget_type)),
            })?;

        (factory.build)(value, lua)
    }
}

/// Common properties shared by all widgets (layout, CSS classes, IDs, etc).
#[derive(LuaClass)]
#[lua_class(name = "Widget")]
pub struct Properties {
    /// Optional widget ID, used for CSS styling and querying.
    pub id: MaybeReactive<Option<String>>,
    /// Optional list of CSS classes applied to the widget.
    #[lua_attr(optional)]
    pub class_list: MaybeReactive<Vec<String>>,
    /// Optional horizontal alignment for the widget.
    pub halign: MaybeReactive<Option<Alignment>>,
    /// Optional vertical alignment for the widget.
    pub valign: MaybeReactive<Option<Alignment>>,
    /// Whether the widget should expand to fill available horizontal space.
    #[lua_attr(default = false)]
    pub hexpand: MaybeReactive<bool>,
    /// Whether the widget should expand to fill available vertical space.
    #[lua_attr(default = false)]
    pub vexpand: MaybeReactive<bool>,
    /// Whether the widget is visible.
    #[lua_attr(default = true)]
    pub visible: MaybeReactive<bool>,
    /// Whether the widget can receive keyboard focus.
    #[lua_attr(default = true)]
    pub focusable: MaybeReactive<bool>,
    /// Optional tooltip markup text for the widget.
    pub tooltip: MaybeReactive<Option<String>>,
    /// Optional margins around the widget.
    #[lua_attr(optional)]
    pub margins: MaybeReactive<Margins>,
    /// Optional width request for the widget.
    #[lua_attr(default = -1)]
    pub width_request: MaybeReactive<i32>,
    /// Optional height request for the widget.
    #[lua_attr(default = -1)]
    pub height_request: MaybeReactive<i32>,
    /// Whether the widget should be sensitive to user input.
    #[lua_attr(default = true)]
    pub sensitive: MaybeReactive<bool>,
    /// Optional function to execute when scrolling over the widget. Receives (dx, dy) as arguments.
    pub on_scroll: Option<Rc<mlua::RegistryKey>>,
}

impl Properties {
    /// Parses properties from a Lua table.
    ///
    /// Used turbofish syntax extensively to provide defaults for all properties without crashing
    /// if they are missing from the Lua table but still crashing if they are of the wrong type.
    fn parse(table: &mlua::Table) -> mlua::Result<Self> {
        let lua = LUA
            .get()
            .ok_or_else(|| mlua::Error::RuntimeError("Lua instance not initialized".to_string()))?;

        let on_scroll = match table.get::<LuaValue>("on_scroll")? {
            LuaValue::Function(f) => Some(Rc::new(lua.create_registry_value(f)?)),
            LuaValue::Nil => None,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: "non-function",
                    to: "Widget on_scroll".to_string(),
                    message: Some("Expected a function for on_scroll".to_string()),
                });
            }
        };

        Ok(Properties {
            id: table
                .get::<Option<MaybeReactive<Option<String>>>>("id")?
                .unwrap_or(MaybeReactive::Static(None)),
            class_list: table
                .get::<Option<MaybeReactive<Vec<String>>>>("class_list")?
                .unwrap_or(MaybeReactive::Static(Vec::new())),
            halign: table
                .get::<Option<MaybeReactive<Option<Alignment>>>>("halign")?
                .unwrap_or(MaybeReactive::Static(None)),
            valign: table
                .get::<Option<MaybeReactive<Option<Alignment>>>>("valign")?
                .unwrap_or(MaybeReactive::Static(None)),
            hexpand: table
                .get::<Option<MaybeReactive<bool>>>("hexpand")?
                .unwrap_or(MaybeReactive::Static(false)),
            vexpand: table
                .get::<Option<MaybeReactive<bool>>>("vexpand")?
                .unwrap_or(MaybeReactive::Static(false)),
            visible: table
                .get::<Option<MaybeReactive<bool>>>("visible")?
                .unwrap_or(MaybeReactive::Static(true)),
            focusable: table
                .get::<Option<MaybeReactive<bool>>>("focusable")?
                .unwrap_or(MaybeReactive::Static(true)),
            tooltip: table
                .get::<Option<MaybeReactive<Option<String>>>>("tooltip")?
                .unwrap_or(MaybeReactive::Static(None)),
            margins: table
                .get::<Option<MaybeReactive<Margins>>>("margins")?
                .unwrap_or(MaybeReactive::Static(Margins::default())),
            width_request: table
                .get::<Option<MaybeReactive<i32>>>("width_request")?
                .unwrap_or(MaybeReactive::Static(-1)),
            height_request: table
                .get::<Option<MaybeReactive<i32>>>("height_request")?
                .unwrap_or(MaybeReactive::Static(-1)),
            sensitive: table
                .get::<Option<MaybeReactive<bool>>>("sensitive")?
                .unwrap_or(MaybeReactive::Static(true)),
            on_scroll,
        })
    }

    /// Applies the properties to a given GTK widget.
    /// If a property is dynamic, this method automatically registers the necessary background
    /// loops and event listeners to keep the widget updated.
    ///
    /// # Errors
    /// Returns an error if evaluating a dynamic property fails, or if the GTK widget fails to
    /// update for some reason.
    pub fn apply(&self, widget: &impl IsA<gtk4::Widget>) -> Result<()> {
        let widget = widget.as_ref();

        self.id.bind(widget, "id", |w, id| {
            if let Some(id) = id {
                w.set_widget_name(&id);
            }
        })?;

        self.class_list.bind(widget, "class_list", |w, classes| {
            let class_refs: Vec<&str> = classes.iter().map(|s| s.as_str()).collect();
            w.set_css_classes(&class_refs);
        })?;

        self.halign.bind(widget, "halign", |w, halign| {
            if let Some(halign) = halign {
                w.set_halign(halign.into());
            }
        })?;

        self.valign.bind(widget, "valign", |w, valign| {
            if let Some(valign) = valign {
                w.set_valign(valign.into());
            }
        })?;

        self.hexpand.bind(widget, "hexpand", |w, hexpand| {
            w.set_hexpand(hexpand);
        })?;

        self.vexpand.bind(widget, "vexpand", |w, vexpand| {
            w.set_vexpand(vexpand);
        })?;

        self.visible.bind(widget, "visible", |w, visible| {
            w.set_visible(visible);
        })?;

        self.focusable.bind(widget, "focusable", |w, focusable| {
            w.set_focusable(focusable);
        })?;

        self.tooltip.bind(widget, "tooltip", |w, tooltip| {
            w.set_tooltip_markup(tooltip.as_deref());
        })?;

        self.margins.bind(widget, "margins", |w, margins| {
            w.set_margin_start(margins.left);
            w.set_margin_end(margins.right);
            w.set_margin_top(margins.top);
            w.set_margin_bottom(margins.bottom);
        })?;

        self.width_request
            .bind(widget, "width_request", |w, width| {
                w.set_width_request(width);
            })?;

        self.height_request
            .bind(widget, "height_request", |w, height| {
                w.set_height_request(height);
            })?;

        self.sensitive.bind(widget, "sensitive", |w, sensitive| {
            w.set_sensitive(sensitive);
        })?;

        if let Some(ref on_scroll) = self.on_scroll {
            let scroll_controller =
                EventControllerScroll::new(EventControllerScrollFlags::BOTH_AXES);

            let on_scroll = on_scroll.clone();

            scroll_controller.connect_scroll(move |_, dx, dy| {
                let Some(lua) = LUA.get() else {
                    return gtk4::glib::Propagation::Proceed;
                };

                if let Ok(func) = lua.registry_value::<mlua::Function>(&on_scroll)
                    && let Err(e) = func.call::<()>((dx, dy))
                {
                    tracing::error!("Error calling on_scroll function: {}", e);
                }

                gtk4::glib::Propagation::Stop
            });

            widget.add_controller(scroll_controller);
        }

        Ok(())
    }
}
