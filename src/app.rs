use cosmic::iced::futures::{stream, StreamExt};
use cosmic::iced::{window::Id, Color, Length, Rectangle, Subscription};
use cosmic::prelude::*;
use cosmic::surface::action::{app_popup, destroy_popup};
use cosmic::theme;
use cosmic::widget;
use cosmic::widget::MouseArea;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;
use tracing::{error, info, warn};

use crate::backend::CaffeineBackend;
use crate::service::{CaffeineManagerProxy, CaffeineService, DBUS_NAME, DBUS_PATH};
use crate::state::{CaffeineState, TimerSelection};

const ACTIVE_COLOR: Color = Color::from_rgb(0.698, 0.133, 0.133);

const SYSTEM_ICON_PATH: &str =
    "/usr/share/icons/hicolor/scalable/apps/oussama-berchi-caffeine-cosmic.svg";

const DEV_ICON_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/oussama-berchi-caffeine-cosmic.svg"
);

fn get_icon_path() -> PathBuf {
    let system_path = PathBuf::from(SYSTEM_ICON_PATH);
    if system_path.exists() {
        system_path
    } else {
        PathBuf::from(DEV_ICON_PATH)
    }
}

static ICON_HANDLE: LazyLock<widget::icon::Handle> =
    LazyLock::new(|| widget::icon::from_path(get_icon_path()).symbolic(true));

pub struct AppModel {
    core: cosmic::Core,
    selected_timer: TimerSelection,
    manual_input: String,
    caffeine_state: CaffeineState,
    popup: Option<Id>,
    proxy: Option<CaffeineManagerProxy<'static>>,
    active_icon_style: cosmic::theme::Svg,
    is_hovered: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectTimer(TimerSelection),
    ManualInputChanged(String),
    ToggleCaffeine,
    SetState(bool),
    TimerTick,
    PopupClosed(Id),
    TogglePopup(Rectangle),
    Surface(cosmic::surface::Action),
    Hover(bool),
    DBusReady(Option<CaffeineManagerProxy<'static>>),
    StateChanged(CaffeineState),
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "com.github.cosmic-caffeine";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        info!("Caffeine applet initialized");

        let active_style =
            cosmic::theme::Svg::Custom(Rc::new(|_theme| cosmic::iced_widget::svg::Style {
                color: Some(ACTIVE_COLOR),
            }));

        let app = AppModel {
            core,
            selected_timer: TimerSelection::default(),
            manual_input: "30".to_string(),
            caffeine_state: CaffeineState::inactive(),
            popup: None,
            proxy: None,
            active_icon_style: active_style,
            is_hovered: false,
        };

        let dbus_task = Task::perform(
            async move {
                let conn = match zbus::Connection::session().await {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Failed to connect to session bus: {}", e);
                        return None;
                    }
                };

                match conn.request_name(DBUS_NAME).await {
                    Ok(_) => {
                        info!("Acquired D-Bus name: {}", DBUS_NAME);
                        let backend = CaffeineBackend::new();
                        let state = Arc::new(Mutex::new(CaffeineState::inactive()));
                        let service = CaffeineService::new(backend, state);
                        if let Err(e) = conn.object_server().at(DBUS_PATH, service).await {
                            error!("Failed to serve object: {}", e);
                        }
                    }
                    Err(_) => {
                        info!("D-Bus name already taken, acting as client");
                    }
                }

                match CaffeineManagerProxy::builder(&conn)
                    .path(DBUS_PATH)
                    .ok()?
                    .destination(DBUS_NAME)
                    .ok()?
                    .build()
                    .await
                {
                    Ok(proxy) => Some(proxy),
                    Err(e) => {
                        error!("Failed to create proxy: {}", e);
                        None
                    }
                }
            },
            |proxy| cosmic::Action::App(Message::DBusReady(proxy)),
        );

