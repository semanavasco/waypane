use crate::{
    dynamic::MaybeReactive,
    widgets::{Properties, Widget},
};
use anyhow::Result;
use gtk4::Stack as GtkStack;
use mlua::{FromLua, Lua, Value as LuaValue};
use waypane_macros::{LuaClass, LuaEnum, WidgetBuilder};

/// Transition types for the Stack widget.
#[derive(Clone, Copy, LuaEnum)]
pub enum StackTransitionType {
    None,
    Crossfade,
    SlideRight,
    SlideLeft,
    SlideUp,
    SlideDown,
    SlideLeftRight,
    SlideUpDown,
    OverUp,
    OverDown,
    OverLeft,
    OverRight,
    UnderUp,
    UnderDown,
    UnderLeft,
    UnderRight,
    OverUpDown,
    OverLeftRight,
    RotateLeft,
    RotateRight,
    RotateLeftRight,
}

impl FromLua for StackTransitionType {
    fn from_lua(value: LuaValue, _: &Lua) -> mlua::Result<Self> {
        let transition = match &value {
            LuaValue::String(s) => s.to_str()?,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "StackTransitionType".to_string(),
                    message: Some("Expected a string".to_string()),
                });
            }
        };

        match transition.to_lowercase().as_str() {
            "none" => Ok(StackTransitionType::None),
            "crossfade" => Ok(StackTransitionType::Crossfade),
            "slide-right" => Ok(StackTransitionType::SlideRight),
            "slide-left" => Ok(StackTransitionType::SlideLeft),
            "slide-up" => Ok(StackTransitionType::SlideUp),
            "slide-down" => Ok(StackTransitionType::SlideDown),
            "slide-left-right" => Ok(StackTransitionType::SlideLeftRight),
            "slide-up-down" => Ok(StackTransitionType::SlideUpDown),
            "over-up" => Ok(StackTransitionType::OverUp),
            "over-down" => Ok(StackTransitionType::OverDown),
            "over-left" => Ok(StackTransitionType::OverLeft),
            "over-right" => Ok(StackTransitionType::OverRight),
            "under-up" => Ok(StackTransitionType::UnderUp),
            "under-down" => Ok(StackTransitionType::UnderDown),
            "under-left" => Ok(StackTransitionType::UnderLeft),
            "under-right" => Ok(StackTransitionType::UnderRight),
            "over-up-down" => Ok(StackTransitionType::OverUpDown),
            "over-left-right" => Ok(StackTransitionType::OverLeftRight),
            "rotate-left" => Ok(StackTransitionType::RotateLeft),
            "rotate-right" => Ok(StackTransitionType::RotateRight),
            "rotate-left-right" => Ok(StackTransitionType::RotateLeftRight),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "StackTransitionType".to_string(),
                message: Some(format!("Invalid transition type: {}", transition)),
            }),
        }
    }
}

impl From<StackTransitionType> for gtk4::StackTransitionType {
    fn from(value: StackTransitionType) -> Self {
        match value {
            StackTransitionType::None => gtk4::StackTransitionType::None,
            StackTransitionType::Crossfade => gtk4::StackTransitionType::Crossfade,
            StackTransitionType::SlideRight => gtk4::StackTransitionType::SlideRight,
            StackTransitionType::SlideLeft => gtk4::StackTransitionType::SlideLeft,
            StackTransitionType::SlideUp => gtk4::StackTransitionType::SlideUp,
            StackTransitionType::SlideDown => gtk4::StackTransitionType::SlideDown,
            StackTransitionType::SlideLeftRight => gtk4::StackTransitionType::SlideLeftRight,
            StackTransitionType::SlideUpDown => gtk4::StackTransitionType::SlideUpDown,
            StackTransitionType::OverUp => gtk4::StackTransitionType::OverUp,
            StackTransitionType::OverDown => gtk4::StackTransitionType::OverDown,
            StackTransitionType::OverLeft => gtk4::StackTransitionType::OverLeft,
            StackTransitionType::OverRight => gtk4::StackTransitionType::OverRight,
            StackTransitionType::UnderUp => gtk4::StackTransitionType::UnderUp,
            StackTransitionType::UnderDown => gtk4::StackTransitionType::UnderDown,
            StackTransitionType::UnderLeft => gtk4::StackTransitionType::UnderLeft,
            StackTransitionType::UnderRight => gtk4::StackTransitionType::UnderRight,
            StackTransitionType::OverUpDown => gtk4::StackTransitionType::OverUpDown,
            StackTransitionType::OverLeftRight => gtk4::StackTransitionType::OverLeftRight,
            StackTransitionType::RotateLeft => gtk4::StackTransitionType::RotateLeft,
            StackTransitionType::RotateRight => gtk4::StackTransitionType::RotateRight,
            StackTransitionType::RotateLeftRight => gtk4::StackTransitionType::RotateLeftRight,
        }
    }
}

