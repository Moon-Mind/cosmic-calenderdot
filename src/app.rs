use crate::calendar::CalendarEvent;
use crate::config::Config;
use chrono::{Datelike, Local, NaiveDate, Weekday};
use cosmic::applet::token::subscription::{TokenUpdate, activation_token_subscription};
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{time, window::Id, Limits, Subscription};
use cosmic::prelude::*;
use cosmic::widget::{self, button, column, container, grid, row, scrollable, text, Space};
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use std::time::Duration;

mod comic {
    use cosmic::iced::Color;

    // Hintergrund #121214
    pub const PAPER: Color = Color::from_rgb(0.071, 0.071, 0.078);
    // Kreis-Hintergrund heute #52CAD7
    pub const CYAN_BRIGHT: Color = Color::from_rgb(0.322, 0.792, 0.843);
    // Normale Kalendertage #56B4C2
    pub const CYAN_MUTED: Color = Color::from_rgb(0.337, 0.706, 0.761);
    // Tage außerhalb Monat #315B63
    pub const CYAN_DIM: Color = Color::from_rgb(0.192, 0.357, 0.388);
    // Ziffer im Kreis #121214
    pub const BADGE_TEXT: Color = PAPER;
    // Wochentage & Datumstext #FFFFFF
    pub const TITLE_TEXT: Color = Color::from_rgb(1.0, 1.0, 1.0);
    // Trennlinie & Einstellungen #A9A9A9
    pub const GRID_HEADER: Color = Color::from_rgb(0.663, 0.663, 0.663);
    pub const CARD_BG: Color = Color::from_rgb(0.102, 0.102, 0.110);
    pub const DIMMED_TEXT: Color = CYAN_DIM;
    pub const EVENT_BLUE: Color = CYAN_BRIGHT;
    pub const SECTION_BG: Color = Color::from_rgb(0.082, 0.082, 0.090);
}

#[derive(Default)]
pub struct AppModel {
    core: cosmic::Core,
    popup: Option<Id>,
    config: Config,
    selected_date: NaiveDate,
    date_today: NaiveDate,
    events: Vec<CalendarEvent>,
    tick_count: u32,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    TogglePopup,
    PopupClosed(Id),
    UpdateConfig(Config),
    CalendarPrev,
    CalendarNext,
    SelectDay(NaiveDate),
    Token(TokenUpdate),
    CreateEvent,
    OpenNotifications,
    ToggleView,
    OpenSettings,
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
        let today = Local::now().date_naive();
        let events = crate::calendar::fetch_events(today, today + chrono::Duration::days(30));

        let app = AppModel {
            core,
            config: cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
                .map(|context| match Config::get_entry(&context) {
                    Ok(config) => config,
                    Err((_errors, config)) => config,
                })
                .unwrap_or_default(),
            selected_date: today,
            date_today: today,
            events,
            tick_count: 0,
            ..Default::default()
        };