        (app, dbus_task)
    }

    fn on_close_requested(&self, id: Id) -> Option<Self::Message> {
        info!("Close requested for window: {:?}", id);
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let is_active = self.caffeine_state.is_active();

        let icon_handle = ICON_HANDLE.clone();

        let suggested_size = self.core.applet.suggested_size(true);
        let (major_padding, minor_padding) = self.core.applet.suggested_padding(true);
        let (horizontal_padding, vertical_padding) = if self.core.applet.is_horizontal() {
            (major_padding, minor_padding)
        } else {
            (minor_padding, major_padding)
        };

        let scale = if self.is_hovered { 1.05 } else { 1.0 };
        let icon_width = suggested_size.0 as f32 * scale;
        let icon_height = suggested_size.1 as f32 * scale;

        let mut icon_widget = widget::icon::icon(icon_handle)
            .width(Length::Fixed(icon_width))
            .height(Length::Fixed(icon_height));

        if is_active {
            icon_widget = icon_widget.class(self.active_icon_style.clone());
        }

        let have_popup = self.popup;

        let button = widget::button::custom(
            widget::container(icon_widget)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center)
                .align_y(cosmic::iced::alignment::Vertical::Center),
        )
        .width(Length::Fixed(
            (suggested_size.0 + 2 * horizontal_padding) as f32,
        ))
        .height(Length::Fixed(
            (suggested_size.1 + 2 * vertical_padding) as f32,
        ))
        .class(cosmic::theme::Button::AppletIcon)
        .on_press_with_rectangle(move |offset, bounds| {
            if let Some(id) = have_popup {
                Message::Surface(destroy_popup(id))
            } else {
                Message::TogglePopup(Rectangle {
                    x: bounds.x - offset.x,
                    y: bounds.y - offset.y,
                    width: bounds.width,
                    height: bounds.height,
                })
            }
        });

        MouseArea::new(button)
            .on_enter(Message::Hover(true))
            .on_exit(Message::Hover(false))
            .into()
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::DBusReady(proxy) => {
                if let Some(proxy) = proxy {
                    info!("D-Bus proxy ready");
                    self.proxy = Some(proxy.clone());

                    // Initial state fetch
                    return Task::perform(
                        async move {
                            match proxy.get_state().await {
                                Ok(state) => Message::StateChanged(state),
                                Err(e) => {
                                    error!("Failed to get initial state: {}", e);
                                    Message::Hover(false)
                                }
                            }
                        },
                        cosmic::Action::App,
                    );
                }
            }

            Message::SelectTimer(selection) => {
                self.selected_timer = selection;
            }

            Message::ManualInputChanged(value) => {
                if value.chars().all(|c| c.is_ascii_digit()) {
                    self.manual_input = value;
                }
            }

            Message::ToggleCaffeine => {
                // UI button pressed (Start or Stop)
                let is_active = self.caffeine_state.is_active();
                return Task::done(cosmic::Action::App(Message::SetState(!is_active)));
            }

            Message::SetState(active) => {
                if let Some(proxy) = &self.proxy {
                    let proxy = proxy.clone();
                    let selection = self.selected_timer;
                    let manual_input = self.manual_input.clone();

                    return Task::perform(
                        async move {
                            let (idx, mins) = match selection {
                                TimerSelection::Infinity => (0, 0),
                                TimerSelection::OneHour => (1, 0),
                                TimerSelection::TwoHours => (2, 0),
                                TimerSelection::Manual => {
                                    (3, manual_input.parse::<u32>().unwrap_or(30))
                                }
                            };

                            if let Err(e) = proxy.set_state(active, idx, mins).await {
                                error!("Failed to set state via D-Bus: {}", e);
                            }
                            Message::Hover(false)
                        },
                        cosmic::Action::App,
                    );
                } else {
                    warn!("Proxy not ready, cannot toggle state");
                }
            }

            Message::StateChanged(new_state) => {
                info!("State synced from D-Bus: {:?}", new_state);
                self.caffeine_state = new_state;
            }

            Message::TimerTick => {
                // Check if the timer has expired
                if let Some(remaining) = self.caffeine_state.remaining_secs() {
                    if remaining == 0 && self.caffeine_state.is_active() {
                         info!("Timer expired, disabling caffeine");
                         return Task::done(cosmic::Action::App(Message::SetState(false)));
                    }
                }
            }

            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }

            Message::TogglePopup(anchor_rect) => {
                if let Some(main_id) = self.core.main_window_id() {
                    let action = app_popup(
                        move |state: &mut AppModel| {
                            let new_id = Id::unique();
                            state.popup = Some(new_id);

                            let mut settings = state
                                .core
                                .applet
                                .get_popup_settings(main_id, new_id, None, None, None);

                            settings.positioner.anchor_rect = Rectangle {
                                x: anchor_rect.x as i32,
                                y: anchor_rect.y as i32,
                                width: anchor_rect.width as i32,
                                height: anchor_rect.height as i32,
                            };

                            settings
                        },
                        Some(Box::new(move |state: &AppModel| {
                            build_popup_content(state).map(cosmic::Action::App)
                        })),
                    );
                    return Task::done(cosmic::Action::Cosmic(cosmic::app::Action::Surface(
                        action,
                    )));
                }
            }

            Message::Surface(action) => {
                return Task::done(cosmic::Action::Cosmic(cosmic::app::Action::Surface(action)));
            }

            Message::Hover(is_hovered) => {
                self.is_hovered = is_hovered;
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let timer = if self.caffeine_state.is_active() {
            use cosmic::iced::futures::stream;
            Subscription::run_with_id(
                "caffeine-timer",
                stream::unfold((), |()| async {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    Some((Message::TimerTick, ()))
                }),
            )
        } else {
            Subscription::none()
        };

        let dbus_signals = if let Some(proxy) = &self.proxy {
            let proxy = proxy.clone();
            Subscription::run_with_id(
                "dbus-signals",
                stream::once(async move {
                    info!("Subscribing to state signals...");
                    match proxy.inner().receive_signal("StateChanged").await {
                        Ok(stream) => {
                            info!("Successfully subscribed to state signals");
                            stream.boxed()
                        },
                        Err(e) => {
                            error!("Failed to subscribe to signals: {}", e);
                            stream::pending().boxed()
                        }
                    }
                })
                .flatten()
                .filter_map(|change: zbus::Message| async move {
                   match change.body().deserialize::<CaffeineState>() {
                       Ok(state) => {
                           info!("Received signal: {:?}", state);
                           Some(Message::StateChanged(state))
                       },
                       Err(e) => {
                           error!("Failed to parse signal body: {}", e);
                           None
                       }
                   }
                }),
            )
        } else {
            Subscription::none()
        };

        Subscription::batch(vec![timer, dbus_signals])
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}

