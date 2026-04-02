use crate::{
    dynamic::state::{State, StateId, create_state_table},
    lua::LUA,
};
use mlua::{IntoLua, Lua, Table, Value as LuaValue};
use notify::{EventKind, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::Duration,
};
use waypane_macros::{LuaEnum, LuaModule, lua_func};

const BATTERY_ROOT: &str = "/sys/class/power_supply";

/// The `battery` module exposes many battery-related states under `waypane.battery`, such as
/// current charge percentage, charging status, and time remaining until full/empty.
#[allow(dead_code)]
#[derive(LuaModule)]
#[lua_module(name = "battery", parent = "waypane")]
struct Battery;

struct BatteryRuntime {
    present: bool,
    level_state_id: StateId,
    status_state_id: StateId,
    power_state_id: StateId,
    time_remaining_state_id: StateId,
    cycles_state_id: StateId,
    health_state_id: StateId,
    energy_state_id: StateId,
    voltage_state_id: StateId,
}

/// Battery status as reported by sysfs or aggregated across multiple batteries.
#[derive(LuaEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    NotCharging,
    /// AC adapter is online but the battery reports an unknown status.
    Plugged,
    Unknown,
}

#[derive(Debug, Clone)]
struct BatteryUpdate {
    level: u8, // 0-100
    status: BatteryStatus,
    power: f64,          // Watts
    time_remaining: f64, // Hours
    cycles: u32,
    health: f64,  // Percentage
    energy: f64,  // Wh
    voltage: f64, // Volts
}

// One runtime per source key: "" = auto-detect, "BAT0" / "BAT1" / … = explicit.
static BATTERY_RUNTIMES: OnceLock<Mutex<HashMap<String, Arc<BatteryRuntime>>>> = OnceLock::new();

fn runtimes_map() -> &'static Mutex<HashMap<String, Arc<BatteryRuntime>>> {
    BATTERY_RUNTIMES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Registers battery-related Lua functions under `waypane.battery`.
pub fn register_lua(lua: &Lua, waypane_table: &Table) -> mlua::Result<()> {
    let battery = lua.create_table()?;

    battery.set(
        "is_present",
        lua.create_function(|lua, name: Option<String>| is_present(lua, name))?,
    )?;
    battery.set(
        "level",
        lua.create_function(|lua, name: Option<String>| level(lua, name))?,
    )?;
    battery.set(
        "status",
        lua.create_function(|lua, name: Option<String>| status(lua, name))?,
    )?;
    battery.set(
        "power",
        lua.create_function(|lua, name: Option<String>| power(lua, name))?,
    )?;
    battery.set(
        "time_remaining",
        lua.create_function(|lua, name: Option<String>| time_remaining(lua, name))?,
    )?;
    battery.set(
        "cycles",
        lua.create_function(|lua, name: Option<String>| cycles(lua, name))?,
    )?;
    battery.set(
        "health",
        lua.create_function(|lua, name: Option<String>| health(lua, name))?,
    )?;
    battery.set(
        "energy",
        lua.create_function(|lua, name: Option<String>| energy(lua, name))?,
    )?;
    battery.set(
        "voltage",
        lua.create_function(|lua, name: Option<String>| voltage(lua, name))?,
    )?;

    waypane_table.set("battery", battery)
}

/// Returns `true` if `path` is a sysfs directory for a battery with at least one readable
/// energy/charge/capacity file.
fn is_valid_battery_dir(path: &Path) -> bool {
    fs::read_to_string(path.join("type"))
        .map(|s| s.trim() == "Battery")
        .unwrap_or(false)
        && (path.join("capacity").exists()
            || path.join("energy_now").exists()
            || path.join("charge_now").exists())
}

/// Resolves the set of battery sysfs directories to monitor.
///
/// When `name` is `Some("BAT0")` (or any other device name), only that directory is returned.
/// When `name` is `None`, all valid battery directories under [`BATTERY_ROOT`] are returned.
fn find_battery_dirs(name: Option<&str>) -> Vec<PathBuf> {
    if let Some(name) = name {
        let path = Path::new(BATTERY_ROOT).join(name);
        return if is_valid_battery_dir(&path) {
            vec![path]
        } else {
            vec![]
        };
    }

    let base = Path::new(BATTERY_ROOT);
    fs::read_dir(base)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| is_valid_battery_dir(p))
                .collect()
        })
        .unwrap_or_default()
}

