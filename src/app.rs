use crate::calendar::{CalendarEvent, MonthCalendar};
use crate::config::Config;
use chrono::{Datelike, Local, NaiveDate, Timelike};
use chrono_tz::{America::New_York, Asia::Kolkata, Europe::London};
use cosmic::applet::token::subscription::{TokenUpdate, activation_token_subscription};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{time, window::Id, Limits, Subscription};
use cosmic::prelude::*;
use cosmic::widget::{
    self, column, container, row, scrollable, text, Space,
};
use cosmic::Element;
use cosmic::iced::{Alignment, Color, Length};
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
    pub const PROGRESS_FILL: Color = Color::from_rgb(0.2, 0.7, 0.2);
    pub const PROGRESS_BG: Color = Color::from_rgb(0.85, 0.85, 0.82);
    pub const TAG_ORANGE: Color = Color::from_rgb(0.95, 0.55, 0.05);
    pub const NOW_GREEN: Color = Color::from_rgb(0.15, 0.6, 0.15);
    pub const TODAY_GREEN: Color = Color::from_rgb(0.2, 0.7, 0.2);
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
    selected_date: NaiveDate,
    focus_time: i64,
    world_clocks: Vec<(String, String)>,
    calendar_year: i32,
    calendar_month: u32,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    TogglePopup,
    PopupClosed(Id),
    UpdateConfig(Config),
    Refresh,
    CalendarPrev,
    CalendarNext,
    SelectDay(NaiveDate),
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

        let selected_date = now.date_naive();
        let events = crate::calendar::fetch_today_events();
        let tomorrow_events = crate::calendar::fetch_tomorrow_events();
        let calendar_year = now.year();
        let calendar_month = now.month();
        let month_calendar =
            crate::calendar::get_month_calendar_for(calendar_year, calendar_month);
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
            selected_date,
            events,
            tomorrow_events,
            month_calendar,
            greeting,
            user_name,
            focus_time,
            world_clocks,
            calendar_year,
            calendar_month,
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
                self.selected_date = now.date_naive();
                self.greeting = get_greeting(now.hour()).to_string();
                self.events = crate::calendar::fetch_today_events();
                self.tomorrow_events = crate::calendar::fetch_tomorrow_events();
                self.calendar_year = now.year();
                self.calendar_month = now.month();
                self.month_calendar = crate::calendar::get_month_calendar_for(
                    self.calendar_year,
                    self.calendar_month,
                );
                self.focus_time = crate::calendar::focus_time_minutes(&self.events);
                self.world_clocks = format_world_times();
            }
            Message::CalendarPrev => {
                if self.calendar_month == 1 {
                    self.calendar_year -= 1;
                    self.calendar_month = 12;
                } else {
                    self.calendar_month -= 1;
                }
                self.month_calendar = crate::calendar::get_month_calendar_for(
                    self.calendar_year,
                    self.calendar_month,
                );
            }
            Message::SelectDay(date) => {
                self.selected_date = date;
                self.calendar_year = date.year();
                self.calendar_month = date.month();
                self.month_calendar = crate::calendar::get_month_calendar_for(
                    self.calendar_year,
                    self.calendar_month,
                );
            }
            Message::CalendarNext => {
                if self.calendar_month == 12 {
                    self.calendar_year += 1;
                    self.calendar_month = 1;
                } else {
                    self.calendar_month += 1;
                }
                self.month_calendar = crate::calendar::get_month_calendar_for(
                    self.calendar_year,
                    self.calendar_month,
                );
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
                    self.render_progress_bar(),
                    self.render_greeting_summary(),
                    self.render_today_events(),
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

    fn render_progress_bar(&self) -> Element<'_, Message> {
        let now = Local::now();
        let year = now.year();
        let start = chrono::NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
        let end = chrono::NaiveDate::from_ymd_opt(year, 12, 31).unwrap();
        let total = (end - start).num_days() + 1;
        let done = now.ordinal() as i64;
        let left = total - done;
        let pct = (done as f64 / total as f64) * 100.0;

        let bar_fill = container(cosmic::widget::Space::new())
            .width(Length::Fixed(280.0 * (pct / 100.0) as f32))
            .height(Length::Fixed(8.0))
            .class(cosmic::theme::Container::Custom(Box::new(
                |_: &cosmic::Theme| container::Style {
                    background: Some(cosmic::iced::Background::Color(
                        comic::PROGRESS_FILL,
                    )),
                    border: cosmic::iced::Border {
                        radius: 4.0.into(),
                        width: 0.0,
                        color: cosmic::iced::Color::TRANSPARENT,
                    },
                    ..Default::default()
                },
            )));

        let bar_track = container(bar_fill)
            .width(Length::Fixed(280.0))
            .height(Length::Fixed(8.0))
            .class(cosmic::theme::Container::Custom(Box::new(
                |_: &cosmic::Theme| container::Style {
                    background: Some(cosmic::iced::Background::Color(
                        comic::PROGRESS_BG,
                    )),
                    border: cosmic::iced::Border {
                        radius: 4.0.into(),
                        width: 1.5,
                        color: comic::OUTLINE,
                    },
                    ..Default::default()
                },
            )));

        container(
            column([
                bar_track.into(),
                container(
                    row([
                        text::caption(format!("{:.1}% of {}", pct, year))
                            .class(cosmic::theme::Text::Color(comic::SECTION_TEXT))
                            .into(),
                        cosmic::widget::Space::new().width(Length::Fill).into(),
                        text::caption(format!("{} days left", left))
                            .class(cosmic::theme::Text::Color(comic::TIME_COLOR))
                            .into(),
                    ])
                    .align_y(Alignment::Center),
                )
                .padding([2, 0])
                .width(Length::Fill)
                .into(),
            ])
            .spacing(2),
        )
        .padding([8, 14])
        .width(Length::Fill)
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

    fn render_today_events(&self) -> Element<'_, Message> {
        let mut widgets: Vec<Element<'_, Message>> = vec![
            render_today_header(&self.events),
            Space::new().height(Length::Fixed(6.0)).into(),
        ];

        if self.events.is_empty() {
            widgets.push(
                text::caption("No events today.")
                    .class(cosmic::theme::Text::Color(comic::EMPTY_TEXT))
                    .into(),
            );
        } else {
            let now = Local::now();

            let naive_now = now.naive_local();

            for event in &self.events {
                let is_active = event.start_time.map_or(false, |s| {
                    let e = event.end_time.unwrap_or(s);
                    s <= naive_now && naive_now <= e
                });
                let is_soon = event.start_time.map_or(false, |s| {
                    let diff = (s - naive_now).num_minutes();
                    diff > 0 && diff <= 60
                });

                if is_active {
                    widgets.push(self.render_active_event_card(event).into());
                } else {
                    let mins = event.start_time.map_or(0, |s| {
                        (s - naive_now).num_minutes().max(0) as u32
                    });
                    widgets.push(
                        self.render_event_row(event, is_soon, mins)
                            .into(),
                    );
                }
            }
        }

        container(column(widgets).spacing(4))
            .padding([4, 14])
            .width(Length::Fill)
            .into()
    }

    fn render_active_event_card(&self, event: &CalendarEvent) -> Element<'_, Message> {
        let time_str = format_event_time(event);

        let green_bar = container(
            cosmic::widget::Space::new().width(Length::Fixed(4.0)).height(Length::Fill),
        )
        .height(Length::Fixed(48.0))
        .class(cosmic::theme::Container::Custom(Box::new(
            |_: &cosmic::Theme| container::Style {
                background: Some(cosmic::iced::Background::Color(
                    comic::NOW_GREEN,
                )),
                border: cosmic::iced::Border {
                    radius: 2.0.into(),
                    width: 0.0,
                    color: cosmic::iced::Color::TRANSPARENT,
                },
                ..Default::default()
            },
        )));

        let now_badge = container(
            text::caption("NOW")
                .size(10)
                .class(cosmic::theme::Text::Color(comic::BADGE_TEXT)),
        )
        .padding([2, 8])
        .class(cosmic::theme::Container::Custom(Box::new(
            |_: &cosmic::Theme| container::Style {
                background: Some(cosmic::iced::Background::Color(
                    comic::NOW_GREEN,
                )),
                border: cosmic::iced::Border {
                    radius: 10.0.into(),
                    width: 1.0,
                    color: comic::OUTLINE,
                },
                ..Default::default()
            },
        )));

        let top_row = row([
            green_bar.into(),
            Space::new().width(Length::Fixed(6.0)).into(),
            text::body(format!("{} • {}", time_str, event.summary))
                .size(13)
                .class(cosmic::theme::Text::Color(comic::TITLE_TEXT))
                .into(),
            cosmic::widget::Space::new().width(Length::Fill).into(),
            now_badge.into(),
        ])
        .align_y(Alignment::Center);

        let icons = self.render_event_actions(event);

        container(
            column([
                top_row.into(),
                container(icons)
                    .padding([4, 0, 0, 10])
                    .into(),
            ])
            .spacing(0),
        )
        .padding([8, 10])
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
    }

    fn render_event_row(
        &self,
        event: &CalendarEvent,
        is_soon: bool,
        mins_until: u32,
    ) -> Element<'_, Message> {
        let time_str = format_event_time(event);

        let middle = row([
            text::body(format!("{} • {}", time_str, event.summary))
                .size(13)
                .class(cosmic::theme::Text::Color(comic::TITLE_TEXT))
                .into(),
            cosmic::widget::Space::new().width(Length::Fill).into(),
            if is_soon && mins_until > 0 {
                container(
                    text::caption(format!("in {}m", mins_until))
                        .size(10)
                        .class(cosmic::theme::Text::Color(comic::BADGE_TEXT)),
                )
                .padding([2, 8])
                .class(cosmic::theme::Container::Custom(Box::new(
                    |_: &cosmic::Theme| container::Style {
                        background: Some(cosmic::iced::Background::Color(
                            comic::TAG_ORANGE,
                        )),
                        border: cosmic::iced::Border {
                            radius: 10.0.into(),
                            width: 1.0,
                            color: comic::OUTLINE,
                        },
                        ..Default::default()
                    },
                )))
                .into()
            } else {
                Space::new().height(Length::Fixed(0.0)).into()
            },
        ])
        .align_y(Alignment::Center);

        container(middle)
            .padding([4, 0])
            .width(Length::Fill)
            .into()
    }

    fn render_event_actions(&self, event: &CalendarEvent) -> Element<'_, Message> {
        let mut icons: Vec<Element<'_, Message>> = Vec::new();

        let all_text = event_action_text(event);

        if all_text.contains("zoom")
            || all_text.contains("meet")
            || all_text.contains("video")
        {
            icons.push(action_icon("\u{1f3ac}", comic::ACCENT_BLUE));
        }
        if all_text.contains("figma")
            || all_text.contains("design")
            || all_text.contains("mockup")
        {
            icons.push(action_icon("\u{1f3a8}", comic::ACCENT_RED));
        }
        if all_text.contains("docs") || all_text.contains("doc") || all_text.contains("sheet") {
            icons.push(action_icon("\u{1f4c4}", comic::ACCENT_BLUE));
        }
        if all_text.contains("review") || all_text.contains("slides") || all_text.contains("web") {
            icons.push(action_icon("\u{1f310}", comic::PROGRESS_FILL));
        }

        row(icons).spacing(4).align_y(Alignment::Center).into()
    }

    fn render_calendar_grid(&self) -> Element<'_, Message> {
        let weekday_names = ["S", "M", "T", "W", "T", "F", "S"];

        let header_row = row(
            weekday_names.iter().map(|name| {
                container(
                    text::caption(*name)
                        .size(10)
                        .class(cosmic::theme::Text::Color(comic::GRID_HEADER)),
                )
                .width(Length::Fill)
                .center_x(Length::Fill)
                .padding([2, 0])
                .into()
            })
            .collect::<Vec<Element<'_, Message>>>(),
        )
        .spacing(2)
        .width(Length::Fill);

        let year = self.calendar_year;
        let month = self.calendar_month;

        let mut week_rows: Vec<Element<'_, Message>> = Vec::new();
        let mut current: Vec<Element<'_, Message>> = Vec::new();

        for cal_day in &self.month_calendar.days {
            let cell: Element<'_, Message> = if cal_day.day == 0 {
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fixed(34.0))
                    .into()
            } else {
                let date = NaiveDate::from_ymd_opt(year, month, cal_day.day).unwrap();
                let is_selected = date == self.selected_date;

                let day_text = text::body(cal_day.day.to_string()).size(14);

                let (bg, text) = if cal_day.is_today {
                    (Some(comic::PROGRESS_FILL), comic::BADGE_TEXT)
                } else if is_selected {
                    (Some(comic::ACCENT_YELLOW), comic::TITLE_TEXT)
                } else {
                    (None, comic::TITLE_TEXT)
                };

                let btn = widget::button::custom(day_text)
                    .width(Length::Fill)
                    .height(Length::Fixed(34.0))
                    .class(cosmic::theme::Button::Custom {
                        active: Box::new(move |_: bool, _: &cosmic::Theme| {
                            cosmic::widget::button::Style {
                                background: bg.map(cosmic::iced::Background::Color),
                                text_color: Some(text),
                                border_radius: [4.0; 4].into(),
                                border_width: 1.5,
                                border_color: comic::OUTLINE,
                                ..Default::default()
                            }
                        }),
                        disabled: Box::new(|_: &cosmic::Theme| {
                            cosmic::widget::button::Style::default()
                        }),
                        hovered: Box::new(move |_: bool, _: &cosmic::Theme| {
                            cosmic::widget::button::Style {
                                background: bg
                                    .map(|c| {
                                        let r = (c.r * 1.15).min(1.0);
                                        let g = (c.g * 1.15).min(1.0);
                                        let b = (c.b * 1.15).min(1.0);
                                        cosmic::iced::Background::Color(
                                            cosmic::iced::Color::from_rgb(r, g, b),
                                        )
                                    })
                                    .or_else(|| {
                                        Some(cosmic::iced::Background::Color(comic::CARD_BG))
                                    }),
                                text_color: Some(text),
                                border_radius: [4.0; 4].into(),
                                border_width: 1.5,
                                border_color: comic::OUTLINE,
                                ..Default::default()
                            }
                        }),
                        pressed: Box::new(move |_: bool, _: &cosmic::Theme| {
                            cosmic::widget::button::Style {
                                background: bg
                                    .map(|c| {
                                        let r = (c.r * 0.85).max(0.0);
                                        let g = (c.g * 0.85).max(0.0);
                                        let b = (c.b * 0.85).max(0.0);
                                        cosmic::iced::Background::Color(
                                            cosmic::iced::Color::from_rgb(r, g, b),
                                        )
                                    })
                                    .or_else(|| {
                                        Some(cosmic::iced::Background::Color(
                                            comic::CARD_BG,
                                        ))
                                    }),
                                text_color: Some(text),
                                border_radius: [4.0; 4].into(),
                                border_width: 1.5,
                                border_color: comic::OUTLINE,
                                ..Default::default()
                            }
                        }),
                    })
                    .on_press(Message::SelectDay(date));

                let mut col = column([btn.into()])
                    .align_x(Alignment::Center)
                    .spacing(1)
                    .width(Length::Fill);

                if cal_day.has_event {
                    col = col.push(
                        text::caption("\u{25cf}")
                            .size(6)
                            .class(cosmic::theme::Text::Color(comic::ACCENT_BLUE)),
                    );
                } else {
                    col = col.push(Space::new().height(Length::Fixed(2.0)));
                }

                col.into()
            };

            current.push(cell);
            if current.len() == 7 {
                week_rows.push(
                    row(std::mem::take(&mut current))
                        .spacing(2)
                        .width(Length::Fill)
                        .into(),
                );
            }
        }

        let title_row = row([
            widget::button::custom(text::body("  +  "))
                .class(cosmic::theme::Button::Text)
                .into(),
            cosmic::widget::Space::new().width(Length::Fill).into(),
            text::body(&self.month_calendar.title)
                .class(cosmic::theme::Text::Color(comic::TITLE_TEXT))
                .into(),
            cosmic::widget::Space::new().width(Length::Fill).into(),
            widget::button::custom(text::body("  \u{25c0}  "))
                .on_press(Message::CalendarPrev)
                .class(cosmic::theme::Button::Text)
                .into(),
            widget::button::custom(text::body("  \u{25b6}  "))
                .on_press(Message::CalendarNext)
                .class(cosmic::theme::Button::Text)
                .into(),
            widget::button::custom(text::body("  ?  "))
                .class(cosmic::theme::Button::Text)
                .into(),
        ])
        .align_y(Alignment::Center);

        let calendar = column(
            std::iter::once(header_row.into())
                .chain(week_rows.into_iter())
                .collect::<Vec<Element<'_, Message>>>(),
        )
        .spacing(4)
        .width(Length::Fill);

        container(
            column([title_row.into(), calendar.into()])
                .align_x(Alignment::Center)
                .spacing(6),
        )
        .padding([4, 14])
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
                        .size(14)
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

