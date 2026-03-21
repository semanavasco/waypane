//! Lua runtime bootstrap and shared Lua state.
//!
//! This module is responsible for:
//! - setting up the global `waypane` Lua table with built-in helpers and runtime bindings
//! - storing the process-wide [`LUA`] instance used by dynamic bindings and event forwarding
//! - generating Lua stubs for all Rust-defined Lua classes and functions

pub mod stubs;
pub mod types;
mod waypane;

use crate::dynamic;
use anyhow::Result;
use mlua::{Lua, Table};
use std::{collections::HashSet, sync::OnceLock};
use stubs::{Module, Stub, StubFactory};

/// Global Lua instance used by dynamic bindings and modules event forwarding.
/// This is set during config loading, after the Lua environment is initialized and the config file
/// is loaded. It is expected to be initialized by the time any dynamic bindings or events are
/// used, since they can only be used in the config file which is loaded after initialization.
pub static LUA: OnceLock<Lua> = OnceLock::new();

/// Initializes the Lua environment for a new shell instance.
///
/// Loads built-in helpers (`setInterval`, `onSignal`, widget constructors) and registers all
/// runtime bindings under the global `waypane` table, including `emitSignal` and any
/// window-manager-specific subtable (e.g. `waypane.hyprland`).
///
/// Must be called before the user config is evaluated and before [`LUA`] is set.
pub fn register_lua(lua: &Lua) -> Result<()> {
    let globals = lua.globals();

    // Register widget builder functions as Lua globals
    // Each builder takes a config table, sets its `type` field, and returns it
    for factory in inventory::iter::<StubFactory> {
        if let Stub::WidgetBuilder(wb) = (factory.build)() {
            let type_name = wb.type_name;
            globals.set(
                wb.name,
                lua.create_function(move |_, config: mlua::Table| {
                    config.set("type", type_name)?;
                    Ok(config)
                })?,
            )?;
        }
    }

    let waypane = lua.create_table()?;
    globals.set("waypane", &waypane)?;

    waypane.set("state", lua.create_function(dynamic::state::state)?)?;

    waypane.set(
        "shell",
        lua.create_function(|_, config: Table| waypane::shell(config))?,
    )?;

    waypane.set(
        "setTimeout",
        lua.create_function(|lua, (cb, ms)| dynamic::timer::set_timeout(lua, cb, ms))?,
    )?;

    waypane.set(
        "setInterval",
        lua.create_function(|lua, (cb, ms)| dynamic::timer::set_interval(lua, cb, ms))?,
    )?;

    waypane.set(
        "onSignal",
        lua.create_function(|lua, (sigs, cb)| dynamic::signals::on_signal(lua, sigs, cb))?,
    )?;

    waypane.set(
        "emitSignal",
        lua.create_function(|_, (sig, data)| dynamic::signals::emit_signal(sig, data))?,
    )?;

    waypane.set(
        "exec",
        lua.create_function(|_, (cmd, cb)| dynamic::commands::exec(cmd, cb))?,
    )?;

    waypane.set(
        "poll",
        lua.create_function(|lua, (cmd, cb, ms)| dynamic::commands::poll(lua, cmd, cb, ms))?,
    )?;

    // Inject Lua bindings for the enabled modules
    // They are injected under a `waypane.<module>` table, e.g. `waypane.hyprland`
    #[cfg(any(feature = "backlight", feature = "hyprland"))]
    crate::modules::register_lua(lua, &waypane)?;

    Ok(())
}

/// Generates Lua stubs for all Lua classes and functions defined in Rust, to provide better
/// autocompletion and type hints in the user config when using an LSP that supports it.
pub fn gen_stubs() -> Result<String> {
    let mut modules = Vec::new();
    let mut enums = Vec::new();
    let mut classes = Vec::new();
    let mut functions = Vec::new();
    let mut builders = Vec::new();

    for factory in inventory::iter::<StubFactory> {
        match (factory.build)() {
            Stub::Module(m) => modules.push(m),
            Stub::Enum(e) => enums.push(e.to_string()),
            Stub::Class(c) => classes.push(c.to_string()),
            Stub::Function(f) => functions.push(f),
            Stub::WidgetBuilder(wb) => builders.push(wb.to_string()),
        }
    }

    // Compute full paths for modules, validate uniqueness and parent references, then sort by
    // dot count and format
    let mut module_entries: Vec<(&Module, String)> = modules
        .iter()
        .map(|m| {
            let full_path = match m.parent {
                None => m.name.to_string(),
                Some(parent) => format!("{}.{}", parent, m.name),
            };
            (m, full_path)
        })
        .collect();

    let mut module_path_set = HashSet::new();
    for (_, path) in &module_entries {
        if !module_path_set.insert(path.clone()) {
            anyhow::bail!("Duplicate module path: '{}'", path);
        }
    }

    for (m, _) in &module_entries {
        if let Some(parent_path) = m.parent
            && !module_path_set.contains(parent_path)
        {
            anyhow::bail!(
                "Module '{}' references unknown parent path '{}'",
                m.name,
                parent_path
            );
        }
    }

    module_entries.sort_by_key(|(_, path)| path.chars().filter(|&c| c == '.').count());

    let module_strings: Vec<String> = module_entries
        .iter()
        .map(|(m, path)| m.format_with_path(path))
        .collect();

    let mut function_strings = Vec::new();
    for f in functions {
        if let stubs::FnType::Function {
            module: Some(mod_path),
        } = f.ty
            && !module_path_set.contains(mod_path)
        {
            anyhow::bail!(
                "Function '{}' belongs to unknown module path '{}'",
                f.name,
                mod_path
            );
        }

        function_strings.push(f.to_string());
    }

    // Build final output string
    let mut out = String::new();
    out.push_str("---@meta\n\n--- This file is auto-generated by waypane. Do not edit manually.");

    if !module_strings.is_empty() {
        out.push_str("\n\n");
        out.push_str(&module_strings.join("\n\n"));
    }

    if !enums.is_empty() {
        out.push_str("\n\n");
        out.push_str(&enums.join("\n\n"));
    }

    if !classes.is_empty() {
        out.push_str("\n\n");
        out.push_str(&classes.join("\n"));
    }

    if !function_strings.is_empty() {
        out.push('\n');
        out.push_str(&function_strings.join("\n\n"));
    }

    if !builders.is_empty() {
        out.push_str("\n\n");
        out.push_str(&builders.join("\n\n"));
    }

    Ok(out)
}
