use mlua::{Function as LuaFn, Lua, Table, Value as LuaValue};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use waypane_macros::{LuaClass, lua_func};

/// Unique identifier for a [`StateEntry`] in the thread-local [`STATE_REGISTRY`].
pub type StateId = usize;

/// A subscriber callback invoked whenever the state value changes.
/// Receives the new Lua value.
pub type StateSubscriber = Rc<dyn Fn(LuaValue)>;

/// A reactive value with its current data and list of subscribers.
struct StateEntry {
    /// The current value, stored as a key into the Lua registry so it stays rooted.
    value: mlua::RegistryKey,
    /// Callbacks to invoke when `set()` is called. Keyed by subscription ID.
    subscribers: HashMap<usize, StateSubscriber>,
    /// Auto-incrementing counter for subscriber IDs within this entry.
    next_sub_id: usize,
}

/// Thread-local store for all reactive state entries.
#[derive(Default)]
struct StateRegistry {
    entries: HashMap<StateId, StateEntry>,
    next_id: StateId,
}

impl StateRegistry {
    /// Creates a new state entry with the given initial value and returns its ID.
    fn create(&mut self, value: mlua::RegistryKey) -> StateId {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.insert(
            id,
            StateEntry {
                value,
                subscribers: HashMap::new(),
                next_sub_id: 0,
            },
        );
        id
    }

    /// Returns a clone of the current Lua value for the given state.
    fn get(&self, id: StateId, lua: &Lua) -> mlua::Result<LuaValue> {
        let entry = self
            .entries
            .get(&id)
            .ok_or_else(|| mlua::Error::external(format!("State {id} not found")))?;
        lua.registry_value::<LuaValue>(&entry.value)
    }

    /// Updates the value and notifies all subscribers. Returns cloned subscriber handles
    /// so the caller can invoke them after releasing the registry borrow (preventing re-entrancy
    /// panics).
    fn set(
        &mut self,
        id: StateId,
        lua: &Lua,
        new_value: LuaValue,
    ) -> mlua::Result<(LuaValue, Vec<StateSubscriber>)> {
        let entry = self
            .entries
            .get_mut(&id)
            .ok_or_else(|| mlua::Error::external(format!("State {id} not found")))?;

        lua.replace_registry_value(&mut entry.value, new_value.clone())?;

        let subs: Vec<StateSubscriber> = entry.subscribers.values().cloned().collect();
        Ok((new_value, subs))
    }

    /// Adds a subscriber to a state. Returns a subscription ID for later removal.
    fn subscribe(&mut self, id: StateId, cb: StateSubscriber) -> Option<usize> {
        let entry = self.entries.get_mut(&id)?;
        let sub_id = entry.next_sub_id;
        entry.next_sub_id += 1;
        entry.subscribers.insert(sub_id, cb);
        Some(sub_id)
    }

    /// Removes a subscriber from a state.
    fn unsubscribe(&mut self, id: StateId, sub_id: usize) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.subscribers.remove(&sub_id);
        }
    }
}

thread_local! {
    /// Thread-local store for all reactive state entries.
    static STATE_REGISTRY: RefCell<StateRegistry> = RefCell::new(StateRegistry::default());
}

/// A handle to a reactive state entry. Contains the state ID and an optional transform function.
/// Can be used on properties that support it (e.g. `label.text`) to provide dynamic values that
/// automatically update when the state changes.
/// Provides methods to either read, write or bind with/out a transform function.
///
/// # INTERNAL USE ONLY
/// Not intended for direct use. Should only be constructed via the `waypane.state()` Lua
/// function, which ensures the state is properly registered and rooted in the Lua registry.
#[derive(LuaClass)]
pub struct State {
    /// The ID of this state in the registry.
    pub id: StateId,
    /// An optional Lua transform function applied when this state is used as a binding.
    /// Stored as a registry key so it survives across Lua calls.
    pub transform: Option<mlua::RegistryKey>,
}

impl State {
    /// Creates a new state with the given initial Lua value.
    pub fn create(lua: &Lua, initial: LuaValue) -> mlua::Result<Self> {
        let key = lua.create_registry_value(initial)?;
        let id = STATE_REGISTRY.with(|r| r.borrow_mut().create(key));
        Ok(State {
            id,
            transform: None,
        })
    }