fn build_popup_content(state: &AppModel) -> Element<'_, Message> {
    let spacing = theme::active().cosmic().spacing;
    let is_active = state.caffeine_state.is_active();

    let header = widget::text::heading("Caffeine Mode");

    let status_text = if !state.caffeine_state.is_active() {
        "Caffeine is off".to_string()
    } else {
        let selection = state.caffeine_state.selection;
        if let Some(secs) = state.caffeine_state.remaining_secs() {
            let hours = secs / 3600;
            let mins = (secs % 3600) / 60;
            if hours > 0 {
                format!("{} - {}h {}m remaining", selection.label(), hours, mins)
            } else if mins > 0 {
                format!("{} - {}m remaining", selection.label(), mins)
            } else {
                format!("{} - {}s remaining", selection.label(), secs)
            }
        } else {
            format!("{} mode active", selection.label())
        }
    };

    let status_indicator = widget::text::caption(status_text);

    let mut options = widget::column()
        .push(
            widget::radio(
                widget::text::body("Infinity"),
                TimerSelection::Infinity,
                Some(state.selected_timer),
                Message::SelectTimer,
            )
            .width(Length::Fill),
        )
        .push(
            widget::radio(
                widget::text::body("1 Hour"),
                TimerSelection::OneHour,
                Some(state.selected_timer),
                Message::SelectTimer,
            )
            .width(Length::Fill),
        )
        .push(
            widget::radio(
                widget::text::body("2 Hours"),
                TimerSelection::TwoHours,
                Some(state.selected_timer),
                Message::SelectTimer,
            )
            .width(Length::Fill),
        );

    let manual_radio = widget::radio(
        widget::text::body("Manual (min)"),
        TimerSelection::Manual,
        Some(state.selected_timer),
        Message::SelectTimer,
    );

    let manual_input = widget::text_input("Mins", &state.manual_input)
        .on_input(Message::ManualInputChanged)
        .width(Length::Fixed(80.0));

    let manual_row = widget::row()
        .push(manual_radio)
        .push(manual_input)
        .spacing(spacing.space_xs)
        .align_y(cosmic::iced::Alignment::Center);

    options = options.push(manual_row).spacing(spacing.space_xxs);

    let action_button = if is_active {
        widget::button::destructive("Stop Caffeine")
            .on_press(Message::ToggleCaffeine)
            .width(Length::Fill)
    } else {
        widget::button::suggested("Start Caffeine")
            .on_press(Message::ToggleCaffeine)
            .width(Length::Fill)
    };

    let content = widget::column()
        .push(header)
        .push(status_indicator)
        .push(widget::divider::horizontal::light())
        .push(options)
        .push(widget::divider::horizontal::light())
        .push(action_button)
        .spacing(spacing.space_s)
        .padding([spacing.space_s, spacing.space_m]);

    Element::from(state.core.applet.popup_container(content))
}