        (app, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let now = Local::now();

        let clock = text::body(now.format("%d.%m., %H:%M").to_string())
            .size(13);

        self.core
            .applet
            .button_from_element(clock, false)
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
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
        match message {
            Message::Tick => {
                self.tick_count += 1;
                if self.tick_count % 60 == 0 {
                    let today = Local::now().date_naive();
                    self.events = crate::calendar::fetch_events(today, today + chrono::Duration::days(30));
                }
            }
            Message::Token(update) => {
                tracing::info!("Token update: {:?}", update);
            }
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::TogglePopup => {
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
            Message::CalendarPrev => {
                let (year, month) = if self.selected_date.month() == 1 {
                    (self.selected_date.year() - 1, 12)
                } else {
                    (self.selected_date.year(), self.selected_date.month() - 1)
                };
                if let Some(date) = NaiveDate::from_ymd_opt(year, month, 1) {
                    self.selected_date = date;
                }
            }
            Message::SelectDay(date) => {
                self.selected_date = date;
            }
            Message::CreateEvent => {
                tracing::info!("Create event");
            }
            Message::OpenNotifications => {
                tracing::info!("Open notifications");
            }
            Message::ToggleView => {
                tracing::info!("Toggle view");
            }
            Message::OpenSettings => {
                tracing::info!("Open settings");
            }
            Message::CalendarNext => {
                let (year, month) = if self.selected_date.month() == 12 {
                    (self.selected_date.year() + 1, 1)
                } else {
                    (self.selected_date.year(), self.selected_date.month() + 1)
                };
                if let Some(date) = NaiveDate::from_ymd_opt(year, month, 1) {
                    self.selected_date = date;
                }
            }
        }
        Task::none()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

impl AppModel {
    fn render_popup_content(&self) -> Element<'_, Message> {
        let content = column([
            self.render_calendar_grid(),
            self.render_divider(),
            self.render_toolbar(),
            scrollable(self.render_agenda())
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
        ]).spacing(0);

        container(content)
            .width(380)
            .max_height(650)
            .class(cosmic::theme::Container::Custom(Box::new(
                |_: &cosmic::Theme| container::Style {
                    background: Some(cosmic::iced::Background::Color(comic::PAPER)),
                    ..Default::default()
                },
            )))
            .into()
    }

    fn render_calendar_grid(&self) -> Element<'_, Message> {
        let mut cal = grid().width(Length::Fill);

        let year = self.selected_date.year();
        let month = self.selected_date.month();
        let first_day = crate::calendar::get_calendar_first(year, month, Weekday::Sun);

        for i in 0..7 {
            let name = match first_day.checked_add_signed(chrono::Duration::days(i)).unwrap().weekday() {
                Weekday::Sun => "S",
                Weekday::Mon => "M",
                Weekday::Tue => "T",
                Weekday::Wed => "W",
                Weekday::Thu => "T",
                Weekday::Fri => "F",
                Weekday::Sat => "S",
            };
            cal = cal.push(
                container(text::caption(name).class(cosmic::theme::Text::Color(comic::GRID_HEADER)))
                    .center_x(Length::Fill)
                    .padding([2, 0])
            );
        }
        cal = cal.insert_row();

        for i in 0..42 {
            if i > 0 && i % 7 == 0 {
                cal = cal.insert_row();
            }

            let date = first_day.checked_add_signed(chrono::Duration::days(i)).unwrap();
            let is_month = date.month() == month;
            let is_selected = date == self.selected_date;
            let is_today = date == self.date_today;

            let (bg, text_color, day_style) = if is_today {
                (Some(comic::CYAN_BRIGHT), comic::BADGE_TEXT, true)
            } else if is_selected {
                (Some(comic::CYAN_BRIGHT), comic::BADGE_TEXT, true)
            } else if is_month {
                (None, comic::CYAN_MUTED, true)
            } else {
                (None, comic::DIMMED_TEXT, false)
            };

            let content = column([
                text::body(format!("{}", date.day()))
                    .size(14)
                    .class(cosmic::theme::Text::Color(text_color))
                    .into(),
            ])
            .align_x(Alignment::Center)
            .spacing(1)
            .width(Length::Fill);

            let btn = button::custom(content)
                .width(Length::Fixed(40.0))
                .height(Length::Fixed(38.0))
                .class(cosmic::theme::Button::Custom {
                    active: Box::new(move |_: bool, _: &cosmic::Theme| {
                        cosmic::widget::button::Style {
                            background: bg.map(cosmic::iced::Background::Color),
                            text_color: Some(text_color),
                            border_radius: [if bg.is_some() { 20.0 } else { 6.0 }; 4].into(),
                            border_width: 0.0,
                            border_color: cosmic::iced::Color::TRANSPARENT,
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
                                    Some(cosmic::iced::Background::Color(
                                        comic::CARD_BG,
                                    ))
                                }),
                            text_color: Some(text_color),
                            border_radius: [if bg.is_some() { 20.0 } else { 6.0 }; 4].into(),
                            border_width: 0.0,
                            border_color: cosmic::iced::Color::TRANSPARENT,
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
                                        comic::PAPER,
                                    ))
                                }),
                            text_color: Some(text_color),
                            border_radius: [if bg.is_some() { 20.0 } else { 6.0 }; 4].into(),
                            border_width: 0.0,
                            border_color: cosmic::iced::Color::TRANSPARENT,
                            ..Default::default()
                        }
                    }),
                });

            if day_style {
                cal = cal.push(btn.on_press(Message::SelectDay(date)));
            } else {
                cal = cal.push(btn);
            }
        }

        let title = format!(
            "{} {}",
            NaiveDate::from_ymd_opt(year, month, 1)
                .unwrap()
                .format("%B"),
            year
        );

        let title_row = row([
            cosmic::widget::Space::new().width(Length::Fill).into(),
            text::body(title)
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
        ])
        .align_y(Alignment::Center);

        container(
            column([title_row.into(), cal.into()])
                .align_x(Alignment::Center)
                .spacing(6),
        )
        .padding([4, 14])
        .width(Length::Fill)
        .into()
    }

    fn render_divider(&self) -> Element<'_, Message> {
        container(Space::new().height(Length::Fixed(1.0)))
            .width(Length::Fill)
            .class(cosmic::theme::Container::Custom(Box::new(
                |_: &cosmic::Theme| container::Style {
                    background: Some(cosmic::iced::Background::Color(
                        comic::GRID_HEADER,
                    )),
                    ..Default::default()
                },
            )))
            .into()
    }

    fn render_toolbar(&self) -> Element<'_, Message> {
        let buttons = [
            ("+", Message::CreateEvent),
            ("\u{1f514}", Message::OpenNotifications),
            ("\u{1f4cb}", Message::ToggleView),
            ("\u{2699}", Message::OpenSettings),
        ];

        let items: Vec<Element<'_, Message>> = buttons
            .iter()
            .map(|(icon, msg)| {
                widget::button::custom(text::body(*icon).size(16))
                    .on_press(msg.clone())
                    .class(cosmic::theme::Button::Text)
                    .width(Length::Fixed(36.0))
                    .height(Length::Fixed(36.0))
                    .into()
            })
            .collect();

        container(row(items).spacing(4).align_y(Alignment::Center))
            .padding([6, 14])
            .width(Length::Fill)
            .into()
    }

    fn render_agenda(&self) -> Element<'_, Message> {
        let mut day_groups: Vec<(NaiveDate, Vec<&CalendarEvent>)> = Vec::new();
        for event in &self.events {
            let date = event.start_time.map(|t| t.date()).unwrap_or(self.date_today);
            if let Some(group) = day_groups.iter_mut().find(|(d, _)| *d == date) {
                group.1.push(event);
            } else {
                day_groups.push((date, vec![event]));
            }
        }
        day_groups.sort_by_key(|(date, _)| *date);

        let mut widgets: Vec<Element<'_, Message>> = Vec::new();

        for (i, (date, events)) in day_groups.iter().enumerate() {
            if i > 0 {
                widgets.push(Space::new().height(Length::Fixed(8.0)).into());
            }

            let date_label = if *date == self.date_today {
                format!("Today \u{2022} {}", date.format("%m/%d/%y"))
            } else {
                format!("{} \u{2022} {}", date.format("%A"), date.format("%m/%d/%y"))
            };

            widgets.push(
                container(
                    text::caption(date_label)
                        .class(cosmic::theme::Text::Color(comic::GRID_HEADER)),
                )
                .width(Length::Fill)
                .padding([6, 14])
                .class(cosmic::theme::Container::Custom(Box::new(
                    |_: &cosmic::Theme| container::Style {
                        background: Some(cosmic::iced::Background::Color(comic::SECTION_BG)),
                        ..Default::default()
                    },
                )))
                .into(),
            );

            for event in events {
                widgets.push(self.render_agenda_event(event).into());
            }
        }

        column(widgets).spacing(0).into()
    }

    fn render_agenda_event<'a>(&self, event: &'a CalendarEvent) -> Element<'a, Message> {
        let dot = container(Space::new())
            .width(Length::Fixed(8.0))
            .height(Length::Fixed(8.0))
            .class(cosmic::theme::Container::Custom(Box::new(
                |_: &cosmic::Theme| container::Style {
                    background: Some(cosmic::iced::Background::Color(comic::EVENT_BLUE)),
                    border: cosmic::iced::Border {
                        radius: 4.0.into(),
                        width: 0.0,
                        color: cosmic::iced::Color::TRANSPARENT,
                    },
                    ..Default::default()
                },
            )));

        let time_str = if event.all_day {
            String::new()
        } else {
            match (event.start_time, event.end_time) {
                (Some(s), Some(e)) => {
                    format!("{} - {}", s.format("%-I:%M %p"), e.format("%-I:%M %p"))
                }
                (Some(s), None) => {
                    format!("{}", s.format("%-I:%M %p"))
                }
                _ => String::new(),
            }
        };

        let info = column(vec![
            text::body(&event.summary)
                .size(13)
                .class(cosmic::theme::Text::Color(comic::TITLE_TEXT))
                .into(),
            if !time_str.is_empty() {
                text::caption(time_str)
                    .size(11)
                    .class(cosmic::theme::Text::Color(comic::GRID_HEADER))
                    .into()
            } else {
                Space::new().height(Length::Fixed(0.0)).into()
            },
        ])
        .spacing(1);

        container(
            row([
                dot.into(),
                Space::new().width(Length::Fixed(10.0)).into(),
                info.into(),
                Space::new().width(Length::Fill).into(),
            ])
            .align_y(Alignment::Center),
        )
        .padding([6, 14])
        .width(Length::Fill)
        .into()
    }
}
