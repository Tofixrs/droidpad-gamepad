use gpui::{
    AnyElement, App, AppContext, Context, Entity, FontWeight, IntoElement, ParentElement, Render,
    SharedString, Styled, Subscription, Window, div, px,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{
    Input, InputEvent, InputState, NumberInput, NumberInputEvent, StepAction,
};
use gpui_component::label::Label;
use gpui_component::menu::{DropdownMenu, PopupMenuItem};
use gpui_component::separator::Separator;
use gpui_component::setting::{
    NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_component::{ActiveTheme, Sizable, Size, StyledExt, Theme, ThemeMode};
use std::rc::Rc;

use crate::app::{Args, TransportKind};
use crate::ui::state::Data;

fn h_flex() -> gpui::Div {
    div().flex().flex_row()
}

fn v_flex() -> gpui::Div {
    div().flex().flex_col()
}

impl Render for Data {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        let settings = self.settings.clone();
        let is_starting = self
            .running_transport
            .as_ref()
            .is_some_and(|transport| !transport.is_live);
        let is_running = self
            .running_transport
            .as_ref()
            .is_some_and(|transport| transport.is_live);
        let has_transport_task = self.running_transport.is_some();
        let is_dark_mode = self.effective_theme_mode(cx).is_dark();
        let running_transport_label = self
            .running_transport
            .as_ref()
            .map(|transport| {
                if transport.is_live {
                    transport.kind.label()
                } else {
                    "Starting"
                }
            })
            .unwrap_or("Stopped");
        let transport_button_label = if has_transport_task {
            "Stop Transport"
        } else {
            "Start Transport"
        };
        let _status_color = if is_running {
            cx.theme().primary
        } else {
            cx.theme().muted_foreground
        };

        div()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                v_flex().size_full().p_6().gap_5().child(
                    v_flex()
                        .w_full()
                        .max_w(px(960.0))
                        .mx_auto()
                        .gap_5()
                        .child(
                            Label::new("DroidPad Gamepad")
                                .text_2xl()
                                .font_weight(FontWeight::BOLD),
                        )
                        .child(render_service_card(
                            view.clone(),
                            settings.transport,
                            running_transport_label,
                            transport_button_label,
                            is_starting,
                            is_running,
                            cx,
                        ))
                        .child(render_settings_shell(view, settings, is_dark_mode, cx)),
                ),
            )
    }
}

fn render_service_card(
    view: Entity<Data>,
    configured_transport: TransportKind,
    running_transport_label: &'static str,
    transport_button_label: &'static str,
    is_starting: bool,
    is_running: bool,
    cx: &mut Context<Data>,
) -> impl IntoElement {
    let transport_options = TransportKind::all();
    let transport_menu_view = view.clone();
    let transport_toggle_view = view.clone();

    v_flex()
        .w_full()
        .p_5()
        .gap_4()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().secondary)
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            Label::new("Service Status")
                                .text_sm()
                                .text_color(cx.theme().muted_foreground),
                        )
                        .child(
                            Label::new(running_transport_label)
                                .text_xl()
                                .font_weight(FontWeight::BOLD),
                        ),
                )
                .child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded(cx.theme().radius)
                        .bg(if is_running {
                            cx.theme().primary
                        } else if is_starting {
                            cx.theme().secondary_foreground
                        } else {
                            cx.theme().muted_foreground
                        })
                        .text_color(cx.theme().primary_foreground)
                        .child(if is_running {
                            "LIVE"
                        } else if is_starting {
                            "STARTING"
                        } else {
                            "IDLE"
                        }),
                ),
        )
        .child(Separator::horizontal())
        .child(
            h_flex()
                .justify_between()
                .items_end()
                .gap_4()
                .child(
                    v_flex().gap_3().child(
                        v_flex()
                            .gap_1()
                            .child(
                                Label::new("Transport")
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(
                                Button::new("transport-select")
                                    .label(configured_transport.label())
                                    .dropdown_caret(true)
                                    .outline()
                                    .dropdown_menu(move |menu, _, _| {
                                        transport_options.iter().fold(menu, |menu, transport| {
                                            menu.item(
                                                PopupMenuItem::new(transport.label())
                                                    .checked(*transport == configured_transport)
                                                    .on_click({
                                                        let view = transport_menu_view.clone();
                                                        let transport = *transport;
                                                        move |_, _, cx| {
                                                            view.update(cx, |data, cx| {
                                                                data.apply_settings_change(
                                                                    cx,
                                                                    true,
                                                                    |settings| {
                                                                        settings.transport =
                                                                            transport;
                                                                    },
                                                                );
                                                            });
                                                        }
                                                    }),
                                            )
                                        })
                                    }),
                            ),
                    ),
                )
                .child(
                    Button::new("transport-toggle")
                        .label(transport_button_label)
                        .primary()
                        .on_click(move |_, _, cx| {
                            transport_toggle_view.update(cx, |data, cx| {
                                data.toggle_transport(cx);
                            });
                        }),
                ),
        )
}

