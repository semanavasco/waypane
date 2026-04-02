#[allow(clippy::non_minimal_cfg)]
#[cfg(any(feature = "hyprland"))]
pub mod wm;

#[cfg(feature = "backlight")]
pub mod backlight;

#[cfg(feature = "battery")]
pub mod battery;

/// Registers Lua bindings for all enabled modules under the global `waypane` table.
///
/// This is called during Lua runtime initialization. Each enabled module injects its own
/// subtable/functions (for example, `waypane.backlight` and `waypane.hyprland`).
#[cfg(any(feature = "backlight", feature = "battery", feature = "hyprland"))]
pub fn register_lua(lua: &mlua::Lua, waypane_table: &mlua::Table) -> anyhow::Result<()> {
    #[cfg(feature = "backlight")]
    backlight::register_lua(lua, waypane_table)?;

    #[cfg(feature = "hyprland")]
    wm::register_lua(lua, waypane_table)?;

    #[cfg(feature = "battery")]
    battery::register_lua(lua, waypane_table)?;

    Ok(())
}

#[allow(clippy::non_minimal_cfg)]
#[cfg(any(feature = "hyprland"))]
/// Starts background listeners for enabled modules that require runtime event loops.
///
/// This is called from the shell lifecycle during GTK application activation.
/// Modules that are query-only (such as backlight) do not need startup hooks here.
pub fn start_listeners() {
    #[cfg(feature = "hyprland")]
    wm::start_listener();
}
