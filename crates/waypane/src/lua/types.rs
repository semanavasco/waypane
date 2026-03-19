//! Lua-to-Rust type conversion bridges for configuration types.
//!
//! Each type here implements [`mlua::FromLua`] so it can be parsed directly from a Lua config
//! table, and [`From<T>`] for the corresponding GTK / layer-shell type so it can be passed to
//! GTK without an extra conversion step.
//!
//! This module also implements the [`LuaType`] trait, which provides a string representation
//! of the type for documentation generation and error messages in Lua parsing.

use crate::lua::stubs::LuaType;
use gtk4::{Align as GtkAlign, Orientation as GtkOrientation};
use gtk4_layer_shell::Layer as GtkLayer;
use mlua::{FromLua, IntoLua, Lua, Table as LuaTable, Value as LuaValue};
use std::{borrow::Cow, rc::Rc};
use waypane_macros::{LuaClass, LuaEnum};

/// Simple macro to implement LuaType for multiple types with the same Lua type string.
macro_rules! impl_lua_type {
    ($s:literal for $($t:ty),+) => {
        $(impl LuaType for $t {
            fn lua_type() -> Cow<'static, str> {
                $s.into()
            }
        })*
    }
}

impl LuaType for String {
    fn lua_type() -> Cow<'static, str> {
        "string".into()
    }
}

impl LuaType for bool {
    fn lua_type() -> Cow<'static, str> {
        "boolean".into()
    }
}

impl_lua_type!("number" for i16, i32, i128, u16, u32, u64, f32, f64, usize);

impl<T> LuaType for Rc<T>
where
    T: LuaType,
{
    fn lua_type() -> Cow<'static, str> {
        T::lua_type()
    }
}

impl<T> LuaType for Option<T>
where
    T: LuaType,
{
    fn lua_type() -> Cow<'static, str> {
        format!("? {}", T::lua_type()).into()
    }
}

impl<T> LuaType for Vec<T>
where
    T: LuaType,
{
    fn lua_type() -> Cow<'static, str> {
        format!("{}[]", T::lua_type()).into()
    }
}

impl LuaType for () {
    fn lua_type() -> Cow<'static, str> {
        "nil".into()
    }
}

impl<T> LuaType for mlua::Result<T>
where
    T: LuaType,
{
    fn lua_type() -> Cow<'static, str> {
        T::lua_type()
    }
}

impl LuaType for LuaValue {
    fn lua_type() -> Cow<'static, str> {
        "any".into()
    }
}

impl_lua_type!("function" for mlua::Function, mlua::RegistryKey);

impl LuaType for LuaTable {
    fn lua_type() -> Cow<'static, str> {
        "table".into()
    }
}

/// The z-level layer where the window will be placed.
#[derive(Clone, Copy, LuaEnum)]
pub enum Layer {
    Background,
    Bottom,
    Top,
    Overlay,
}

impl FromLua for Layer {
    fn from_lua(value: LuaValue, _: &Lua) -> mlua::Result<Self> {
        let layer = match &value {
            LuaValue::String(s) => s.to_str()?,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Layer".to_string(),
                    message: Some("Expected a string".to_string()),
                });
            }
        };

        match layer.to_lowercase().as_str() {
            "background" => Ok(Layer::Background),
            "bottom" => Ok(Layer::Bottom),
            "top" => Ok(Layer::Top),
            "overlay" => Ok(Layer::Overlay),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Layer".to_string(),
                message: Some(format!("Invalid layer: {}", layer)),
            }),
        }
    }
}

impl From<Layer> for GtkLayer {
    fn from(value: Layer) -> Self {
        match value {
            Layer::Background => GtkLayer::Background,
            Layer::Bottom => GtkLayer::Bottom,
            Layer::Top => GtkLayer::Top,
            Layer::Overlay => GtkLayer::Overlay,
        }
    }
}

/// Anchor points for the window to stick to specific edges of the monitor.
#[derive(Clone, Default, LuaClass)]
pub struct Anchors {
    /// Whether to anchor the window to the top edge of the monitor.
    #[lua_attr(default = false)]
    pub top: bool,
    /// Whether to anchor the window to the right edge of the monitor.
    #[lua_attr(default = false)]
    pub right: bool,
    /// Whether to anchor the window to the bottom edge of the monitor.
    #[lua_attr(default = false)]
    pub bottom: bool,
    /// Whether to anchor the window to the left edge of the monitor.
    #[lua_attr(default = false)]
    pub left: bool,
}

impl FromLua for Anchors {
    fn from_lua(value: LuaValue, _: &Lua) -> mlua::Result<Self> {
        let anchors = match &value {
            LuaValue::Table(t) => t,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Anchors".to_string(),
                    message: Some("Expected a table".to_string()),
                });
            }
        };

        Ok(Anchors {
            top: anchors.get::<Option<bool>>("top")?.unwrap_or(false),
            right: anchors.get::<Option<bool>>("right")?.unwrap_or(false),
            bottom: anchors.get::<Option<bool>>("bottom")?.unwrap_or(false),
            left: anchors.get::<Option<bool>>("left")?.unwrap_or(false),
        })
    }
}