fn render_settings_shell(
    view: Entity<Data>,
    settings: Args,
    is_dark_mode: bool,
    cx: &mut Context<Data>,
) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_4()
        .child(
            Label::new("Settings")
                .text_xl()
                .font_weight(FontWeight::BOLD),
        )
        .child(
            div()
                .w_full()
                .min_h(px(520.0))
                .rounded(cx.theme().radius_lg)
                .border_1()
                .border_color(cx.theme().border)
                .overflow_hidden()
                .child(build_settings(view, settings, is_dark_mode)),
        )
}

fn build_settings(view: Entity<Data>, settings: Args, is_dark_mode: bool) -> impl IntoElement {
    let pages: Vec<SettingPage> = [
        Some(relay_page(view.clone(), &settings)),
        Some(input_page(view.clone(), &settings)),
        Some(appearance_page(view.clone(), is_dark_mode)),
        #[cfg(target_os = "windows")]
        controller_page(view.clone(), &settings),
    ]
    .into_iter()
    .flatten()
    .collect();

    Settings::new("droidpad-settings")
        .pages(pages)
        .sidebar_width(px(220.0))
        .with_size(Size::Medium)
}

fn relay_page(view: Entity<Data>, settings: &Args) -> SettingPage {
    let disable_tray = settings.disable_tray;
    let mut connection_items = vec![
        string_input_item(
            "relay-port",
            "Port",
            "TCP port used by the WebSocket transport.",
            SharedString::from(settings.port.to_string()),
            {
                let view = view.clone();
                move |value, cx| {
                    let Ok(port) = value.parse::<u16>() else {
                        return;
                    };
                    view.update(cx, |data, cx| {
                        data.apply_settings_change(cx, true, |settings| {
                            settings.port = port;
                        });
                    });
                }
            },
        ),
        SettingItem::new(
            "Disable tray",
            SettingField::<bool>::switch(move |_| disable_tray, {
                let view = view.clone();
                move |value, cx| {
                    view.update(cx, |data, cx| {
                        data.apply_settings_change(cx, false, |settings| {
                            settings.disable_tray = value;
                        });
                    });
                }
            })
            .default_value(false),
        )
        .description("Disables hiding the window to the tray"),
    ];

    if let Some(item) = bluetooth_channel_item(view.clone(), settings) {
        connection_items.push(item);
    }

    SettingPage::new("Transport").default_open(true).group(
        SettingGroup::new()
            .title("Connection")
            .items(connection_items),
    )
}

fn input_page(view: Entity<Data>, settings: &Args) -> SettingPage {
    SettingPage::new("Input")
        .description("Controller input handling behavior.")
        .group(SettingGroup::new().title("Double Tap").items([
            stepped_number_item(
                "double-tap-timing",
                "Double tap timing",
                "Milliseconds allowed between taps. Use -1 to disable the behavior.",
                settings.double_tap_timing as f64,
                NumberFieldOptions {
                    min: -1.0,
                    max: i64::MAX as f64,
                    step: 25.0,
                },
                {
                    let view = view.clone();
                    move |value, cx| {
                        let timing = value.round().max(-1.0) as i64;
                        view.update(cx, |data, cx| {
                            data.apply_settings_change(cx, true, |settings| {
                                settings.double_tap_timing = timing;
                            });
                        });
                    }
                },
            ),
            string_input_item(
                "double-tap-postfix",
                "Double tap postfix",
                "Only buttons with this suffix participate in double-tap-to-hold.",
                SharedString::from(settings.double_tap_postfix.clone()),
                {
                    let view = view.clone();
                    move |value, cx| {
                        view.update(cx, |data, cx| {
                            data.apply_settings_change(cx, true, |settings| {
                                settings.double_tap_postfix = value;
                            });
                        });
                    }
                },
            ),
        ]))
}

