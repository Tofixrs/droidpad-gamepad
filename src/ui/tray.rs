use gpui::{AppContext, Context, EventEmitter};
use log::{error, warn};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem},
};

pub struct Tray;

pub enum TrayEvent {
    StartWS,
    StartBluetooth,
    ToggleWindow,
    Exit,
}

impl EventEmitter<TrayEvent> for Tray {}

static TOGGLE_WINDOW_ID: &str = "toggle_window";
static EXIT_ID: &str = "exit";
static START_WS: &str = "start_ws";
static START_BLUETOOTH: &str = "start_bluetooth";

impl Tray {
    fn new_tray_icon() -> Option<TrayIcon> {
        let tray_menu = Menu::new();
        let start_ws = MenuItem::with_id(START_WS, "Start websocket", true, None);
        let start_bluetooth = MenuItem::with_id(START_BLUETOOTH, "Start bluetooth", true, None);
        let toggle_item = MenuItem::with_id(TOGGLE_WINDOW_ID, "Toggle Window", true, None);
        let exit_item = MenuItem::with_id(EXIT_ID, "Exit", true, None);
        if let Err(err) = tray_menu.append_items(&[
            &start_ws,
            &start_bluetooth,
            &PredefinedMenuItem::separator(),
            &toggle_item,
            &exit_item,
        ]) {
            error!("Failed to create tray menu: {err}");
            return None;
        }

        match TrayIconBuilder::new()
            .with_icon(icon())
            .with_menu(Box::new(tray_menu))
            .build()
        {
            Ok(icon) => Some(icon),
            Err(err) => {
                error!("Failed to create tray icon: {err}");
                None
            }
        }
    }
    pub fn new(cx: &mut Context<Self>) -> Self {
        std::thread::spawn(move || {
            use log::info;
            info!("Spawning bg thread for tray");
            #[cfg(target_os = "linux")]
            if let Err(err) = gtk::init() {
                warn!("Failed to initialize GTK: {err}");
                return;
            }

            let _icon = match Tray::new_tray_icon() {
                Some(icon) => icon,
                None => return,
            };

            #[cfg(target_os = "linux")]
            {
                gtk::main();
            }
            #[cfg(target_os = "windows")]
            {
                use windows::Win32::UI::WindowsAndMessaging::{
                    DispatchMessageW, GetMessageW, MSG, TranslateMessage,
                };

                let mut msg = MSG::default();
                unsafe {
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        let _ = DispatchMessageW(&msg);
                    }
                }
            }
        });

        cx.spawn(async |this, cx| {
            let (menu_tx, menu_rx) = tokio::sync::mpsc::unbounded_channel::<TrayEvent>();
            cx.background_spawn(async move {
                while let Ok(evt) = MenuEvent::receiver().recv() {
                    let res = match evt.id {
                        v if v == START_WS => menu_tx.send(TrayEvent::StartWS),
                        v if v == START_BLUETOOTH => menu_tx.send(TrayEvent::StartBluetooth),
                        v if v == TOGGLE_WINDOW_ID => menu_tx.send(TrayEvent::ToggleWindow),
                        v if v == EXIT_ID => menu_tx.send(TrayEvent::Exit),
                        MenuId(_) => Ok(()),
                    };

                    if let Err(err) = res {
                        error!("{err:#?}");
                        break;
                    }
                }
            })
            .detach();

            let mut menu_rx = menu_rx;
            while let Some(event) = menu_rx.recv().await {
                if this.update(cx, |_, cx| cx.emit(event)).is_err() {
                    warn!("Tray entity dropped while handling tray event");
                    break;
                }
            }

            warn!("Tray event channel closed");
        })
        .detach();

        Self
    }
}

fn icon() -> Icon {
    let img =
        image::load_from_memory(include_bytes!("../../res/icon.png")).expect("Failed to load icon");
    Icon::from_rgba(img.to_rgba8().to_vec(), img.width(), img.height())
        .expect("Failed to load icon")
}
