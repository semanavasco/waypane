use mlua::{Function as LuaFn, Lua, Table, Value as LuaValue};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use waypane_macros::{LuaClass, lua_func};

/// Unique identifier for a [`StateEntry`] in the thread-local [`STATE_REGISTRY`].
pub type StateId = usize;

/// A subscriber callback invoked whenever the state value changes.
/// Receives the new Lua value.
pub type StateSubscriber = Rc<dyn Fn(LuaValue)>;

const STATE_ID_KEY: &str = "__state_id";
const TRANSFORM_KEY: &str = "__transform";

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

/// Internal Rust representation of a reactive state handle.
///
/// This struct is used by the bridge and widgets to track state subscriptions and
/// transformations. It is not exposed directly to Lua; instead, a Lua table with
/// a specialized metatable is used as the handle.
pub struct State {
    /// The unique ID of the state in the registry.
    pub id: StateId,
    /// An optional transformation applied to the state value when bound to a property.
    pub transform: Option<mlua::RegistryKey>,
}

/// A handle to a reactive state entry.
///
/// State handles can be bound to widget properties (e.g., `Label.text`) to create
/// reactive UIs that update automatically when the underlying data changes.
///
/// State handles support reading the current value with `:get()` and creating derived bindings
/// with `:as(transform)`.
#[derive(LuaClass)]
#[lua_class(name = "State")]
pub struct StateStub {}

/// A mutable handle to a reactive state entry.
///
/// Mutable handles support writing new values with `:set(value)`, which updates the state and
/// notifies all subscribers. This is returned by `waypane.state(initial)`.
#[derive(LuaClass)]
#[lua_class(name = "MutableState")]
#[allow(dead_code)]
pub struct MutableStateStub {
    #[lua_attr(parent)]
    _state: StateStub,
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

/// Helper function to extract state ID and apply transform if present.
fn get_state_value(lua: &Lua, this: &Table) -> mlua::Result<LuaValue> {
    let id = this.get::<StateId>(STATE_ID_KEY)?;
    let raw = STATE_REGISTRY.with(|r| r.borrow().get(id, lua))?;

    if let Some(transform) = this.get::<Option<mlua::Function>>(TRANSFORM_KEY)? {
        transform.call::<LuaValue>(raw)
    } else {
        Ok(raw)
    }
}

/// Retrieves the current value of a state.
#[lua_func(name = "get", class = "State", skip = "lua", skip = "this")]
#[ret(doc = "value The current value of the state.")]
fn state_get(lua: &Lua, this: Table) -> mlua::Result<LuaValue> {
    get_state_value(lua, &this)
}

/// Creates a new (read-only) state binding with a transform function.
#[lua_func(name = "as", class = "State", skip = "lua", skip = "this")]
#[arg(
    name = "transform",
    doc = "A function that transforms the state value and returns the transformed result."
)]
#[ret(
    ty = "State",
    doc = "state a new state handle with the transform applied."
)]
fn state_as(lua: &Lua, this: Table, transform: LuaFn) -> mlua::Result<Table> {
    let id = this.get::<StateId>(STATE_ID_KEY)?;
    let binding = create_state_table(lua, id, false)?;
    binding.set(TRANSFORM_KEY, transform)?;
    Ok(binding)
}

/// Updates the value of a mutable reactive state and notifies all subscribers.
#[lua_func(name = "set", class = "MutableState", skip = "lua", skip = "this")]
#[arg(name = "value", doc = "The new value to set for the state.")]
fn mutable_state_set(lua: &Lua, this: Table, value: LuaValue) -> mlua::Result<()> {
    let id = this.get::<StateId>(STATE_ID_KEY)?;
    State::set(lua, id, value)?;
    Ok(())
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
#[ret(ty = "MutableState", doc = "state A mutable reactive state handle.")]
pub fn state(lua: &Lua, initial: LuaValue) -> mlua::Result<Table> {
    let state = State::create(lua, initial)?;
    create_state_table(lua, state.id, true)
}

/// Helper function to create a Lua table representing a state handle, with appropriate metatable
/// methods. The table always provides `:get()` and `:as()`. When `mutable` is true, `:set()` is
/// also included.
pub fn create_state_table(lua: &Lua, id: StateId, mutable: bool) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    table.set(STATE_ID_KEY, id)?;

    let metatable = lua.create_table()?;

    metatable.set(
        "get",
        lua.create_function(move |lua, this: Table| state_get(lua, this))?,
    )?;

    metatable.set(
        "as",
        lua.create_function(move |lua, (this, transform): (Table, LuaFn)| {
            state_as(lua, this, transform)
        })?,
    )?;

    if mutable {
        metatable.set(
            "set",
            lua.create_function(move |lua, (this, value): (Table, LuaValue)| {
                mutable_state_set(lua, this, value)
            })?,
        )?;
    }

    metatable.set("__index", metatable.clone())?;
    table.set_metatable(Some(metatable))?;

    Ok(table)
}