/// Margin in pixels from each edge of the monitor.
#[derive(Clone, Default, LuaClass)]
pub struct Margins {
    /// Margin from the top edge of the monitor, in pixels.
    #[lua_attr(default = 0)]
    pub top: i32,
    /// Margin from the right edge of the monitor, in pixels.
    #[lua_attr(default = 0)]
    pub right: i32,
    /// Margin from the bottom edge of the monitor, in pixels.
    #[lua_attr(default = 0)]
    pub bottom: i32,
    /// Margin from the left edge of the monitor, in pixels.
    #[lua_attr(default = 0)]
    pub left: i32,
}

impl FromLua for Margins {
    fn from_lua(value: LuaValue, _: &Lua) -> mlua::Result<Self> {
        let margins = match &value {
            LuaValue::Table(t) => t,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Margins".to_string(),
                    message: Some("Expected a table".to_string()),
                });
            }
        };

        Ok(Margins {
            top: margins.get::<Option<i32>>("top")?.unwrap_or(0),
            right: margins.get::<Option<i32>>("right")?.unwrap_or(0),
            bottom: margins.get::<Option<i32>>("bottom")?.unwrap_or(0),
            left: margins.get::<Option<i32>>("left")?.unwrap_or(0),
        })
    }
}

/// Orientation for container widgets.
#[derive(Clone, Copy, LuaEnum)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl FromLua for Orientation {
    fn from_lua(value: LuaValue, _: &Lua) -> mlua::Result<Self> {
        let orientation = match &value {
            LuaValue::String(s) => s.to_str()?,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Orientation".to_string(),
                    message: Some("Expected a string".to_string()),
                });
            }
        };

        match orientation.to_lowercase().as_str() {
            "horizontal" => Ok(Orientation::Horizontal),
            "vertical" => Ok(Orientation::Vertical),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Orientation".to_string(),
                message: Some(format!("Invalid orientation: {}", orientation)),
            }),
        }
    }
}

impl From<Orientation> for GtkOrientation {
    fn from(value: Orientation) -> Self {
        match value {
            Orientation::Horizontal => GtkOrientation::Horizontal,
            Orientation::Vertical => GtkOrientation::Vertical,
        }
    }
}

/// Alignment for widgets within their parent container.
#[derive(Clone, Copy, LuaEnum)]
pub enum Alignment {
    Start,
    Center,
    End,
    Fill,
    Baseline,
}

impl FromLua for Alignment {
    fn from_lua(value: LuaValue, _: &Lua) -> mlua::Result<Self> {
        let alignment = match &value {
            LuaValue::String(s) => s.to_str()?,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Alignment".to_string(),
                    message: Some("Expected a string".to_string()),
                });
            }
        };

        match alignment.to_lowercase().as_str() {
            "start" => Ok(Alignment::Start),
            "center" => Ok(Alignment::Center),
            "end" => Ok(Alignment::End),
            "fill" => Ok(Alignment::Fill),
            "baseline" => Ok(Alignment::Baseline),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Alignment".to_string(),
                message: Some(format!("Invalid alignment: {}", alignment)),
            }),
        }
    }
}

impl From<Alignment> for GtkAlign {
    fn from(value: Alignment) -> Self {
        match value {
            Alignment::Start => GtkAlign::Start,
            Alignment::Center => GtkAlign::Center,
            Alignment::End => GtkAlign::End,
            Alignment::Fill => GtkAlign::Fill,
            Alignment::Baseline => GtkAlign::Baseline,
        }
    }
}

/// A helper type to allow either a single string or an array of strings in Lua.
pub enum StringOrStrings {
    Single(String),
    Multiple(Vec<String>),
}

impl LuaType for StringOrStrings {
    fn lua_type() -> Cow<'static, str> {
        "string | string[]".into()
    }
}

impl FromLua for StringOrStrings {
    fn from_lua(value: LuaValue, _: &Lua) -> mlua::Result<Self> {
        match value {
            LuaValue::String(s) => Ok(StringOrStrings::Single(s.to_str()?.to_string())),
            LuaValue::Table(t) => {
                let mut strings = Vec::new();
                for pair in t.sequence_values::<String>() {
                    strings.push(pair?);
                }
                Ok(StringOrStrings::Multiple(strings))
            }
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "string or string[]".to_string(),
                message: Some("Expected a string or an array of strings".to_string()),
            }),
        }
    }
}

impl IntoLua for StringOrStrings {
    fn into_lua(self, lua: &Lua) -> mlua::Result<LuaValue> {
        match self {
            StringOrStrings::Single(s) => s.into_lua(lua),
            StringOrStrings::Multiple(vec) => vec.into_lua(lua),
        }
    }
}