/// A page within a Stack widget, consisting of a name and a widget to display.
#[derive(LuaClass)]
struct StackPage {
    /// The name of the page, used to identify it when switching between pages.
    name: String,
    /// The widget to display on this page.
    widget: Box<dyn Widget>,
}

impl FromLua for StackPage {
    fn from_lua(value: LuaValue, _: &Lua) -> mlua::Result<Self> {
        let table = match &value {
            LuaValue::Table(t) => t,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "StackPage".to_string(),
                    message: Some("Expected a table".to_string()),
                });
            }
        };

        Ok(StackPage {
            name: table.get("name")?,
            widget: table.get("widget")?,
        })
    }
}

/// A container widget that can hold multiple child widgets, but only shows one at a time.
#[derive(LuaClass, WidgetBuilder)]
#[lua_class(name = "StackWidget")]
struct Stack {
    #[lua_attr(parent)]
    pub properties: Properties,
    /// The name of the currently visible page.
    pub visible_page: MaybeReactive<String>,
    /// The child widgets contained within this container.
    pub pages: Vec<StackPage>,
    /// The type of animation used when switching between pages.
    #[lua_attr(default = "none")]
    pub transition_type: MaybeReactive<StackTransitionType>,
    /// The duration of the transition animation in milliseconds.
    #[lua_attr(default = 200)]
    pub transition_duration: MaybeReactive<u32>,
    /// Whether the stack should interpolate its size when switching between pages.
    #[lua_attr(default = false)]
    pub interpolate_size: MaybeReactive<bool>,
}

impl Widget for Stack {
    fn build(&self) -> Result<gtk4::Widget> {
        let stack = GtkStack::new();
        self.properties.apply(&stack)?;

        for page in &self.pages {
            stack.add_named(&page.widget.build()?, Some(&page.name));
        }

        self.visible_page
            .bind(&stack, "visible_page", move |w, name| {
                w.set_visible_child_name(&name);
            })?;

        self.transition_type
            .bind(&stack, "transition_type", move |w, transition| {
                w.set_transition_type(transition.into());
            })?;

        self.transition_duration
            .bind(&stack, "transition_duration", move |w, duration| {
                w.set_transition_duration(duration);
            })?;

        self.interpolate_size
            .bind(&stack, "interpolate_size", move |w, interpolate| {
                w.set_interpolate_size(interpolate);
            })?;

        Ok(stack.into())
    }
}

impl FromLua for Stack {
    fn from_lua(value: LuaValue, _: &Lua) -> mlua::Result<Self> {
        let table = match &value {
            LuaValue::Table(t) => t,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "Stack".to_string(),
                    message: Some("Expected a table".to_string()),
                });
            }
        };

        Ok(Stack {
            properties: Properties::parse(table)?,
            visible_page: table.get("visible_page")?,
            pages: table.get("pages")?,
            transition_type: table
                .get::<Option<MaybeReactive<StackTransitionType>>>("transition_type")?
                .unwrap_or(MaybeReactive::Static(StackTransitionType::None)),
            transition_duration: table
                .get::<Option<MaybeReactive<u32>>>("transition_duration")?
                .unwrap_or(MaybeReactive::Static(200)),
            interpolate_size: table
                .get::<Option<MaybeReactive<bool>>>("interpolate_size")?
                .unwrap_or(MaybeReactive::Static(false)),
        })
    }
}
