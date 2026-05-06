use gpui::{AnyWindowHandle, AppContext, Context, Entity, Task};
use gpui_component::{ActiveTheme, ThemeMode};
use log::error;
use tokio::sync::{mpsc, watch};

use crate::{
    app::{Args, SettingsManager, TransportKind, serve_transport_loop, start_transport},
    ui::tray::Tray,
};

pub struct Data {
    pub settings: Args,
    pub theme_mode: Option<ThemeMode>,
    pub running_transport: Option<RunningTransport>,
    pub main_window: Option<AnyWindowHandle>,
    pub tray: Option<Entity<Tray>>,
    pub next_transport_id: u64,
}

pub struct RunningTransport {
    pub id: u64,
    pub shutdown: watch::Sender<bool>,
    pub task: Task<()>,
    pub kind: TransportKind,
    pub is_live: bool,
}

enum TransportEvent {
    Started { id: u64 },
    Stopped { id: u64, error: Option<String> },
}

impl Data {
    pub fn new(settings: Args, theme_mode: Option<ThemeMode>, tray: Option<Entity<Tray>>) -> Self {
        Self {
            settings,
            theme_mode,
            running_transport: None,
            main_window: None,
            tray,
            next_transport_id: 0,
        }
    }

    pub fn persist_settings(&self) {
        if let Err(err) = SettingsManager::save(&self.settings) {
            error!("Failed to save settings: {err}");
        }
    }

    pub fn set_theme_mode(&mut self, theme_mode: ThemeMode) {
        self.theme_mode = Some(theme_mode);
        if let Err(err) = SettingsManager::save_theme_mode(theme_mode) {
            error!("Failed to save theme settings: {err}");
        }
    }

    pub fn effective_theme_mode(&self, cx: &gpui::App) -> ThemeMode {
        self.theme_mode.unwrap_or_else(|| {
            if cx.theme().is_dark() {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            }
        })
    }

    pub fn apply_settings_change(
        &mut self,
        cx: &mut Context<Self>,
        restart_transport: bool,
        update: impl FnOnce(&mut Args),
    ) {
        update(&mut self.settings);
        self.persist_settings();

        if restart_transport && self.running_transport.is_some() {
            self.restart_transport(cx);
        } else {
            cx.notify();
        }
    }

    pub fn restart_transport(&mut self, cx: &mut Context<Self>) {
        if let Some(running_transport) = self.running_transport.take() {
            let entity = cx.entity().clone();
            cx.spawn(async move |_, cx| {
                let _ = running_transport.shutdown.send(true);
                running_transport.task.await;

                entity.update(cx, |data, cx| {
                    if data.running_transport.is_none() {
                        data.start_transport(cx);
                    }
                });
            })
            .detach();
        } else {
            self.start_transport(cx);
        }

        cx.notify();
    }

    pub fn start_transport(&mut self, cx: &mut Context<Self>) {
        if self.running_transport.is_some() {
            return;
        }

        self.persist_settings();

        self.next_transport_id += 1;
        let id = self.next_transport_id;
        let kind = self.settings.transport;
        let args = self.settings.clone();
        let (shutdown, receiver) = watch::channel(false);
        let (events_tx, mut events_rx) = mpsc::unbounded_channel();
        let task = cx.background_spawn(async move {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    let _ = events_tx.send(TransportEvent::Stopped {
                        id,
                        error: Some(format!("Failed to create transport runtime: {err}")),
                    });
                    return;
                }
            };

            let result = runtime.block_on(async {
                let transport = start_transport(&args).await?;
                let _ = events_tx.send(TransportEvent::Started { id });
                serve_transport_loop(transport, args, Some(receiver)).await?;
                Ok::<(), anyhow::Error>(())
            });

            let _ = events_tx.send(TransportEvent::Stopped {
                id,
                error: result
                    .err()
                    .map(|err| format!("Transport service exited with error: {err}")),
            });
        });

        cx.spawn(async move |this, cx| {
            while let Some(event) = events_rx.recv().await {
                let _ = this.update(cx, |data, cx| {
                    data.handle_transport_event(event, cx);
                });
            }
        })
        .detach();

        self.running_transport = Some(RunningTransport {
            id,
            shutdown,
            task,
            kind,
            is_live: false,
        });
        cx.notify();
    }

    pub fn stop_transport(&mut self) {
        if let Some(running_transport) = self.running_transport.take() {
            let _ = running_transport.shutdown.send(true);
        }
    }

    pub fn toggle_transport(&mut self, cx: &mut Context<Self>) {
        if self.running_transport.is_some() {
            self.stop_transport();
            cx.notify();
            return;
        }

        self.start_transport(cx);
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        self.stop_transport();
    }
}

impl TransportKind {
    pub fn label(self) -> &'static str {
        match self {
            #[cfg(feature = "ws")]
            Self::Ws => "WebSocket",
            #[cfg(feature = "bluetooth")]
            Self::Bluetooth => "Bluetooth",
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            #[cfg(feature = "ws")]
            Self::Ws,
            #[cfg(feature = "bluetooth")]
            Self::Bluetooth,
        ]
    }
}

impl Data {
    fn handle_transport_event(&mut self, event: TransportEvent, cx: &mut Context<Self>) {
        match event {
            TransportEvent::Started { id } => {
                if let Some(running_transport) = self.running_transport.as_mut()
                    && running_transport.id == id
                {
                    running_transport.is_live = true;
                    cx.notify();
                }
            }
            TransportEvent::Stopped { id, error } => {
                if let Some(running_transport) = self.running_transport.as_ref()
                    && running_transport.id == id
                {
                    self.running_transport = None;
                    if let Some(error) = error {
                        error!("{error}");
                    }
                    cx.notify();
                }
            }
        }
    }
}
