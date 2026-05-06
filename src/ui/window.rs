use gpui::{
    AnyWindowHandle, App, AppContext, Entity, QuitMode, Styled, WindowAppearance, WindowOptions,
};
use gpui_component::{ActiveTheme, Root, Theme, ThemeMode};
use gpui_platform::application;

use crate::{
    app::{SettingsManager, TransportKind, init_logging},
    ui::{
        state::Data,
        tray::{Tray, TrayEvent},
    },
};

pub fn run() {
    init_logging();
    application()
        .with_quit_mode(QuitMode::Explicit)
        .run(|cx: &mut App| {
            gpui_component::init(cx);

            let (settings, saved_theme_mode) = SettingsManager::load_ui_settings(
                theme_mode_for_appearance(cx.window_appearance()),
            );
            if let Some(theme_mode) = saved_theme_mode {
                Theme::change(theme_mode, None, cx);
            }

            let tray = if settings.disable_tray {
                None
            } else {
                Some(cx.new(Tray::new))
            };

            let view = cx.new(|_| Data::new(settings, saved_theme_mode, tray.clone()));

            if let Some(tray_emitter) = tray.as_ref() {
                cx.subscribe(tray_emitter, {
                    let view = view.clone();
                    move |_, event, app| match event {
                        TrayEvent::ToggleWindow => toggle_window(app, view.clone()),
                        TrayEvent::Exit => quit_app(app, true),
                        TrayEvent::StartWS => start_transport(app, view.clone(), TransportKind::Ws),
                        TrayEvent::StartBluetooth => {
                            start_transport(app, view.clone(), TransportKind::Bluetooth)
                        }
                    }
                })
                .detach();
            }

            cx.on_window_closed({
                let view = view.clone();
                let tray_active = tray.is_some();
                move |app, window_id| {
                    let should_quit = view.update(app, |data, _| {
                        let is_main_window = data
                            .main_window
                            .is_some_and(|handle| handle.window_id() == window_id);
                        if is_main_window {
                            data.main_window = None;
                        }
                        is_main_window && data.tray.is_none()
                    });

                    if should_quit {
                        quit_app(app, tray_active);
                    }
                }
            })
            .detach();

            let handle = open_window(cx, view.clone());
            view.update(cx, |data, _| {
                data.main_window = Some(handle);
            });
        });
}

fn quit_app(app: &mut App, _tray_active: bool) {
    #[cfg(target_os = "linux")]
    if _tray_active {
        gtk::glib::MainContext::default().invoke(gtk::main_quit);
    }

    app.quit();
}

fn start_transport(app: &mut App, view: Entity<Data>, kind: TransportKind) {
    view.update(app, |data, cx| {
        if data.running_transport.is_some() {
            return;
        };
        data.apply_settings_change(cx, true, |settings| settings.transport = kind);
        data.start_transport(cx);
    });
}

fn toggle_window(app: &mut App, view: Entity<Data>) {
    let existing_window = view.read(app).main_window;

    if let Some(handle) = existing_window
        && handle
            .update(app, |_, window, _| window.remove_window())
            .is_ok()
    {
        return;
    }

    let handle = open_window(app, view.clone());
    view.update(app, |data, _| {
        data.main_window = Some(handle);
    });
    app.activate(true);
}

fn open_window(cx: &mut App, view: Entity<Data>) -> AnyWindowHandle {
    cx.open_window(WindowOptions::default(), |window, cx| {
        let should_follow_system_theme = view.read(cx).theme_mode.is_none();
        if should_follow_system_theme {
            Theme::sync_system_appearance(Some(window), cx);
        }

        let _appearance_subscription = if should_follow_system_theme {
            Some(window.observe_window_appearance({
                let view = view.clone();
                move |window, cx| {
                    let should_follow_system_theme = view.read(cx).theme_mode.is_none();
                    if should_follow_system_theme {
                        Theme::sync_system_appearance(Some(window), cx);
                        view.update(cx, |_, cx| {
                            cx.notify();
                        });
                    }
                }
            }))
        } else {
            None
        };
        if let Some(subscription) = _appearance_subscription {
            subscription.detach();
        }

        cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
    })
    .expect("Failed to open window")
    .into()
}

fn theme_mode_for_appearance(appearance: WindowAppearance) -> ThemeMode {
    match appearance {
        WindowAppearance::Dark | WindowAppearance::VibrantDark => ThemeMode::Dark,
        WindowAppearance::Light | WindowAppearance::VibrantLight => ThemeMode::Light,
    }
}