fn appearance_page(view: Entity<Data>, is_dark_mode: bool) -> SettingPage {
    SettingPage::new("Appearance").group(SettingGroup::new().title("Theme").items([
        SettingItem::new(
            "Dark mode",
            SettingField::<bool>::switch(move |_| is_dark_mode, {
                let view = view.clone();
                move |enabled, cx| {
                    let mode = if enabled {
                        ThemeMode::Dark
                    } else {
                        ThemeMode::Light
                    };
                    Theme::change(mode, None, cx);
                    cx.refresh_windows();
                    view.update(cx, |data, cx| {
                        data.set_theme_mode(mode);
                        cx.notify();
                    });
                }
            }),
        ),
    ]))
}

#[cfg(all(feature = "bluetooth", target_os = "linux"))]
fn bluetooth_channel_item(view: Entity<Data>, settings: &Args) -> Option<SettingItem> {
    Some(stepped_number_item(
        "bluetooth-rfcomm-channel",
        "Bluetooth RFCOMM channel",
        "RFCOMM channel used by the Linux Bluetooth transport.",
        settings.bt_channel as f64,
        NumberFieldOptions {
            min: 1.0,
            max: u8::MAX as f64,
            step: 1.0,
        },
        {
            let view = view.clone();
            move |value, cx| {
                let channel = value.round().clamp(1.0, u8::MAX as f64) as u8;
                view.update(cx, |data, cx| {
                    data.apply_settings_change(cx, true, |settings| {
                        settings.bt_channel = channel;
                    });
                });
            }
        },
    ))
}

#[cfg(not(all(feature = "bluetooth", target_os = "linux")))]
fn bluetooth_channel_item(_: Entity<Data>, _: &Args) -> Option<SettingItem> {
    None
}

#[cfg(target_os = "windows")]
fn controller_page(view: Entity<Data>, settings: &Args) -> Option<SettingPage> {
    use crate::controller::Backend;

    let backend_options = Backend::all()
        .iter()
        .map(|backend| {
            (
                SharedString::from(backend.id()),
                SharedString::from(backend.label()),
            )
        })
        .collect::<Vec<_>>();
    let backend_id = SharedString::from(settings.controller.backend.id());

    let mut items = vec![
        SettingItem::new(
            "Backend",
            SettingField::<SharedString>::dropdown(backend_options, move |_| backend_id.clone(), {
                let view = view.clone();
                move |value, cx| {
                    let Some(backend) = Backend::from_id(value.as_ref()) else {
                        return;
                    };
                    let _ = view.update(cx, |data, cx| {
                        data.apply_settings_change(cx, true, |settings| {
                            settings.controller.backend = backend;
                        });
                    });
                }
            })
            .default_value(SharedString::from(Backend::default().id())),
        )
        .description("Virtual controller backend used for outgoing desktop gamepad events."),
    ];

    if let Some(item) = vjoy_item(view.clone(), settings) {
        items.push(item);
    }

    Some(
        SettingPage::new("Controller")
            .description("Choose the virtual controller backend used on Windows.")
            .group(SettingGroup::new().title("Output").items(items)),
    )
}

#[cfg(all(target_os = "windows", feature = "vjoy"))]
fn vjoy_item(view: Entity<Data>, settings: &Args) -> Option<SettingItem> {
    Some(stepped_number_item(
        "vjoy-device",
        "vJoy Device ID",
        "Device ID to use when the vJoy backend is selected.",
        settings.controller.vjoy_device as f64,
        NumberFieldOptions {
            min: 0.0,
            max: u8::MAX as f64,
            step: 1.0,
        },
        {
            let view = view.clone();
            move |value, cx| {
                let device = value.clamp(0.0, u8::MAX as f64) as u8;
                let _ = view.update(cx, |data, cx| {
                    data.apply_settings_change(cx, true, |settings| {
                        settings.controller.vjoy_device = device;
                    });
                });
            }
        },
    ))
}

#[cfg(target_os = "windows")]
trait BackendUiExt {
    fn id(self) -> &'static str;
    fn label(self) -> &'static str;
    fn all() -> &'static [Self]
    where
        Self: Sized;
    fn from_id(value: &str) -> Option<Self>
    where
        Self: Sized;
}

#[cfg(target_os = "windows")]
impl BackendUiExt for crate::controller::Backend {
    fn id(self) -> &'static str {
        match self {
            #[cfg(feature = "vigem")]
            Self::Vigem => "vigem",
            #[cfg(feature = "vjoy")]
            Self::Vjoy => "vjoy",
        }
    }

    fn label(self) -> &'static str {
        match self {
            #[cfg(feature = "vigem")]
            Self::Vigem => "ViGEm",
            #[cfg(feature = "vjoy")]
            Self::Vjoy => "vJoy",
        }
    }

    fn all() -> &'static [Self] {
        &[
            #[cfg(feature = "vigem")]
            Self::Vigem,
            #[cfg(feature = "vjoy")]
            Self::Vjoy,
        ]
    }

    fn from_id(value: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|backend| backend.id() == value)
    }
}

