use crate::calendar::{CalendarDay, CalendarEvent, MonthCalendar};
use crate::config::Config;
use chrono::{Datelike, Local, Timelike};
use chrono_tz::{America::New_York, Asia::Kolkata, Europe::London};
use cosmic::applet::token::subscription::{TokenUpdate, activation_token_subscription};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{time, window::Id, Limits, Subscription};
use cosmic::prelude::*;
use cosmic::widget::{
    self, column, container, row, scrollable, text, Grid, Space,
};
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use std::time::Duration;

mod comic {
    use cosmic::iced::Color;

    pub const PAPER: Color = Color::from_rgb(0.96, 0.94, 0.88);
    pub const OUTLINE: Color = Color::from_rgb(0.06, 0.06, 0.06);
    pub const ACCENT_YELLOW: Color = Color::from_rgb(1.0, 0.93, 0.27);
    pub const ACCENT_RED: Color = Color::from_rgb(0.91, 0.16, 0.13);
    pub const ACCENT_BLUE: Color = Color::from_rgb(0.06, 0.25, 0.84);
    pub const BADGE_TEXT: Color = Color::from_rgb(1.0, 1.0, 1.0);
    pub const HEADER_BG: Color = ACCENT_YELLOW;
    pub const HEADER_TEXT: Color = Color::from_rgb(0.06, 0.06, 0.06);
    pub const CARD_BG: Color = Color::from_rgb(1.0, 1.0, 1.0);
    pub const TITLE_TEXT: Color = OUTLINE;
    pub const EMPTY_TEXT: Color = Color::from_rgb(0.5, 0.5, 0.5);
    pub const SECTION_TEXT: Color = Color::from_rgb(0.4, 0.4, 0.4);
    pub const GRID_HEADER: Color = Color::from_rgb(0.5, 0.5, 0.5);
    pub const TIME_COLOR: Color = ACCENT_RED;
}

