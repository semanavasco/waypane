use super::{
    Monitor, Shell, style,
    window::{Window, WindowInstance},
};
use gtk4::{
    Application, ApplicationWindow, gdk,
    gio::prelude::{ApplicationExt, ApplicationExtManual, ListModelExt},
    glib::{self, ExitCode},
    prelude::{Cast, DisplayExt, GtkWindowExt, MonitorExt, WidgetExt},
};
use gtk4_layer_shell::{Edge, LayerShell};
use std::rc::Rc;

/// Initializes and runs the GTK application based on the provided shell configuration.
pub fn run_app(shell: Shell, watch_css: bool) -> ExitCode {
    let app_id = shell.title.to_lowercase().replace(" ", "-");
    let app = Application::builder()
        .application_id(format!("com.github.semanavasco.{}", app_id))
        .build();

    // Load style on startup
    let style_path = shell.style.clone();
    app.connect_startup(move |_| {
        if let Some(style_path) = &style_path {
            tracing::info!("Loading style from {}", style_path);

            if let Err(e) = style::load(style_path, watch_css) {
                tracing::error!("Failed to load style: {}", e);
            }
        }
    });

    let shell = Rc::new(shell);
    app.connect_activate(move |app| {
        #[allow(clippy::non_minimal_cfg)]
        #[cfg(any(feature = "hyprland"))]
        crate::modules::start_listeners();

        let display = match gdk::Display::default() {
            Some(d) => d,
            None => {
                tracing::error!("No GDK display found");
                return;
            }
        };

        let monitors = display.monitors();

        // Open windows on each monitor detected at startup
        for i in 0..monitors.n_items() {
            if let Some(monitor) = monitors
                .item(i)
                .and_then(|m: glib::Object| m.downcast::<gdk::Monitor>().ok())
            {
                open_windows_for_monitor(app, &shell, &monitor);
            }
        }

        // Handle hotplugging
        let app_clone = app.clone();
        let shell_clone = Rc::clone(&shell);
        monitors.connect_items_changed(move |list: &gtk4::gio::ListModel, pos, _removed, added| {
            for i in pos..(pos + added) {
                if let Some(monitor) = list
                    .item(i)
                    .and_then(|m: glib::Object| m.downcast::<gdk::Monitor>().ok())
                {
                    open_windows_for_monitor(&app_clone, &shell_clone, &monitor);
                }
            }
        });
    });

    app.run_with_args::<&str>(&[])
}

/// Opens all windows configured for the specified monitor.
///
/// This iterates through the shell's window templates and instantiates those
/// that match the monitor's name (or all if no monitor list is specified).
fn open_windows_for_monitor(app: &Application, shell: &Shell, monitor: &gdk::Monitor) {
    let shell_monitor: Monitor = monitor.into();

    for template in &shell.windows {
        if !template.monitors.is_empty() && !template.monitors.contains(&shell_monitor.name) {
            continue;
        }

        open_window_instance(app, template, &shell_monitor, monitor);
    }
}

/// Instantiates and opens a single window instance on a specific monitor.
///
/// This handles the Lua-side instantiation of the widget tree and the
/// physical creation of the GTK/Layer-Shell window.
fn open_window_instance(
    app: &Application,
    template: &Window,
    shell_monitor: &Monitor,
    monitor: &gdk::Monitor,
) {
    let instance = match template.instantiate(shell_monitor) {
        Ok(i) => i,
        Err(e) => {
            tracing::error!(
                "Failed to instantiate window '{}' for monitor {}: {}",
                template.name,
                shell_monitor.name,
                e
            );
            return;
        }
    };

    tracing::debug!(
        "Opening window '{}' on monitor: {}",
        instance.name,
        shell_monitor.name
    );

    let window = build_window(app, &instance, monitor);

    match instance.child.build() {
        Ok(child) => {
            window.set_child(Some(&child));

            let child_clone = child.clone();
            let window_clone = window.clone();

            child.connect_visible_notify(move |_| {
                if child_clone.get_visible() {
                    window_clone.present();
                } else {
                    window_clone.hide();
                }
            });

            if child.get_visible() {
                window.present();
            }
        }
        Err(e) => tracing::error!(
            "Failed to build child widget for window '{}': {}",
            instance.name,
            e
        ),
    }

    // Close this window when its monitor is invalidated
    let window_clone = window.clone();
    let monitor_name = shell_monitor.name.clone();
    monitor.connect_invalidate(move |_| {
        tracing::debug!("Monitor {} invalidated, closing window", monitor_name);
        window_clone.close();
    });
}

/// Builds and configures a new application window based on the provided instance and monitor.
///
/// This handles the low-level GTK and Layer Shell configuration, including anchoring,
/// margins, and layer placement.
fn build_window(
    app: &Application,
    instance: &WindowInstance,
    monitor: &gdk::Monitor,
) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(&instance.name)
        .build();

    window.init_layer_shell();
    window.set_monitor(Some(monitor));

    if instance.exclusive_zone {
        window.auto_exclusive_zone_enable();
    }

    window.set_layer(instance.layer.into());

    let anchors = &instance.anchors;
    let anchor_states = [
        (Edge::Top, anchors.top),
        (Edge::Bottom, anchors.bottom),
        (Edge::Left, anchors.left),
        (Edge::Right, anchors.right),
    ];
    for (edge, state) in anchor_states {
        window.set_anchor(edge, state);
    }

    let margins = &instance.margins;
    let margin_states = [
        (Edge::Top, margins.top),
        (Edge::Bottom, margins.bottom),
        (Edge::Left, margins.left),
        (Edge::Right, margins.right),
    ];
    for (edge, state) in margin_states {
        window.set_margin(edge, state);
    }

    window
}