    /// Reads the current value, applying the transform if one is set.
    pub fn get(&self, lua: &Lua) -> mlua::Result<LuaValue> {
        let raw = STATE_REGISTRY.with(|r| r.borrow().get(self.id, lua))?;
        match &self.transform {
            Some(key) => {
                let func = lua.registry_value::<mlua::Function>(key)?;
                func.call::<LuaValue>(raw)
            }
            None => Ok(raw),
        }
    }

    /// Writes a new value and notifies all subscribers.
    pub fn set(lua: &Lua, id: StateId, new_value: LuaValue) -> mlua::Result<()> {
        let (value, subs) = STATE_REGISTRY.with(|r| r.borrow_mut().set(id, lua, new_value))?;

        // Invoke subscribers outside the borrow to avoid re-entrancy panics
        for sub in subs {
            sub(value.clone());
        }
        Ok(())
    }

    /// Subscribes a callback to this state. Returns a subscription ID for cleanup.
    pub fn subscribe(&self, cb: StateSubscriber) -> Option<usize> {
        STATE_REGISTRY.with(|r| r.borrow_mut().subscribe(self.id, cb))
    }

    /// Removes a subscription from this state.
    pub fn unsubscribe(id: StateId, sub_id: usize) {
        STATE_REGISTRY.with(|r| r.borrow_mut().unsubscribe(id, sub_id));
    }
}

/// Retrieves the current value of a reactive state.
#[lua_func(name = "get", class = "State", skip = "lua", skip = "this")]
#[ret(doc = "value The current value of the state.")]
fn state_get(lua: &Lua, this: Table) -> mlua::Result<LuaValue> {
    let id = this.get::<usize>("__state_id")?;
    let s = State {
        id,
        transform: None,
    };
    s.get(lua)
}

/// Updates the value of a reactive state and notifies all subscribers.
#[lua_func(name = "set", class = "State", skip = "lua", skip = "this")]
#[arg(name = "value", doc = "The new value to set for the state.")]
fn state_set(lua: &Lua, this: Table, value: LuaValue) -> mlua::Result<()> {
    let id = this.get::<usize>("__state_id")?;
    State::set(lua, id, value)?;
    Ok(())
}

/// Creates a new state binding with a transform function that maps the state value to a new value.
#[lua_func(name = "as", class = "State", skip = "lua", skip = "this")]
#[arg(
    name = "transform",
    doc = "A function that transforms the state value and returns the transformed result."
)]
#[ret(
    ty = "State",
    doc = "state A new state handle with the transform applied."
)]
fn state_as(lua: &Lua, this: Table, transform: LuaFn) -> mlua::Result<Table> {
    let id = this.get::<usize>("__state_id")?;
    let binding = lua.create_table()?;
    binding.set("__state_id", id)?;
    binding.set("__transform", transform)?;
    Ok(binding)
}

/// Creates a new reactive state with the given initial value.
/// Can be used on properties that support it (e.g. `label.text`) to provide dynamic values that
/// automatically update when the state changes.
///
/// # Example:
/// ```lua
/// local count = waypane.state(0)
///
/// -- ... inside layout:
/// Label({ text = count:as(function(count)
///    return "Count: " .. count
/// end) }) -- bind state to label text with transform function
///
/// -- ... somewhere else in the code:
/// waypane.setInterval(function()
///     local current = count:get() -- read current state value
///     count:set(current + 1) -- update state value, triggers UI update
/// end, 1000) -- repeat every 1000 ms
/// ```
#[lua_func(name = "state", skip = "lua", module = "waypane")]
#[arg(name = "initial", doc = "The initial value of the state.")]
#[ret(ty = "State", doc = "state A reactive state handle.")]
pub fn state(lua: &Lua, initial: LuaValue) -> mlua::Result<Table> {
    let state = State::create(lua, initial)?;
    let state_id = state.id;

    let table = lua.create_table()?;
    table.set("__state_id", state_id)?;

    let metatable = lua.create_table()?;

    metatable.set(
        "get",
        lua.create_function(move |lua, this: Table| state_get(lua, this))?,
    )?;

    metatable.set(
        "set",
        lua.create_function(move |lua, (this, value): (Table, LuaValue)| {
            state_set(lua, this, value)
        })?,
    )?;

    metatable.set(
        "as",
        lua.create_function(move |lua, (this, transform): (Table, LuaFn)| {
            state_as(lua, this, transform)
        })?,
    )?;

    metatable.set("__index", metatable.clone())?;
    table.set_metatable(Some(metatable))?;

    Ok(table)
}
