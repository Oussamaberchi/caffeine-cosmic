use cosmic::iced::widget::MouseArea;
use cosmic::iced::{futures, window::Id, Color, Length, Rectangle, Subscription};
use cosmic::prelude::*;
use cosmic::surface::action::{app_popup, destroy_popup};
use cosmic::theme;
use cosmic::widget;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::LazyLock;
use std::time::Duration;
use tracing::{error, info, warn};

use crate::backend::CaffeineBackend;

const ACTIVE_COLOR: Color = Color::from_rgb(0.698, 0.133, 0.133);

const SYSTEM_ICON_PATH: &str =
    "/usr/share/icons/hicolor/scalable/apps/caffeine.svg";

const DEV_ICON_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/caffeine-cup-symbolic.svg"
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimerSelection {
    #[default]
    Infinity,
    OneHour,
    TwoHours,
    Manual,
}

impl TimerSelection {
    fn label(&self) -> &'static str {
        match self {
            TimerSelection::Infinity => "Infinity",
            TimerSelection::OneHour => "1 Hour",
            TimerSelection::TwoHours => "2 Hours",
            TimerSelection::Manual => "Manual",
        }
    }

    fn duration_secs(&self, manual_mins: Option<u64>) -> Option<u64> {
        match self {
            TimerSelection::Infinity => None,
            TimerSelection::OneHour => Some(3600),
            TimerSelection::TwoHours => Some(7200),
            TimerSelection::Manual => manual_mins.map(|m| m * 60),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaffeineState {
    #[default]
    Inactive,
    Active {
        selection: TimerSelection,
        remaining_secs: Option<u64>,
    },
}

impl CaffeineState {
    fn is_active(&self) -> bool {
        matches!(self, CaffeineState::Active { .. })
    }
}

pub struct AppModel {
    core: cosmic::Core,
    selected_timer: TimerSelection,
    manual_input: String,
    caffeine_state: CaffeineState,
    popup: Option<Id>,
    backend: CaffeineBackend,
    active_icon_style: cosmic::theme::Svg,
    is_hovered: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectTimer(TimerSelection),
    ManualInputChanged(String),
    StartCaffeine,
    StopCaffeine,
    TimerTick,
    TimerExpired,
    PopupClosed(Id),
    Surface(cosmic::surface::Action),
    Hover(bool),
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
            caffeine_state: CaffeineState::Inactive,
            popup: None,
            backend: CaffeineBackend::new(),
            active_icon_style: active_style,
            is_hovered: false,
        };
        (app, Task::none())
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

        let have_popup = self.popup.clone();

        let button =
            widget::button::custom(
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
                        Message::Surface(app_popup::<AppModel>(
                            move |state: &mut AppModel| {
                                let new_id = Id::unique();
                                state.popup = Some(new_id);

                                let mut settings = state.core.applet.get_popup_settings(
                                    state.core.main_window_id().unwrap(),
                                    new_id,
                                    None,
                                    None,
                                    None,
                                );

                                settings.positioner.anchor_rect = Rectangle {
                                    x: (bounds.x - offset.x) as i32,
                                    y: (bounds.y - offset.y) as i32,
                                    width: bounds.width as i32,
                                    height: bounds.height as i32,
                                };

                                settings
                            },
                            Some(Box::new(move |state: &AppModel| {
                                build_popup_content(state).map(cosmic::Action::App)
                            })),
                        ))
                    }
                });

        MouseArea::new(button)
            .on_enter(Message::Hover(true))
            .on_exit(Message::Hover(false))
            .into()
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        info!("Received Message: {:?}", message);

        match message {
            Message::SelectTimer(selection) => {
                info!("Timer selection changed to: {:?}", selection);
                self.selected_timer = selection;
            }

            Message::ManualInputChanged(value) => {
                if value.chars().all(|c| c.is_ascii_digit()) {
                    self.manual_input = value;
                }
            }

            Message::StartCaffeine => {
                info!(
                    "Starting caffeine with selection: {:?}",
                    self.selected_timer
                );
                let selection = self.selected_timer;

                let manual_mins = if selection == TimerSelection::Manual {
                    match self.manual_input.parse::<u64>() {
                        Ok(m) if m > 0 => Some(m),
                        _ => {
                            warn!("Invalid manual input, defaulting to 30 mins");
                            Some(30)
                        }
                    }
                } else {
                    None
                };

                let duration_secs = selection.duration_secs(manual_mins);

                self.caffeine_state = CaffeineState::Active {
                    selection,
                    remaining_secs: duration_secs,
                };

                let backend = self.backend.clone();
                let reason = match selection {
                    TimerSelection::Infinity => "User enabled infinity caffeine mode".to_string(),
                    TimerSelection::OneHour => "User enabled 1-hour caffeine timer".to_string(),
                    TimerSelection::TwoHours => "User enabled 2-hour caffeine timer".to_string(),
                    TimerSelection::Manual => format!(
                        "User enabled {}-minute caffeine timer",
                        manual_mins.unwrap_or(30)
                    ),
                };

                tokio::spawn(async move {
                    if let Err(e) = backend.inhibit(&reason).await {
                        error!("Failed to inhibit: {}", e);
                    }
                });
            }

            Message::StopCaffeine => {
                info!("Stopping caffeine");
                self.caffeine_state = CaffeineState::Inactive;

                let backend = self.backend.clone();
                tokio::spawn(async move {
                    if let Err(e) = backend.uninhibit().await {
                        error!("Failed to uninhibit: {}", e);
                    }
                });
            }

            Message::TimerTick => {
                if let CaffeineState::Active {
                    remaining_secs: Some(secs),
                    ..
                } = &mut self.caffeine_state
                {
                    if *secs > 0 {
                        *secs -= 1;
                    }
                    if *secs == 0 {
                        info!("Timer expired, stopping caffeine");
                        return Task::done(cosmic::Action::App(Message::TimerExpired));
                    }
                }
            }

            Message::TimerExpired => {
                info!("Timer expired message received");
                self.caffeine_state = CaffeineState::Inactive;

                let backend = self.backend.clone();
                tokio::spawn(async move {
                    if let Err(e) = backend.uninhibit().await {
                        error!("Failed to uninhibit: {}", e);
                    }
                });
            }

            Message::PopupClosed(id) => {
                info!("Popup closed: {:?}", id);
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }

            Message::Surface(action) => {
                info!("Surface action received");
                return Task::done(cosmic::Action::Cosmic(cosmic::app::Action::Surface(action)));
            }

            Message::Hover(is_hovered) => {
                self.is_hovered = is_hovered;
            }
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        match &self.caffeine_state {
            CaffeineState::Active {
                remaining_secs: Some(secs),
                ..
            } if *secs > 0 => timer_subscription(),
            _ => Subscription::none(),
        }
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }
}