fn render_today_header(events: &[CalendarEvent]) -> Element<'_, Message> {
    let has_marked = events.iter().any(|e| {
        e.summary.to_lowercase().contains("launch")
    });

    row([
        text::caption("TODAY")
            .size(12)
            .class(cosmic::theme::Text::Color(comic::TODAY_GREEN))
            .into(),
        if has_marked {
            row([
                text::caption("  •  ")
                    .size(12)
                    .class(cosmic::theme::Text::Color(comic::SECTION_TEXT))
                    .into(),
                container(
                    text::caption("Launch day!")
                        .size(10)
                        .class(cosmic::theme::Text::Color(comic::BADGE_TEXT)),
                )
                .padding([1, 8])
                .class(cosmic::theme::Container::Custom(Box::new(
                    |_: &cosmic::Theme| container::Style {
                        background: Some(cosmic::iced::Background::Color(
                            comic::TAG_ORANGE,
                        )),
                        border: cosmic::iced::Border {
                            radius: 10.0.into(),
                            width: 1.0,
                            color: comic::OUTLINE,
                        },
                        ..Default::default()
                    },
                )))
                .into(),
            ])
            .spacing(0)
            .align_y(Alignment::Center)
            .into()
        } else {
            Space::new().height(Length::Fixed(0.0)).into()
        },
    ])
    .spacing(0)
    .align_y(Alignment::Center)
    .into()
}

fn format_event_time(event: &CalendarEvent) -> String {
    match event.start_time {
        Some(dt) => {
            let h = dt.hour12().1;
            let m = dt.minute();
            let ampm = if dt.hour() < 12 { "AM" } else { "PM" };
            format!("{}:{:02} {}", h, m, ampm)
        }
        None => "All Day".into(),
    }
}

fn event_action_text(event: &CalendarEvent) -> String {
    let lower_summary = event.summary.to_lowercase();
    let lower_loc = event
        .location
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    let lower_desc = event
        .description
        .as_deref()
        .unwrap_or("")
        .to_lowercase();
    format!("{} {} {}", lower_summary, lower_loc, lower_desc)
}

fn action_icon(emoji: &str, color: Color) -> Element<'_, Message> {
    container(text::body(format!("  {}  ", emoji)).size(14))
        .padding([2, 4])
        .class(cosmic::theme::Container::Custom(Box::new(
            move |_: &cosmic::Theme| container::Style {
                background: Some(cosmic::iced::Background::Color(color)),
                border: cosmic::iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: comic::OUTLINE,
                },
                ..Default::default()
            },
        )))
        .into()
}
