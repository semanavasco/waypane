use crate::lua::types::StringOrStrings;
use mlua::{Function as LuaFn, Lua, Table, Value as LuaValue};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, Mutex},
};
use waypane_macros::lua_func;

/// Type alias for a signal callback function. It takes a Lua value as input and returns nothing.
///
/// `Rc` makes callbacks cloneable so call sites can collect handles, release the
/// `RefCell` borrow, and only then invoke the callbacks. This prevents re-entrancy
/// panics when a callback tries to subscribe or unsubscribe.
pub type SignalCallback = Rc<dyn Fn(LuaValue)>;

/// Simple publish-subscribe bus for named signals.
#[derive(Default)]
pub struct SignalBus {
    listeners: HashMap<String, HashMap<usize, SignalCallback>>,
    next_id: usize,
}

impl SignalBus {
    /// Subscribes a callback to a signal name. Returns a unique subscription ID.
    pub fn subscribe(&mut self, signal: &str, cb: SignalCallback) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.listeners
            .entry(signal.to_string())
            .or_default()
            .insert(id, cb);
        id
    }

    /// Unsubscribes a listener using its signal name and subscription ID.
    pub fn unsubscribe(&mut self, signal: &str, id: usize) {
        if let Some(callbacks) = self.listeners.get_mut(signal) {
            callbacks.remove(&id);
        }
    }

    /// Returns cloned `Rc` handles for all callbacks registered for `signal`.
    pub fn callbacks_for(&self, signal: &str) -> Vec<SignalCallback> {
        self.listeners
            .get(signal)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }
}

thread_local! {
    /// Thread-local bus for broadcasting signals across the application.
    pub static SIGNAL_BUS: RefCell<SignalBus> = RefCell::new(SignalBus::default());
}

/// Listen for one or more signals and call the provided callback when they are emitted.
#[lua_func(name = "onSignal", skip = "lua", module = "waypane")]
#[arg(name = "signals", doc = "The signal or signals to listen for.")]
#[arg(
    name = "callback",
    doc = "The callback to call when the signal(s) are emitted."
)]
#[ret(
    ty = "CancelHandle",
    doc = "handle A handle that can be used to cancel the signal subscription with :cancel()."
)]
pub fn on_signal(lua: &Lua, signals: StringOrStrings, callback: LuaFn) -> mlua::Result<Table> {
    let signal_names: Vec<String> = match signals {
        StringOrStrings::Single(s) => vec![s],
        StringOrStrings::Multiple(v) => v,
    };

    let callback = Rc::new(callback);
    let mut sub_ids = Vec::new();

    for signal in &signal_names {
        let cb = callback.clone();
        let listener: Rc<dyn Fn(LuaValue)> = Rc::new(move |data: LuaValue| {
            if let Err(e) = cb.call::<LuaValue>(data) {
                tracing::error!("Error in onSignal callback: {}", e);
            }
        });

        let sub_id = SIGNAL_BUS.with(|bus| bus.borrow_mut().subscribe(signal, listener));
        sub_ids.push((signal.clone(), sub_id));
    }

    create_signal_cancel_handle(lua, sub_ids)
}

/// Emit a signal with the given name and optional data payload.
///
/// Signals in the `::` namespace are reserved for native module events (e.g.,
/// `hyprland::workspace_changed`) and cannot be emitted from Lua. Attempting to do so will result
/// in an error.
#[lua_func(name = "emitSignal", module = "waypane")]
#[arg(name = "signal", doc = "The name of the signal to emit.")]
#[arg(
    name = "data",
    doc = "Optional data to include with the signal. Can be any Lua value."
)]
pub fn emit_signal(signal: String, data: Option<LuaValue>) -> mlua::Result<()> {
    // Prevent emitting signals in the reserved `::` namespace, which is used for native module
    // events. This avoids confusion and potential conflicts between user-defined signals and
    // native events.
    if signal.contains("::") {
        return Err(mlua::Error::RuntimeError(format!(
            "Cannot emit reserved internal signal: '{}'. The '::' namespace is restricted to native modules.",
            signal
        )));
    }

    let data = data.unwrap_or(LuaValue::Nil);
    let callbacks = SIGNAL_BUS.with(|bus| bus.borrow().callbacks_for(&signal));
    for cb in callbacks {
        cb(data.clone());
    }
    Ok(())
}

/// Creates a cancel handle for signal bus subscriptions.
pub fn create_signal_cancel_handle(
    lua: &Lua,
    sub_ids: Vec<(String, usize)>,
) -> mlua::Result<Table> {
    let sub_ids = Arc::new(Mutex::new(sub_ids));
    let table = lua.create_table()?;

    let metatable = lua.create_table()?;
    let ids_ref = sub_ids.clone();
    metatable.set(
        "cancel",
        lua.create_function(move |_, _this: Table| {
            let ids = ids_ref.lock().unwrap().drain(..).collect::<Vec<_>>();
            SIGNAL_BUS.with(|bus| {
                let mut bus = bus.borrow_mut();
                for (signal, id) in ids {
                    bus.unsubscribe(&signal, id);
                }
            });
            Ok(())
        })?,
    )?;
    metatable.set("__index", metatable.clone())?;
    table.set_metatable(Some(metatable))?;

    Ok(table)
}