fn find_adapter_dir() -> Option<PathBuf> {
    let base = Path::new(BATTERY_ROOT);
    fs::read_dir(base)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|path| {
            fs::read_to_string(path.join("type"))
                .map(|s| s.trim() == "Mains")
                .unwrap_or(false)
        })
}

fn read_u64(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn read_i64(path: &Path) -> Option<i64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn collect_battery_data(battery_dirs: &[PathBuf], adapter_dir: &Option<PathBuf>) -> BatteryUpdate {
    let mut total_energy_now = 0.0;
    let mut total_energy_full = 0.0;
    let mut total_energy_full_design = 0.0;
    let mut total_power_now = 0.0;
    let mut total_capacity = 0.0;
    let mut capacity_count = 0;

    let mut max_cycle_count = 0;
    let mut statuses = Vec::new();

    let mut avg_voltage = 0.0;
    let mut battery_count = 0;

    for dir in battery_dirs {
        battery_count += 1;
        let status_raw = fs::read_to_string(dir.join("status")).unwrap_or_default();
        statuses.push(status_raw.trim().to_string());

        let cycle_count = read_u64(&dir.join("cycle_count")).unwrap_or(0);
        if cycle_count > max_cycle_count {
            max_cycle_count = cycle_count;
        }

        let voltage_now = read_u64(&dir.join("voltage_now"))
            .or_else(|| read_u64(&dir.join("voltage_avg")))
            .unwrap_or(0) as f64
            / 1_000_000.0; // Volts

        avg_voltage += voltage_now;

        let energy_now = read_u64(&dir.join("energy_now"))
            .map(|v| v as f64 / 1_000_000.0) // Wh
            .or_else(|| {
                let charge_now = read_u64(&dir.join("charge_now"))? as f64 / 1_000_000.0; // Ah
                Some(charge_now * voltage_now)
            });

        let energy_full = read_u64(&dir.join("energy_full"))
            .map(|v| v as f64 / 1_000_000.0) // Wh
            .or_else(|| {
                let charge_full = read_u64(&dir.join("charge_full"))? as f64 / 1_000_000.0; // Ah
                Some(charge_full * voltage_now)
            });

        let energy_full_design = read_u64(&dir.join("energy_full_design"))
            .map(|v| v as f64 / 1_000_000.0) // Wh
            .or_else(|| {
                let charge_full_design =
                    read_u64(&dir.join("charge_full_design"))? as f64 / 1_000_000.0; // Ah
                Some(charge_full_design * voltage_now)
            });

        let power_now = read_i64(&dir.join("power_now"))
            .map(|v| v.abs() as f64 / 1_000_000.0) // W
            .or_else(|| {
                let current_now = read_i64(&dir.join("current_now"))
                    .or_else(|| read_i64(&dir.join("current_avg")))?
                    .abs() as f64
                    / 1_000_000.0; // A
                Some(current_now * voltage_now)
            });

        if let Some(cap) = read_u64(&dir.join("capacity")) {
            total_capacity += cap as f64;
            capacity_count += 1;
        }

        if let Some(en) = energy_now {
            total_energy_now += en;
        }
        if let Some(ef) = energy_full {
            total_energy_full += ef;
        }
        if let Some(efd) = energy_full_design {
            total_energy_full_design += efd;
        }
        if let Some(pn) = power_now {
            total_power_now += pn;
        }
    }

    if battery_count > 0 {
        avg_voltage /= battery_count as f64;
    }

    // Aggregate status across multiple batteries.
    // Priority: Charging > Discharging > NotCharging > Full > Unknown
    let mut status = BatteryStatus::Unknown;
    for s in &statuses {
        let current_enum = match s.as_str() {
            "Charging" => BatteryStatus::Charging,
            "Discharging" => BatteryStatus::Discharging,
            "Full" => BatteryStatus::Full,
            "Not charging" => BatteryStatus::NotCharging,
            _ => BatteryStatus::Unknown,
        };

        status = match (status, current_enum) {
            (BatteryStatus::Charging, _) | (_, BatteryStatus::Charging) => BatteryStatus::Charging,
            (BatteryStatus::Discharging, _) | (_, BatteryStatus::Discharging) => {
                BatteryStatus::Discharging
            }
            (BatteryStatus::NotCharging, _) | (_, BatteryStatus::NotCharging) => {
                BatteryStatus::NotCharging
            }
            (BatteryStatus::Full, _) | (_, BatteryStatus::Full) => BatteryStatus::Full,
            _ => BatteryStatus::Unknown,
        };
    }

    // Only promote Unknown → Plugged when the AC adapter is online. We don't override
    // Discharging (battery can discharge while plugged in under high load), nor NotCharging/Full
    if let Some(adapter) = adapter_dir {
        let online = read_u64(&adapter.join("online")).unwrap_or(0) == 1;
        if online && status == BatteryStatus::Unknown {
            status = BatteryStatus::Plugged;
        }
    }

    let level = if total_energy_full > 0.0 {
        ((total_energy_now / total_energy_full) * 100.0).round() as u8
    } else if capacity_count > 0 {
        (total_capacity / capacity_count as f64).round() as u8
    } else {
        0
    }
    .clamp(0, 100);

    let time_remaining = if total_power_now > 0.0 {
        if status == BatteryStatus::Discharging {
            total_energy_now / total_power_now
        } else if status == BatteryStatus::Charging {
            (total_energy_full - total_energy_now).max(0.0) / total_power_now
        } else {
            0.0
        }
    } else {
        0.0
    };

    let health = if total_energy_full_design > 0.0 {
        (total_energy_full / total_energy_full_design) * 100.0
    } else {
        0.0
    };

    BatteryUpdate {
        level,
        status,
        power: total_power_now,
        time_remaining,
        cycles: max_cycle_count as u32,
        health,
        energy: total_energy_now,
        voltage: avg_voltage,
    }
}

fn start_battery_watcher(
    battery_dirs: Vec<PathBuf>,
    adapter_dir: Option<PathBuf>,
    tx: async_channel::Sender<BatteryUpdate>,
) {
    thread::spawn(move || {
        let callback_tx = tx.clone();
        let callback_dirs = battery_dirs.clone();
        let callback_adapter = adapter_dir.clone();

        let mut watcher =
            match notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
                Ok(event) => {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        let update = collect_battery_data(&callback_dirs, &callback_adapter);
                        callback_tx.send_blocking(update).ok();
                    }
                }
                Err(e) => tracing::warn!("Battery watcher error: {}", e),
            }) {
                Ok(w) => w,
                Err(e) => {
                    tracing::warn!("Failed to create battery watcher: {}", e);
                    return;
                }
            };

        for dir in &battery_dirs {
            let uevent = dir.join("uevent");
            if let Err(e) = watcher.watch(&uevent, RecursiveMode::NonRecursive) {
                tracing::warn!("Failed to watch {}: {}", uevent.display(), e);
            }
        }
        if let Some(dir) = &adapter_dir {
            let uevent = dir.join("uevent");
            if let Err(e) = watcher.watch(&uevent, RecursiveMode::NonRecursive) {
                tracing::warn!("Failed to watch {}: {}", uevent.display(), e);
            }
        }

        // Periodic refresh as a fallback (every 30 seconds) for kernels or battery drivers
        // that don't reliably emit uevent notifications.
        loop {
            thread::sleep(Duration::from_secs(30));
            let update = collect_battery_data(&battery_dirs, &adapter_dir);
            tx.send_blocking(update).ok();
        }
    });
}

