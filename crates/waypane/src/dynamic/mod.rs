//! Core reactivity system for `waypane`.
//!
//! This module enables dynamic UI updates after widgets are built, by providing:
//! - **[`state`]**: Reactive data bindings (`waypane.state()`).
//! - **[`signals`]**: A global pub-sub event bus (`waypane.emitSignal()`, `waypane.onSignal()`).
//! - **[`timer`]**: GLib-backed asynchronous intervals (`waypane.setInterval()`).
//! - **[`commands`]**: Asynchronous shell command execution (`waypane.exec()`, `waypane.poll()`).
//!
//! Widget properties use the [`MaybeReactive`] bridge to seamlessly accept either static values or
//! reactive state bindings (`State` / `MutableState`) directly from the Lua configuration.

pub mod commands;
pub mod signals;
pub mod state;
pub mod timer;

use crate::lua::{LUA, stubs::LuaType};
use anyhow::{Context, Result};
use gtk4::{glib::object::IsA, prelude::WidgetExt};
use mlua::{FromLua, Lua, Value as LuaValue};
use state::{State, StateId};
use std::{borrow::Cow, cell::RefCell, rc::Rc};
use waypane_macros::{LuaClass, lua_func};

/// A widget property that is either a plain static value or bound to a reactive state handle.
pub enum MaybeReactive<T> {
    /// A plain value of type `T`, resolved at config parse time.
    Static(T),
    /// A binding to a [`State`] entry, optionally with a transform function.
    Bound(State),
}

impl<T> LuaType for MaybeReactive<T>
where
    T: LuaType,
{
    fn lua_type() -> Cow<'static, str> {
        let base_type = T::lua_type();
        format!("{} | State", base_type).into()
    }
}

impl<T> FromLua for MaybeReactive<T>
where
    T: FromLua,
{
    fn from_lua(value: LuaValue, lua: &Lua) -> mlua::Result<Self> {
        // Check if the value is a state handle (read-only or mutable)
        if let LuaValue::Table(ref t) = value
            && let Ok(state_id) = t.get::<StateId>("__state_id")
        {
            let transform = t
                .get::<Option<mlua::Function>>("__transform")
                .ok()
                .flatten()
                .map(|f| lua.create_registry_value(f))
                .transpose()?;

            return Ok(MaybeReactive::Bound(State {
                id: state_id,
                transform,
            }));
        }

        // Otherwise, parse as a static value
        let val_type = value.type_name();
        T::from_lua(value, lua)
            .map(MaybeReactive::Static)
            .map_err(|_| mlua::Error::FromLuaConversionError {
                from: val_type,
                to: std::any::type_name::<MaybeReactive<T>>().to_string(),
                message: Some("Expected a static value or a State binding".to_string()),
            })
    }
}

impl<T> MaybeReactive<T>
where
    T: FromLua + Clone + 'static,
{
    /// Binds this value to a GTK widget property.
    ///
    /// - If `Static`: the value is applied once immediately.
    /// - If `Bound`: the current state value is applied immediately, then a subscriber is
    ///   registered so future state updates automatically re-apply the property.
    ///   The subscription is cleaned up when the widget is destroyed.
    pub fn bind<W, F>(&self, widget: &W, prop_name: &'static str, mut apply_fn: F) -> Result<()>
    where
        W: IsA<gtk4::Widget>,
        F: FnMut(&W, T) + 'static,
    {
        match self {
            MaybeReactive::Static(val) => {
                apply_fn(widget, val.clone());
                Ok(())
            }
            MaybeReactive::Bound(state) => {
                let lua = LUA.get().context("Lua instance not initialized")?;

                // Apply current value immediately
                let current = state.get(lua)?;
                match T::from_lua(current, lua) {
                    Ok(val) => apply_fn(widget, val),
                    Err(e) => {
                        tracing::error!("Error reading initial state for {}: {}", prop_name, e)
                    }
                }

                // Subscribe for future updates
                let widget_clone = widget.clone();
                let transform_key = state
                    .transform
                    .as_ref()
                    .map(|k| lua.registry_value::<mlua::Function>(k))
                    .transpose()?;

                let apply_fn = RefCell::new(apply_fn);
                let subscriber: Rc<dyn Fn(LuaValue)> = Rc::new(move |raw_value: LuaValue| {
                    let Some(lua) = LUA.get() else { return };

                    let value = if let Some(ref func) = transform_key {
                        match func.call::<LuaValue>(raw_value) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::error!("Error in transform for {}: {}", prop_name, e);
                                return;
                            }
                        }
                    } else {
                        raw_value
                    };

                    match T::from_lua(value, lua) {
                        Ok(val) => {
                            if let Ok(mut f) = apply_fn.try_borrow_mut() {
                                f(&widget_clone, val);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Error converting state value for {}: {}", prop_name, e)
                        }
                    }
                });

                let state_id = state.id;
                let sub_id = state.subscribe(subscriber).ok_or_else(|| {
                    anyhow::anyhow!("State {} not found for {}", state_id, prop_name)
                })?;

                // Clean up subscription when the widget is destroyed
                widget.connect_destroy(move |_| {
                    State::unsubscribe(state_id, sub_id);
                });

                Ok(())
            }
        }
    }
}

// The following structs and functions are only used as Lua-facing stubs for cancel handles
// returned by timers and signal subscriptions. Their actual logic is implemented in their
// respective modules.

/// A handle that can be used to cancel a scheduled task or a signal subscription.
#[derive(LuaClass)]
pub struct CancelHandle {}

/// Cancels the scheduled task or signal subscription.
#[allow(dead_code)]
#[lua_func(name = "cancel", class = "CancelHandle")]
pub fn cancel() -> mlua::Result<()> {
    Ok(())
}