#[derive(Default)]
pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config: Config,
    events: Vec<CalendarEvent>,
    tomorrow_events: Vec<CalendarEvent>,
    month_calendar: MonthCalendar,
    greeting: String,
    user_name: String,
    focus_time: i64,
    world_clocks: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    TogglePopup,
    PopupClosed(Id),
    UpdateConfig(Config),
    Refresh,
    Token(TokenUpdate),
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "com.cosmic.calenderdot";

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
        tracing::info!("AppModel::init called");

        let now = Local::now();
        let greeting = get_greeting(now.hour()).to_string();
        let user_name = std::env::var("USER").unwrap_or_else(|_| "there".into());

        let events = crate::calendar::fetch_today_events();
        let tomorrow_events = crate::calendar::fetch_tomorrow_events();
        let month_calendar = crate::calendar::get_month_calendar();
        let focus_time = crate::calendar::focus_time_minutes(&events);
        let world_clocks = format_world_times();

        let app = AppModel {
            core,
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => config,
                })
                .unwrap_or_default(),
            events,
            tomorrow_events,
            month_calendar,
            greeting,
            user_name,
            focus_time,
            world_clocks,
            ..Default::default()
        };

        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        tracing::trace!("AppModel::view called");
        let now = Local::now();
        let time_str = now.format("%H:%M").to_string();
        let day_str = now.format("%d").to_string();

        let clock = column([
            text::body(time_str)
                .size(14)
                .class(cosmic::theme::Text::Color(comic::BADGE_TEXT))
                .into(),
            text::caption(day_str)
                .size(8)
                .class(cosmic::theme::Text::Color(comic::BADGE_TEXT))
                .into(),
        ])
        .align_x(Alignment::Center)
        .spacing(0);

        self.core
            .applet
            .button_from_element(clock, false)
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, id: Id) -> Element<'_, Self::Message> {
        tracing::info!("AppModel::view_window called with id: {:?}", id);
        let content = self.render_popup_content();
        self.core.applet.popup_container(content).into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            activation_token_subscription(0).map(Message::Token),
            time::every(Duration::from_secs(60))
                .map(|_| Message::Tick),
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
        ])
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        tracing::info!("AppModel::update called with: {:?}", message);
        match message {
            Message::Tick => {}
            Message::Token(update) => {
                tracing::info!("Token update: {:?}", update);
            }
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::TogglePopup => {
                tracing::info!("TogglePopup");
                return if let Some(p) = self.popup.take() {
                    destroy_popup(p)
                } else {
                    let new_id = Id::unique();
                    self.popup.replace(new_id);
                    let mut popup_settings = self.core.applet.get_popup_settings(
                        self.core.main_window_id().unwrap(),
                        new_id,
                        None,
                        None,
                        None,
                    );
                    popup_settings.positioner.size_limits = Limits::NONE
                        .max_width(420.0)
                        .min_width(320.0)
                        .min_height(300.0)
                        .max_height(650.0);
                    get_popup(popup_settings)
                };
            }
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::Refresh => {
                tracing::info!("Refresh");
                let now = Local::now();
                self.greeting = get_greeting(now.hour()).to_string();
                self.events = crate::calendar::fetch_today_events();
                self.tomorrow_events = crate::calendar::fetch_tomorrow_events();
                self.month_calendar = crate::calendar::get_month_calendar();
                self.focus_time = crate::calendar::focus_time_minutes(&self.events);
                self.world_clocks = format_world_times();
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

fn get_greeting(hour: u32) -> &'static str {
    match hour {
        0..=11 => "Good morning",
        12..=16 => "Good afternoon",
        _ => "Good evening",
    }
}

fn format_world_times() -> Vec<(String, String)> {
    let utc = chrono::Utc::now().naive_utc();
    let nyc_time = utc.and_local_timezone(New_York).unwrap();
    let bho_time = utc.and_local_timezone(Kolkata).unwrap();
    let ldn_time = utc.and_local_timezone(London).unwrap();

    vec![
        ("NYC".into(), nyc_time.format("%H:%M").to_string()),
        ("BHO".into(), bho_time.format("%H:%M").to_string()),
        ("LDN".into(), ldn_time.format("%H:%M").to_string()),
    ]
}

impl AppModel {
    fn render_popup_content(&self) -> Element<'_, Message> {
        let sections: Vec<Element<'_, Message>> = vec![
            self.render_header(),
            scrollable(
                column([
                    self.render_greeting_summary(),
                    self.render_calendar_grid(),
                    self.render_tomorrow_section(),
                    self.render_world_clock(),
                ])
                .spacing(0),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
            self.render_refresh_button(),
        ];

        let content = column(sections).spacing(0);

        container(content)
            .width(380)
            .max_height(650)
            .class(cosmic::theme::Container::Custom(Box::new(
                |_: &cosmic::Theme| container::Style {
                    background: Some(cosmic::iced::Background::Color(comic::PAPER)),
                    border: cosmic::iced::Border {
                        radius: [0.0, 0.0, 8.0, 8.0].into(),
                        width: 2.0,
                        color: comic::OUTLINE,
                    },
                    ..Default::default()
                },
            )))
            .into()
    }

    fn render_header(&self) -> Element<'_, Message> {
        let now = Local::now();
        let header_text = format!(
            "  {} {}  ",
            now.format("%B %d").to_string(),
            now.year()
        );

        container(
            text::title4(header_text)
                .class(cosmic::theme::Text::Color(comic::HEADER_TEXT)),
        )
        .width(Length::Fill)
        .padding([10, 14])
        .class(cosmic::theme::Container::Custom(Box::new(
            |_: &cosmic::Theme| container::Style {
                background: Some(cosmic::iced::Background::Color(comic::HEADER_BG)),
                border: cosmic::iced::Border {
                    radius: [4.0, 4.0, 0.0, 0.0].into(),
                    width: 2.0,
                    color: comic::OUTLINE,
                },
                text_color: Some(comic::HEADER_TEXT),
                ..Default::default()
            },
        )))
        .into()
    }

    fn render_greeting_summary(&self) -> Element<'_, Message> {
        let greeting_text = text::body(format!("{}, {}.", self.greeting, self.user_name))
            .size(15)
            .class(cosmic::theme::Text::Color(comic::TITLE_TEXT));

        let summary_text = if self.events.is_empty() {
            text::caption("No events today.")
                .class(cosmic::theme::Text::Color(comic::EMPTY_TEXT))
        } else {
            let hours = self.focus_time / 60;
            let mins = self.focus_time % 60;
            let time_str = if hours > 0 {
                format!("{}h {}m focus time", hours, mins)
            } else {
                format!("{}m focus time", mins)
            };
            text::caption(format!(
                "{} event{} · {}",
                self.events.len(),
                if self.events.len() == 1 { "" } else { "s" },
                time_str,
            ))
            .class(cosmic::theme::Text::Color(comic::SECTION_TEXT))
        };

        container(
            column([greeting_text.into(), summary_text.into()]).spacing(2),
        )
        .padding([10, 14])
        .width(Length::Fill)
        .into()
    }

    fn render_calendar_grid(&self) -> Element<'_, Message> {
        let weekday_names = ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"];
        let mut grid = Grid::new()
            .column_alignment(Alignment::Center)
            .column_spacing(4)
            .row_spacing(2)
            .width(Length::Fill);

        for name in weekday_names.iter() {
            grid = grid.push(
                text::caption(*name)
                    .size(10)
                    .class(cosmic::theme::Text::Color(comic::GRID_HEADER)),
            );
        }
        grid = grid.insert_row();

        for day in &self.month_calendar.days {
            grid = grid.push(self.render_day_cell(day));
        }

        container(
            column([
                text::body(&self.month_calendar.title)
                    .class(cosmic::theme::Text::Color(comic::TITLE_TEXT))
                    .into(),
                grid.into(),
            ])
            .align_x(Alignment::Center)
            .spacing(6),
        )
        .padding([4, 14])
        .width(Length::Fill)
        .into()
    }

    fn render_day_cell(&self, day: &CalendarDay) -> Element<'_, Message> {
        if day.day == 0 {
            return Space::new()
                .width(Length::Fill)
                .height(Length::Fixed(28.0))
                .into();
        }

        let cell = if day.is_today {
            container(
                text::body(day.day.to_string())
                    .size(14)
                    .class(cosmic::theme::Text::Color(comic::BADGE_TEXT)),
            )
            .padding([3, 6])
            .class(cosmic::theme::Container::Custom(Box::new(
                |_: &cosmic::Theme| container::Style {
                    background: Some(cosmic::iced::Background::Color(comic::ACCENT_RED)),
                    border: cosmic::iced::Border {
                        radius: 12.0.into(),
                        width: 1.5,
                        color: comic::OUTLINE,
                    },
                    ..Default::default()
                },
            )))
        } else {
            container(
                text::body(day.day.to_string())
                    .size(14)
                    .class(cosmic::theme::Text::Color(comic::TITLE_TEXT)),
            )
            .padding([3, 6])
        };

        let dot = if day.has_event {
            container(
                text::body("\u{25cf}")
                    .size(6)
                    .class(cosmic::theme::Text::Color(comic::ACCENT_BLUE)),
            )
            .into()
        } else {
            Space::new().height(Length::Fixed(2.0)).into()
        };

        column([cell.into(), dot])
            .align_x(Alignment::Center)
            .spacing(1)
            .width(Length::Fill)
            .into()
    }

    fn render_tomorrow_section(&self) -> Element<'_, Message> {
        let mut widgets: Vec<Element<'_, Message>> = vec![
            text::caption("TOMORROW")
                .size(11)
                .class(cosmic::theme::Text::Color(comic::SECTION_TEXT))
                .into(),
            Space::new().height(Length::Fixed(6.0)).into(),
        ];

        if self.tomorrow_events.is_empty() {
            widgets.push(
                text::caption("Clear skies ahead.")
                    .class(cosmic::theme::Text::Color(comic::EMPTY_TEXT))
                    .into(),
            );
        } else {
            for event in &self.tomorrow_events {
                let time_str = match event.start_time {
                    Some(dt) => {
                        let h = dt.hour12().1;
                        let m = dt.minute();
                        let ampm = if dt.hour() < 12 { "AM" } else { "PM" };
                        format!("{}:{:02} {}", h, m, ampm)
                    }
                    None => "All Day".into(),
                };

                let time_label = container(
                    text::body(time_str)
                        .size(12)
                        .class(cosmic::theme::Text::Color(comic::TIME_COLOR)),
                )
                .padding([2, 6])
                .class(cosmic::theme::Container::Custom(Box::new(
                    |_: &cosmic::Theme| container::Style {
                        background: Some(cosmic::iced::Background::Color(comic::ACCENT_YELLOW)),
                        border: cosmic::iced::Border {
                            radius: 4.0.into(),
                            width: 1.5,
                            color: comic::OUTLINE,
                        },
                        ..Default::default()
                    },
                )));

                let event_row = row([
                    time_label.into(),
                    Space::new().width(Length::Fixed(8.0)).into(),
                    text::body(&event.summary)
                        .size(13)
                        .class(cosmic::theme::Text::Color(comic::TITLE_TEXT))
                        .into(),
                ])
                .align_y(Alignment::Center);

                widgets.push(container(event_row).padding([2, 0]).into());
            }
        }

        container(column(widgets).spacing(4))
            .padding([10, 14])
            .width(Length::Fill)
            .into()
    }

    fn render_world_clock(&self) -> Element<'_, Message> {
        let clock_widgets: Vec<Element<'_, Message>> = self
            .world_clocks
            .iter()
            .map(|(city, time)| {
                container(
                    column([
                        text::caption(city.as_str())
                            .size(10)
                            .class(cosmic::theme::Text::Color(comic::SECTION_TEXT))
                            .into(),
                        text::body(time)
                            .size(16)
                            .class(cosmic::theme::Text::Color(comic::TITLE_TEXT))
                            .into(),
                    ])
                    .align_x(Alignment::Center)
                    .spacing(1),
                )
                .padding([6, 8])
                .width(Length::Fill)
                .class(cosmic::theme::Container::Custom(Box::new(
                    |_: &cosmic::Theme| container::Style {
                        background: Some(cosmic::iced::Background::Color(comic::CARD_BG)),
                        border: cosmic::iced::Border {
                            radius: 6.0.into(),
                            width: 1.5,
                            color: comic::OUTLINE,
                        },
                        ..Default::default()
                    },
                )))
                .into()
            })
            .collect();

        container(row(clock_widgets).spacing(6))
            .padding([4, 14])
            .padding([10, 14, 4, 14])
            .width(Length::Fill)
            .into()
    }

    fn render_refresh_button(&self) -> Element<'_, Message> {
        container(
            widget::button::custom(
                text::body("  \u{21bb} Refresh  ")
                    .class(cosmic::theme::Text::Color(comic::ACCENT_BLUE)),
            )
            .on_press(Message::Refresh)
            .class(cosmic::theme::Button::Text),
        )
        .width(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Right)
        .padding([4, 14])
        .into()
    }
}