fn init_battery_runtime(lua: &Lua, name: Option<&str>) -> mlua::Result<BatteryRuntime> {
    let battery_dirs = find_battery_dirs(name);
    let adapter_dir = find_adapter_dir();

    let present = !battery_dirs.is_empty();
    if !present {
        match name {
            Some(n) => tracing::warn!("Battery device '{}' not found in {}", n, BATTERY_ROOT),
            None => tracing::warn!("No battery device found in {}", BATTERY_ROOT),
        }
    }

    let initial = collect_battery_data(&battery_dirs, &adapter_dir);

    let level_state = State::create(lua, LuaValue::Integer(initial.level.into()))?;
    let status_state = State::create(lua, initial.status.into_lua(lua)?)?;
    let power_state = State::create(lua, LuaValue::Number(initial.power))?;
    let time_remaining_state = State::create(lua, LuaValue::Number(initial.time_remaining))?;
    let cycles_state = State::create(lua, LuaValue::Integer(initial.cycles.into()))?;
    let health_state = State::create(lua, LuaValue::Number(initial.health))?;
    let energy_state = State::create(lua, LuaValue::Number(initial.energy))?;
    let voltage_state = State::create(lua, LuaValue::Number(initial.voltage))?;

    let level_id = level_state.id;
    let status_id = status_state.id;
    let power_id = power_state.id;
    let time_id = time_remaining_state.id;
    let cycles_id = cycles_state.id;
    let health_id = health_state.id;
    let energy_id = energy_state.id;
    let voltage_id = voltage_state.id;

    let (tx, rx) = async_channel::unbounded::<BatteryUpdate>();

    gtk4::glib::MainContext::default().spawn_local(async move {
        let mut last = initial;

        while let Ok(update) = rx.recv().await {
            let Some(lua) = LUA.get() else { continue };

            if update.level != last.level {
                State::set(lua, level_id, LuaValue::Integer(update.level.into())).ok();
            }

            if update.status != last.status {
                if let Ok(val) = update.status.into_lua(lua) {
                    State::set(lua, status_id, val).ok();
                }
            }

            if update.cycles != last.cycles {
                State::set(lua, cycles_id, LuaValue::Integer(update.cycles.into())).ok();
            }

            if (update.power - last.power).abs() > 0.01 {
                State::set(lua, power_id, LuaValue::Number(update.power)).ok();
            }

            if (update.time_remaining - last.time_remaining).abs() > 0.01 {
                State::set(lua, time_id, LuaValue::Number(update.time_remaining)).ok();
            }

            if (update.health - last.health).abs() > 0.01 {
                State::set(lua, health_id, LuaValue::Number(update.health)).ok();
            }

            if (update.energy - last.energy).abs() > 0.01 {
                State::set(lua, energy_id, LuaValue::Number(update.energy)).ok();
            }

            if (update.voltage - last.voltage).abs() > 0.01 {
                State::set(lua, voltage_id, LuaValue::Number(update.voltage)).ok();
            }

            last = update;
        }
    });

    if present || adapter_dir.is_some() {
        start_battery_watcher(battery_dirs, adapter_dir, tx);
    }

    Ok(BatteryRuntime {
        present,
        level_state_id: level_id,
        status_state_id: status_id,
        power_state_id: power_id,
        time_remaining_state_id: time_id,
        cycles_state_id: cycles_id,
        health_state_id: health_id,
        energy_state_id: energy_id,
        voltage_state_id: voltage_id,
    })
}

