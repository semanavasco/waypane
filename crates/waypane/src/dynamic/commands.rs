use super::timer::create_cancel_handle;
use gtk4::glib;
use mlua::{Function as LuaFn, Lua, Table as LuaTable};
use std::{process::Command, sync::Arc, thread, time::Duration};
use waypane_macros::lua_func;

/// Executes a shell command asynchronously.
///
/// If a callback is provided, it will be called with the command's stdout and stderr once it
/// finishes.
#[lua_func(name = "exec", module = "waypane")]
#[arg(name = "cmd", doc = "The shell command to execute.")]
#[arg(
    name = "callback",
    doc = "Optional callback function(stdout, stderr) to call when the command finishes."
)]
pub fn exec(cmd: String, callback: Option<LuaFn>) -> mlua::Result<()> {
    if let Some(cb) = callback {
        let (tx, rx) = async_channel::bounded::<(String, String)>(1);

        gtk4::glib::MainContext::default().spawn_local(async move {
            if let Ok((stdout, stderr)) = rx.recv().await
                && let Err(e) = cb.call::<()>((stdout, stderr))
            {
                tracing::error!("Error in exec callback: {}", e);
            }
        });

        thread::spawn(move || {
            let output = Command::new("sh").arg("-c").arg(cmd).output();
            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                    if let Err(e) = tx.send_blocking((stdout, stderr)) {
                        tracing::error!("Error sending command output: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("Error executing command: {}", e);
                }
            }
        });
    } else {
        thread::spawn(move || {
            if let Err(e) = Command::new("sh").arg("-c").arg(cmd).status() {
                tracing::error!("Error executing command: {}", e);
            }
        });
    }
    Ok(())
}

/// Polls a shell command at a regular interval and calls the provided callback.
#[lua_func(name = "poll", module = "waypane", skip = "lua")]
#[arg(name = "cmd", doc = "The shell command to execute.")]
#[arg(
    name = "callback",
    doc = "The callback function(stdout, stderr) to call after each poll."
)]
#[arg(name = "interval", doc = "The polling interval in milliseconds.")]
#[ret(
    ty = "CancelHandle",
    doc = "handle A handle that can be used to cancel the poll with :cancel()."
)]
pub fn poll(lua: &Lua, cmd: String, callback: LuaFn, interval: u64) -> mlua::Result<LuaTable> {
    let cmd = Arc::new(cmd);
    let (tx, rx) = async_channel::unbounded::<(String, String)>();

    glib::MainContext::default().spawn_local(async move {
        while let Ok((stdout, stderr)) = rx.recv().await {
            if let Err(e) = callback.call::<()>((stdout, stderr)) {
                tracing::error!("Error in poll callback: {}", e);
            }
        }
    });

    let run_worker = {
        let tx = tx.clone();
        let cmd = cmd.clone();

        move || {
            let tx_clone = tx.clone();
            let cmd_clone = cmd.clone();

            thread::spawn(move || {
                let output = Command::new("sh").arg("-c").arg(&*cmd_clone).output();

                match output {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                        if let Err(e) = tx_clone.send_blocking((stdout, stderr)) {
                            tracing::error!("Error sending poll results: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error executing poll command: {}", e);
                    }
                }
            });
        }
    };

    run_worker();
    let source_id = glib::timeout_add_local(Duration::from_millis(interval), move || {
        run_worker();
        glib::ControlFlow::Continue
    });

    create_cancel_handle(lua, source_id)
}