fn build_popup_content(state: &AppModel) -> Element<'_, Message> {
    let spacing = theme::active().cosmic().spacing;
    let is_active = state.caffeine_state.is_active();

    let header = widget::text::heading("Caffeine Mode");

    let status_text = match &state.caffeine_state {
        CaffeineState::Inactive => "Caffeine is off".to_string(),
        CaffeineState::Active {
            selection,
            remaining_secs,
        } => match remaining_secs {
            Some(secs) => {
                let mins = secs / 60;
                let hours = mins / 60;
                let mins = mins % 60;
                if hours > 0 {
                    format!("{} - {}h {}m remaining", selection.label(), hours, mins)
                } else if mins > 0 {
                    format!("{} - {}m remaining", selection.label(), mins)
                } else {
                    format!("{} - {}s remaining", selection.label(), secs)
                }
            }
            None => format!("{} mode active", selection.label()),
        },
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
            .on_press(Message::StopCaffeine)
            .width(Length::Fill)
    } else {
        widget::button::suggested("Start Caffeine")
            .on_press(Message::StartCaffeine)
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

fn timer_subscription() -> Subscription<Message> {
    use futures::stream;

    Subscription::run_with_id(
        "caffeine-timer",
        stream::unfold((), |()| async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Some((Message::TimerTick, ()))
        }),
    )
}