/// Returns (or lazily initialises) the runtime for the given source key.
///
/// `name = None` → auto-detect / aggregate all batteries (key `""`).
/// `name = Some("BAT0")` → dedicated runtime scoped to that device.
fn runtime(lua: &Lua, name: Option<&str>) -> mlua::Result<Arc<BatteryRuntime>> {
    let key = name.unwrap_or("").to_string();
    let map = runtimes_map();

    // Fast path: already initialised.
    {
        let guard = map
            .lock()
            .map_err(|e| mlua::Error::external(format!("Battery runtimes lock poisoned: {}", e)))?;
        if let Some(rt) = guard.get(&key) {
            return Ok(Arc::clone(rt));
        }
    }

    // Initialise outside the lock so we don't hold it during sysfs reads.
    let rt = Arc::new(init_battery_runtime(lua, name)?);

    // Re-acquire and insert, checking again in case of a concurrent call.
    let mut guard = map
        .lock()
        .map_err(|e| mlua::Error::external(format!("Battery runtimes lock poisoned: {}", e)))?;
    if let Some(existing) = guard.get(&key) {
        return Ok(Arc::clone(existing));
    }
    let handle = Arc::clone(&rt);
    guard.insert(key, rt);
    Ok(handle)
}

/// Returns whether a battery device was detected on this system.
///
/// Useful for conditionally rendering battery widgets on devices that may or may not have a
/// battery (e.g. desktops vs. laptops).
///
/// # Example
/// ```lua
/// if waypane.battery.is_present() then
///   -- render battery widget
/// end
///
/// -- Check for a specific device:
/// if waypane.battery.is_present("BAT1") then
///   -- render secondary battery widget
/// end
/// ```
#[lua_func(name = "is_present", module = "waypane.battery", skip = "lua")]
#[arg(
    name = "name",
    doc = "Optional battery device name (e.g. `\"BAT0\"`). Checks for any battery when omitted."
)]
#[ret(doc = "boolean `true` if the battery device was detected, `false` otherwise.")]
pub fn is_present(lua: &Lua, name: Option<String>) -> mlua::Result<bool> {
    Ok(runtime(lua, name.as_deref())?.present)
}

/// Returns a read-only state handle with current battery percentage (0-100).
#[lua_func(name = "level", module = "waypane.battery", skip = "lua")]
#[arg(
    name = "name",
    doc = "Optional battery device name (e.g. `\"BAT0\"`). Aggregates all batteries when omitted."
)]
#[ret(
    ty = "State",
    doc = "state A read-only state handle containing the current battery percentage (0-100)."
)]
pub fn level(lua: &Lua, name: Option<String>) -> mlua::Result<Table> {
    let rt = runtime(lua, name.as_deref())?;
    create_state_table(lua, rt.level_state_id, false)
}

