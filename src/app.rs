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
use crate::config::{CaffeineConfig, InhibitMode};
use crate::fl;
use crate::notify;
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
    manual_hours_input: String,
    caffeine_state: CaffeineState,
    popup: Option<Id>,
    proxy: Option<CaffeineManagerProxy<'static>>,
    active_icon_style: cosmic::theme::Svg,
    is_hovered: bool,
    inhibit_mode: InhibitMode,
    warning_issued: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectTimer(TimerSelection),
    ManualInputChanged(String),
    ManualHoursChanged(String),
    ToggleCaffeine,
    SetState(bool),
    TimerTick,
    PopupClosed(Id),
    TogglePopup(Rectangle),
    Surface(cosmic::surface::Action),
    Hover(bool),
    DBusReady(Option<CaffeineManagerProxy<'static>>),
    StateChanged(CaffeineState),
    SetInhibitMode(InhibitMode),
    ClosePopup,
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

        let config = CaffeineConfig::load();
        let active_style = cosmic::theme::Svg::Custom(Rc::new(|_theme| cosmic::iced_widget::svg::Style {
            color: Some(ACTIVE_COLOR),
        }));

        info!("Loaded config: timer={:?}, manual_mins={}, auto_start={}",
            config.last_timer, config.manual_mins, config.auto_start);

        let app = AppModel {
            core,
            selected_timer: config.last_timer,
            manual_input: config.manual_mins.to_string(),
            manual_hours_input: config.manual_hours.to_string(),
            caffeine_state: CaffeineState::inactive(),
            popup: None,
            proxy: None,
            active_icon_style: active_style,
            is_hovered: false,
            inhibit_mode: config.inhibit_mode,
            warning_issued: false,
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

                tokio::time::sleep(Duration::from_millis(100)).await;

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
        let tooltip_text = self.get_tooltip_text();
        let current_state = is_active;

        let button = widget::button::custom(
            widget::container(
                widget::stack()
                    .push(icon_widget)
                    .push(
                        widget::text::caption(tooltip_text)
                            .color(cosmic::theme::active().cosmic().applet.icon_color)
                            .size(10),
                    )
                    .align_y(cosmic::iced::Alignment::Center),
            )
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
            } else if current_state {
                Message::SetState(false)
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
                self.save_config_timer();
            }

            Message::ManualInputChanged(value) => {
                if value.chars().all(|c| c.is_ascii_digit()) {
                    self.manual_input = value.clone();
                    self.save_config();
                }
            }

            Message::ManualHoursChanged(value) => {
                if value.chars().all(|c| c.is_ascii_digit()) {
                    self.manual_hours_input = value.clone();
                    self.save_config();
                }
            }

            Message::ToggleCaffeine => {
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
                                TimerSelection::FiveMins => (1, 0),
                                TimerSelection::TenMins => (2, 0),
                                TimerSelection::ThirtyMins => (3, 0),
                                TimerSelection::OneHour => (4, 0),
                                TimerSelection::TwoHours => (5, 0),
                                TimerSelection::ThreeHours => (6, 0),
                                TimerSelection::FourHours => (7, 0),
                                TimerSelection::Manual => (8, manual_input.parse::<u32>().unwrap_or(30)),
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
                self.warning_issued = false;
            }

            Message::TimerTick => {
                if let Some(remaining) = self.caffeine_state.remaining_secs() {
                    if remaining == 0 && self.caffeine_state.is_active() {
                        info!("Timer expired, disabling caffeine");
                        notify::notify_timer_expired();
                        return Task::done(cosmic::Action::App(Message::SetState(false)));
                    }

                    let config = CaffeineConfig::load();
                    let warning_threshold = (config.warning_mins as u64) * 60;
                    if remaining <= warning_threshold
                        && remaining > 0
                        && !self.warning_issued
                        && config.warning_mins > 0
                    {
                        self.warning_issued = true;
                        notify::notify_warning(config.warning_mins);
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

            Message::SetInhibitMode(mode) => {
                self.inhibit_mode = mode;
                self.save_config();
            }

            Message::ClosePopup => {
                if let Some(id) = self.popup {
                    return Task::done(cosmic::Action::App(Message::Surface(
                        destroy_popup(id),
                    )));
                }
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let timer = if self.caffeine_state.is_active() {
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
                        }
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
                        }
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

impl AppModel {
    fn get_tooltip_text(&self) -> String {
        if self.caffeine_state.is_active() {
            if let Some(secs) = self.caffeine_state.remaining_secs() {
                let hours = secs / 3600;
                let mins = (secs % 3600) / 60;
                if hours > 0 {
                    format!("{}h {}m", hours, mins)
                } else if mins > 0 {
                    format!("{}m", mins)
                } else {
                    format!("{}s", secs)
                }
            } else {
                "Active".to_string()
            }
        } else {
            "Inactive".to_string()
        }
    }

    fn save_config(&self) {
        let mut config = CaffeineConfig::load();
        config.last_timer = self.selected_timer;
        config.manual_mins = self.manual_input.parse().unwrap_or(30);
        config.manual_hours = self.manual_hours_input.parse().unwrap_or(0);
        config.inhibit_mode = self.inhibit_mode;
        let _ = config.save();
    }

    fn save_config_timer(&self) {
        let mut config = CaffeineConfig::load();
        config.last_timer = self.selected_timer;
        let _ = config.save();
    }
}

fn build_popup_content(state: &AppModel) -> Element<'_, Message> {
    let spacing = theme::active().cosmic().spacing;
    let is_active = state.caffeine_state.is_active();

    let header = widget::text::heading(fl!("caffeine-mode"));

    let status_text = if !state.caffeine_state.is_active() {
        fl!("status-off")
    } else {
        let selection = state.caffeine_state.selection;
        if let Some(secs) = state.caffeine_state.remaining_secs() {
            let hours = secs / 3600;
            let mins = (secs % 3600) / 60;
            let time_str = if hours > 0 {
                format!("{}h {}m", hours, mins)
            } else if mins > 0 {
                format!("{}m", mins)
            } else {
                format!("{}s", secs)
            };
            format!("{} - {} remaining", selection.label(), time_str)
        } else {
            format!("{} mode active", selection.label())
        }
    };

    let status_indicator = widget::text::caption(status_text);

    let total_duration = state.caffeine_state.selection.duration_secs(
        Some(state.manual_input.parse().unwrap_or(30) as u64),
        Some(state.manual_hours_input.parse().unwrap_or(0) as u64),
    );
    let progress_bar = if is_active {
        if let Some(total) = total_duration {
            if let Some(remaining) = state.caffeine_state.remaining_secs() {
                let progress = remaining as f32 / total as f32;
                Some(
                    widget::progress_bar(0.0, 1.0, progress)
                        .height(Length::Fixed(4.0))
                        .class(cosmic::theme::ProgressBar::Default),
                )
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let options = widget::column()
        .push(
            widget::radio(
                widget::text::body(fl!("timer-infinity")),
                TimerSelection::Infinity,
                Some(state.selected_timer),
                Message::SelectTimer,
            )
            .width(Length::Fill),
        )
        .push(
            widget::radio(
                widget::text::body(fl!("timer-five-mins")),
                TimerSelection::FiveMins,
                Some(state.selected_timer),
                Message::SelectTimer,
            )
            .width(Length::Fill),
        )
        .push(
            widget::radio(
                widget::text::body(fl!("timer-ten-mins")),
                TimerSelection::TenMins,
                Some(state.selected_timer),
                Message::SelectTimer,
            )
            .width(Length::Fill),
        )
        .push(
            widget::radio(
                widget::text::body(fl!("timer-thirty-mins")),
                TimerSelection::ThirtyMins,
                Some(state.selected_timer),
                Message::SelectTimer,
            )
            .width(Length::Fill),
        )
        .push(
            widget::radio(
                widget::text::body(fl!("timer-one-hour")),
                TimerSelection::OneHour,
                Some(state.selected_timer),
                Message::SelectTimer,
            )
            .width(Length::Fill),
        )
        .push(
            widget::radio(
                widget::text::body(fl!("timer-two-hours")),
                TimerSelection::TwoHours,
                Some(state.selected_timer),
                Message::SelectTimer,
            )
            .width(Length::Fill),
        )
        .push(
            widget::radio(
                widget::text::body(fl!("timer-three-hours")),
                TimerSelection::ThreeHours,
                Some(state.selected_timer),
                Message::SelectTimer,
            )
            .width(Length::Fill),
        )
        .push(
            widget::radio(
                widget::text::body(fl!("timer-four-hours")),
                TimerSelection::FourHours,
                Some(state.selected_timer),
                Message::SelectTimer,
            )
            .width(Length::Fill),
        )
        .spacing(spacing.space_xxs);

    let manual_radio = widget::radio(
        widget::text::body(fl!("timer-manual")),
        TimerSelection::Manual,
        Some(state.selected_timer),
        Message::SelectTimer,
    );

    let manual_input = widget::text_input("Mins", &state.manual_input)
        .on_input(Message::ManualInputChanged)
        .width(Length::Fixed(60.0));

    let manual_hours = widget::text_input("Hours", &state.manual_hours_input)
        .on_input(Message::ManualHoursChanged)
        .width(Length::Fixed(60.0));

    let manual_row = widget::row()
        .push(manual_radio)
        .push(manual_hours)
        .push(manual_input)
        .spacing(spacing.space_xs)
        .align_y(cosmic::iced::Alignment::Center);

    let inhibit_options = widget::row()
        .push(widget::text::body(fl!("inhibit-mode")))
        .push(
            widget::button::text(fl!("inhibit-idle"))
                .on_press(Message::SetInhibitMode(InhibitMode::Idle))
                .class(if state.inhibit_mode == InhibitMode::Idle {
                    cosmic::theme::Button::Secondary
                } else {
                    cosmic::theme::Button::Ghost
                }),
        )
        .push(
            widget::button::text(fl!("inhibit-suspend"))
                .on_press(Message::SetInhibitMode(InhibitMode::Suspend))
                .class(if state.inhibit_mode == InhibitMode::Suspend {
                    cosmic::theme::Button::Secondary
                } else {
                    cosmic::theme::Button::Ghost
                }),
        )
        .push(
            widget::button::text(fl!("inhibit-both"))
                .on_press(Message::SetInhibitMode(InhibitMode::Both))
                .class(if state.inhibit_mode == InhibitMode::Both {
                    cosmic::theme::Button::Secondary
                } else {
                    cosmic::theme::Button::Ghost
                }),
        )
        .spacing(spacing.space_xs);

    let close_button = widget::button::destructive(fl!("stop-caffeine"))
        .on_press(Message::ClosePopup)
        .width(Length::Fill);

    let action_button = if is_active {
        widget::button::destructive(fl!("stop-caffeine"))
            .on_press(Message::ToggleCaffeine)
            .width(Length::Fill)
    } else {
        widget::button::suggested(fl!("start-caffeine"))
            .on_press(Message::ToggleCaffeine)
            .width(Length::Fill)
    };

    let content = widget::column()
        .push(header)
        .push(status_indicator)
        .push(widget::divider::horizontal::light())
        .push(options)
        .push(manual_row)
        .push(widget::divider::horizontal::light())
        .push(inhibit_options)
        .push(
            widget::divider::horizontal::light(),
        )
        .push(action_button)
        .spacing(spacing.space_s)
        .padding([spacing.space_s, spacing.space_m]);

    let content = if let Some(pbar) = progress_bar {
        content.push(widget::divider::horizontal::light()).push(pbar)
    } else {
        content
    };

    Element::from(state.core.applet.popup_container(content))
}