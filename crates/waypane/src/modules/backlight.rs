use crate::{
    dynamic::state::{State, StateId, create_state_table},
    lua::LUA,
};
use anyhow::Context;
use mlua::{Lua, Table, Value as LuaValue};
use notify::{EventKind, RecursiveMode, Watcher};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    thread,
};
use waypane_macros::{LuaModule, lua_func};

const BACKLIGHT_ROOT: &str = "/sys/class/backlight";

/// The `backlight` module exposes reactive screen-brightness state under `waypane.backlight`.
#[allow(dead_code)]
#[derive(LuaModule)]
#[lua_module(name = "backlight", parent = "waypane")]
struct Backlight;

struct BacklightRuntime {
    state_id: StateId,
}

static BACKLIGHT_RUNTIME: OnceLock<BacklightRuntime> = OnceLock::new();
static BACKLIGHT_RUNTIME_INIT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// Registers backlight-related Lua functions under `waypane.backlight`.
pub fn register_lua(lua: &Lua, waypane_table: &Table) -> mlua::Result<()> {
    let backlight_table = lua.create_table()?;
    backlight_table.set("level", lua.create_function(|lua, ()| level(lua))?)?;
    waypane_table.set("backlight", backlight_table)?;
    Ok(())
}

/// Finds a backlight sysfs directory (e.g., `intel_backlight`, `amdgpu_bl0`).
fn find_backlight_dir() -> Option<PathBuf> {
    let base = Path::new(BACKLIGHT_ROOT);
    let mut candidates: Vec<PathBuf> = fs::read_dir(base)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join("brightness").exists() && path.join("max_brightness").exists())
        .collect();

    candidates.sort();
    candidates.into_iter().next()
}

fn read_backlight_percentage(bright_path: &Path, max_path: &Path) -> anyhow::Result<i64> {
    let brightness_raw = fs::read_to_string(bright_path)
        .with_context(|| format!("Failed to read {}", bright_path.display()))?;
    let max_raw = fs::read_to_string(max_path)
        .with_context(|| format!("Failed to read {}", max_path.display()))?;

    let brightness: i64 = brightness_raw.trim().parse().with_context(|| {
        format!(
            "Invalid brightness value in {}: '{}'",
            bright_path.display(),
            brightness_raw.trim()
        )
    })?;
    let max: i64 = max_raw.trim().parse().with_context(|| {
        format!(
            "Invalid max_brightness value in {}: '{}'",
            max_path.display(),
            max_raw.trim()
        )
    })?;

    if max <= 0 {
        anyhow::bail!(
            "Invalid max_brightness value in {}: {}",
            max_path.display(),
            max
        );
    }

    Ok((brightness.saturating_mul(100) / max).clamp(0, 100))
}

fn send_current_level(tx: &async_channel::Sender<i64>, bright_path: &Path, max_path: &Path) {
    match read_backlight_percentage(bright_path, max_path) {
        Ok(level) => {
            if let Err(e) = tx.send_blocking(level) {
                tracing::warn!("Backlight channel closed while sending update: {}", e);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to read current backlight level: {}", e);
        }
    }
}

fn start_backlight_watcher(
    bright_path: PathBuf,
    max_path: PathBuf,
    tx: async_channel::Sender<i64>,
) {
    thread::spawn(move || {
        // Push the initial value from the watcher thread too, so late-starting runtimes can resync.
        send_current_level(&tx, &bright_path, &max_path);

        let callback_tx = tx.clone();
        let callback_bright = bright_path.clone();
        let callback_max = max_path.clone();
        let mut watcher =
            match notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
                Ok(event) => {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        send_current_level(&callback_tx, &callback_bright, &callback_max);
                    }
                }
                Err(e) => tracing::warn!("Backlight watcher error: {}", e),
            }) {
                Ok(watcher) => watcher,
                Err(e) => {
                    tracing::warn!("Failed to create backlight watcher: {}", e);
                    return;
                }
            };

        if let Err(e) = watcher.watch(&bright_path, RecursiveMode::NonRecursive) {
            tracing::warn!("Failed to watch {}: {}", bright_path.display(), e);
            return;
        }

        if let Err(e) = watcher.watch(&max_path, RecursiveMode::NonRecursive) {
            tracing::warn!("Failed to watch {}: {}", max_path.display(), e);
        }

        // Keep watcher alive for process lifetime.
        loop {
            thread::park();
        }
    });
}

fn init_backlight_runtime(lua: &Lua) -> mlua::Result<BacklightRuntime> {
    let backlight_dir = find_backlight_dir();

    let initial_level = if let Some(dir) = &backlight_dir {
        let bright_path = dir.join("brightness");
        let max_path = dir.join("max_brightness");
        match read_backlight_percentage(&bright_path, &max_path) {
            Ok(level) => level,
            Err(e) => {
                tracing::warn!("Failed to read initial backlight level: {}", e);
                0
            }
        }
    } else {
        tracing::warn!("No backlight device found in {}", BACKLIGHT_ROOT);
        0
    };

    let state = State::create(lua, LuaValue::Integer(initial_level))?;
    let state_id = state.id;

    if let Some(dir) = backlight_dir {
        let bright_path = dir.join("brightness");
        let max_path = dir.join("max_brightness");
        let (tx, rx) = async_channel::unbounded::<i64>();

        gtk4::glib::MainContext::default().spawn_local(async move {
            while let Ok(level) = rx.recv().await {
                let Some(lua) = LUA.get() else { continue };

                if let Err(e) = State::set(lua, state_id, LuaValue::Integer(level)) {
                    tracing::error!("Failed to update backlight state: {}", e);
                }
            }
        });

        start_backlight_watcher(bright_path, max_path, tx);
    }

    Ok(BacklightRuntime { state_id })
}

fn runtime(lua: &Lua) -> mlua::Result<&'static BacklightRuntime> {
    if let Some(runtime) = BACKLIGHT_RUNTIME.get() {
        return Ok(runtime);
    }

    let init_lock = BACKLIGHT_RUNTIME_INIT_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = init_lock.lock().map_err(|e| {
        mlua::Error::external(format!("Backlight runtime init lock poisoned: {}", e))
    })?;

    if let Some(runtime) = BACKLIGHT_RUNTIME.get() {
        return Ok(runtime);
    }

    let initialized = init_backlight_runtime(lua)?;
    if BACKLIGHT_RUNTIME.set(initialized).is_err() {
        tracing::warn!("Backlight runtime was initialized concurrently");
    }

    BACKLIGHT_RUNTIME
        .get()
        .ok_or_else(|| mlua::Error::external("Backlight runtime initialization failed"))
}

/// Returns a read-only state handle with current backlight percentage (0-100).
#[lua_func(name = "level", module = "waypane.backlight", skip = "lua")]
#[ret(
    ty = "State",
    doc = "state A read-only state handle containing the current backlight percentage (0-100)."
)]
pub fn level(lua: &Lua) -> mlua::Result<Table> {
    let runtime = runtime(lua)?;
    create_state_table(lua, runtime.state_id, false)
}