/// Returns a read-only state handle with current battery status (e.g. `"charging"`).
#[lua_func(name = "status", module = "waypane.battery", skip = "lua")]
#[arg(
    name = "name",
    doc = "Optional battery device name (e.g. `\"BAT0\"`). Aggregates all batteries when omitted."
)]
#[ret(
    ty = "State",
    doc = "state A read-only state handle containing the current battery status (e.g. `\"charging\"`)."
)]
pub fn status(lua: &Lua, name: Option<String>) -> mlua::Result<Table> {
    let rt = runtime(lua, name.as_deref())?;
    create_state_table(lua, rt.status_state_id, false)
}

/// Returns a read-only state handle with current power draw in Watts.
#[lua_func(name = "power", module = "waypane.battery", skip = "lua")]
#[arg(
    name = "name",
    doc = "Optional battery device name (e.g. `\"BAT0\"`). Aggregates all batteries when omitted."
)]
#[ret(
    ty = "State",
    doc = "state A read-only state handle containing the current power draw in Watts."
)]
pub fn power(lua: &Lua, name: Option<String>) -> mlua::Result<Table> {
    let rt = runtime(lua, name.as_deref())?;
    create_state_table(lua, rt.power_state_id, false)
}

/// Returns a read-only state handle with time remaining in hours.
#[lua_func(name = "time_remaining", module = "waypane.battery", skip = "lua")]
#[arg(
    name = "name",
    doc = "Optional battery device name (e.g. `\"BAT0\"`). Aggregates all batteries when omitted."
)]
#[ret(
    ty = "State",
    doc = "state A read-only state handle containing the time remaining until full/empty (in hours)."
)]
pub fn time_remaining(lua: &Lua, name: Option<String>) -> mlua::Result<Table> {
    let rt = runtime(lua, name.as_deref())?;
    create_state_table(lua, rt.time_remaining_state_id, false)
}

/// Returns a read-only state handle with battery cycle count.
#[lua_func(name = "cycles", module = "waypane.battery", skip = "lua")]
#[arg(
    name = "name",
    doc = "Optional battery device name (e.g. `\"BAT0\"`). Aggregates all batteries when omitted."
)]
#[ret(
    ty = "State",
    doc = "state A read-only state handle containing the battery cycle count."
)]
pub fn cycles(lua: &Lua, name: Option<String>) -> mlua::Result<Table> {
    let rt = runtime(lua, name.as_deref())?;
    create_state_table(lua, rt.cycles_state_id, false)
}

/// Returns a read-only state handle with battery health percentage.
#[lua_func(name = "health", module = "waypane.battery", skip = "lua")]
#[arg(
    name = "name",
    doc = "Optional battery device name (e.g. `\"BAT0\"`). Aggregates all batteries when omitted."
)]
#[ret(
    ty = "State",
    doc = "state A read-only state handle containing the battery health percentage (0-100)."
)]
pub fn health(lua: &Lua, name: Option<String>) -> mlua::Result<Table> {
    let rt = runtime(lua, name.as_deref())?;
    create_state_table(lua, rt.health_state_id, false)
}

/// Returns a read-only state handle with current energy in Wh.
#[lua_func(name = "energy", module = "waypane.battery", skip = "lua")]
#[arg(
    name = "name",
    doc = "Optional battery device name (e.g. `\"BAT0\"`). Aggregates all batteries when omitted."
)]
#[ret(
    ty = "State",
    doc = "state A read-only state handle containing the current energy in Wh."
)]
pub fn energy(lua: &Lua, name: Option<String>) -> mlua::Result<Table> {
    let rt = runtime(lua, name.as_deref())?;
    create_state_table(lua, rt.energy_state_id, false)
}

/// Returns a read-only state handle with current voltage in Volts.
#[lua_func(name = "voltage", module = "waypane.battery", skip = "lua")]
#[arg(
    name = "name",
    doc = "Optional battery device name (e.g. `\"BAT0\"`). Aggregates all batteries when omitted."
)]
#[ret(
    ty = "State",
    doc = "state A read-only state handle containing the current voltage in Volts."
)]
pub fn voltage(lua: &Lua, name: Option<String>) -> mlua::Result<Table> {
    let rt = runtime(lua, name.as_deref())?;
    create_state_table(lua, rt.voltage_state_id, false)
}