struct StringInputFieldState {
    input: Entity<InputState>,
    _subscription: Subscription,
}

struct NumberInputFieldState {
    input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

fn string_input_item(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    initial_value: SharedString,
    on_change: impl Fn(String, &mut App) + 'static,
) -> SettingItem {
    let on_change = Rc::new(on_change);

    SettingItem::render(move |options, window, cx| {
        let state = window
            .use_keyed_state(
                SharedString::from(format!("string-input:{id}")),
                cx,
                |window, cx| {
                    let input = cx
                        .new(|cx| InputState::new(window, cx).default_value(initial_value.clone()));
                    let on_change = on_change.clone();
                    let subscription =
                        cx.subscribe(&input, move |_, input, event: &InputEvent, cx| {
                            if matches!(event, InputEvent::Change) {
                                on_change(input.read(cx).value().to_string(), cx);
                            }
                        });

                    StringInputFieldState {
                        input,
                        _subscription: subscription,
                    }
                },
            )
            .read(cx);

        render_setting_row(
            title,
            description,
            Input::new(&state.input)
                .with_size(options.size)
                .w_32()
                .into_any_element(),
            cx.theme().muted_foreground,
        )
    })
}

fn stepped_number_item(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    initial_value: f64,
    field_options: NumberFieldOptions,
    on_change: impl Fn(f64, &mut App) + 'static,
) -> SettingItem {
    let on_change = Rc::new(on_change);

    SettingItem::render(move |options, window, cx| {
        let state = window
            .use_keyed_state(
                SharedString::from(format!("number-input:{id}")),
                cx,
                |window, cx| {
                    let input = cx.new(|cx| {
                        InputState::new(window, cx).default_value(initial_value.to_string())
                    });
                    let step_options = field_options.clone();
                    let input_options = field_options.clone();
                    let on_change_for_step = on_change.clone();
                    let on_change_for_input = on_change.clone();
                    let subscriptions = vec![
                        cx.subscribe_in(
                            &input,
                            window,
                            move |_, input, event: &NumberInputEvent, window, cx| match event {
                                NumberInputEvent::Step(action) => input.update(cx, |input, cx| {
                                    let Ok(value) = input.value().parse::<f64>() else {
                                        return;
                                    };
                                    let updated = if *action == StepAction::Increment {
                                        value + step_options.step
                                    } else {
                                        value - step_options.step
                                    }
                                    .clamp(step_options.min, step_options.max);

                                    input.set_value(
                                        SharedString::from(updated.to_string()),
                                        window,
                                        cx,
                                    );
                                    on_change_for_step(updated, cx);
                                }),
                            },
                        ),
                        cx.subscribe_in(
                            &input,
                            window,
                            move |_, input, event: &InputEvent, window, cx| {
                                if !matches!(event, InputEvent::Change) {
                                    return;
                                }

                                input.update(cx, |input, cx| {
                                    let Ok(value) = input.value().parse::<f64>() else {
                                        return;
                                    };
                                    let clamped = value.clamp(input_options.min, input_options.max);
                                    if (clamped - value).abs() > f64::EPSILON {
                                        input.set_value(
                                            SharedString::from(clamped.to_string()),
                                            window,
                                            cx,
                                        );
                                    }
                                    on_change_for_input(clamped, cx);
                                });
                            },
                        ),
                    ];

                    NumberInputFieldState {
                        input,
                        _subscriptions: subscriptions,
                    }
                },
            )
            .read(cx);

        render_setting_row(
            title,
            description,
            NumberInput::new(&state.input)
                .with_size(options.size)
                .w_32()
                .into_any_element(),
            cx.theme().muted_foreground,
        )
    })
}

fn render_setting_row(
    title: &'static str,
    description: &'static str,
    field: AnyElement,
    muted_foreground: gpui::Hsla,
) -> impl IntoElement {
    div()
        .w_full()
        .h_flex()
        .justify_between()
        .items_start()
        .gap_3()
        .child(
            v_flex()
                .flex_1()
                .max_w_3_5()
                .gap_1()
                .child(Label::new(title).text_sm())
                .child(
                    div()
                        .text_sm()
                        .text_color(muted_foreground)
                        .child(description),
                ),
        )
        .child(field)
}
