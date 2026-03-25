use super::get_config_dir;
use anyhow::{Context, Result};
use gtk4::{
    CssProvider, STYLE_PROVIDER_PRIORITY_USER,
    gdk::Display,
    gio::{
        self,
        prelude::{FileExt, FileMonitorExt},
    },
    glib::clone,
    style_context_add_provider_for_display,
};
use std::path::{Path, PathBuf};

/// Loads a CSS style from the specified path and applies it to the GTK application.
/// The path can be either absolute or relative to the config file directory.
/// If `watch` is true, the function will monitor the CSS file for changes and reload it
/// automatically when it is modified.
/// Returns a `gio::FileMonitor` if watching is enabled, which should be kept alive to continue
/// monitoring.
pub fn load(path: &str, watch: bool) -> Result<Option<gio::FileMonitor>> {
    let provider = CssProvider::new();

    let path = PathBuf::from(path);

    let path = if path.is_absolute() {
        path
    } else {
        get_config_dir()?.join(path)
    };

    provider.load_from_path(&path);

    style_context_add_provider_for_display(
        &Display::default().context("Could not connect to a display")?,
        &provider,
        STYLE_PROVIDER_PRIORITY_USER,
    );

    if watch {
        let parent = path.parent().expect("CSS path has no parent");
        let filename = path.file_name().unwrap().to_os_string();

        let monitor = gio::File::for_path(parent)
            .monitor_directory(gio::FileMonitorFlags::WATCH_MOVES, gio::Cancellable::NONE)
            .expect("Failed to create CSS monitor");

        monitor.connect_changed(clone!(
            #[strong]
            provider,
            #[strong]
            path,
            move |_, file, _, event_type| {
                if file.basename().as_deref() != Some(Path::new(&filename)) {
                    return;
                }

                if event_type == gio::FileMonitorEvent::Changed {
                    tracing::debug!("CSS file changed, reloading...");
                    provider.load_from_path(&path);
                }
            }
        ));

        return Ok(Some(monitor));
    }

    Ok(None)
}
