use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, Timelike, Weekday};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub summary: String,
    pub start_time: Option<NaiveDateTime>,
    pub end_time: Option<NaiveDateTime>,
    pub location: Option<String>,
    pub description: Option<String>,
}

pub fn is_today_event(event: &CalendarEvent, today: NaiveDate) -> bool {
    event
        .start_time
        .map(|dt| dt.date() == today)
        .unwrap_or(false)
}

pub fn fetch_today_events() -> Vec<CalendarEvent> {
    let today = Local::now().date_naive();
    fetch_events_for_date(today)
}

pub fn fetch_tomorrow_events() -> Vec<CalendarEvent> {
    let tomorrow = Local::now().date_naive() + chrono::Duration::days(1);
    fetch_events_for_date(tomorrow)
}

pub fn focus_time_minutes(events: &[CalendarEvent]) -> i64 {
    events
        .iter()
        .filter_map(|e| {
            let start = e.start_time?;
            let end = e.end_time?;
            Some((end - start).num_minutes())
        })
        .sum()
}

#[derive(Debug, Clone)]
pub struct CalendarDay {
    pub day: u32,
    pub has_event: bool,
    pub is_today: bool,
}

#[derive(Default)]
pub struct MonthCalendar {
    pub title: String,
    pub days: Vec<CalendarDay>,
}

fn fetch_events_for_date(date: NaiveDate) -> Vec<CalendarEvent> {
    let mut events = try_ics_files_for_date(date);
    if events.is_empty() {
        events = demo_events(date);
    }
    events.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    events
}

fn num_days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn generate_month_events(year: i32, month: u32) -> Vec<CalendarEvent> {
    let mut events = Vec::new();
    let days = num_days_in_month(year, month);
    for d in 1..=days {
        if d % 3 == 0 || d % 7 == 0 {
            if let Some(date) = NaiveDate::from_ymd_opt(year, month, d) {
                events.extend(demo_events(date));
            }
        }
    }
    events
}

pub fn get_month_calendar_for(year: i32, month: u32) -> MonthCalendar {
    let today = Local::now().date_naive();
    let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let first_weekday = first_day.weekday().num_days_from_sunday() as usize;
    let days_in_month = num_days_in_month(year, month);
    let month_events = generate_month_events(year, month);

    let mut days = Vec::new();
    for _ in 0..first_weekday {
        days.push(CalendarDay {
            day: 0,
            has_event: false,
            is_today: false,
        });
    }
    for d in 1..=days_in_month {
        let date = NaiveDate::from_ymd_opt(year, month, d).unwrap();
        let has_event = month_events.iter().any(|e| is_today_event(e, date));
        let is_today = date == today;
        days.push(CalendarDay {
            day: d,
            has_event,
            is_today,
        });
    }
    while days.len() % 7 != 0 {
        days.push(CalendarDay {
            day: 0,
            has_event: false,
            is_today: false,
        });
    }

    MonthCalendar {
        title: format!("{} {}", first_day.format("%B"), year),
        days,
    }
}

fn try_ics_files_for_date(date: NaiveDate) -> Vec<CalendarEvent> {
    let mut events: Vec<CalendarEvent> = Vec::new();

    let search_dirs: Vec<Option<PathBuf>> = vec![
        dirs::data_dir().map(|d| d.join("evolution").join("calendar")),
        dirs::data_dir().map(|d| d.join("calendars")),
        dirs::config_dir().map(|d| d.join("calendar")),
        dirs::data_local_dir().map(|d| d.join("calendars")),
    ];

    for dir in search_dirs.into_iter().flatten() {
        if dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "ics") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            for event in parse_ics_events(&content) {
                                if is_today_event(&event, date) {
                                    events.push(event);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    events
}

fn parse_ics_dt(dt_str: &str) -> Option<NaiveDateTime> {
    let len = dt_str.len();
    if len >= 8 {
        let year: i32 = dt_str[0..4].parse().ok()?;
        let month: u32 = dt_str[4..6].parse().ok()?;
        let day: u32 = dt_str[6..8].parse().ok()?;
        if len >= 15 {
            let hour: u32 = dt_str[9..11].parse().ok()?;
            let min: u32 = dt_str[11..13].parse().ok()?;
            let sec: u32 = dt_str[13..15].parse().ok()?;
            NaiveDate::from_ymd_opt(year, month, day)?
                .and_hms_opt(hour, min, sec)
        } else {
            NaiveDate::from_ymd_opt(year, month, day)?
                .and_hms_opt(0, 0, 0)
        }
    } else {
        None
    }
}

fn parse_ics_events(content: &str) -> Vec<CalendarEvent> {
    let vevent_re = Regex::new(r"(?s)BEGIN:VEVENT\n(.+?)\nEND:VEVENT").unwrap();
    let prop_re = Regex::new(r"^([^:;]+)(?:;[^:]*)?:(.*)$").unwrap();
    let mut events = Vec::new();

    for cap in vevent_re.captures_iter(content) {
        let block = &cap[1];
        let mut summary = String::new();
        let mut dtstart = None;
        let mut dtend = None;
        let mut location = None;
        let mut description = None;

        for line in block.lines() {
            let line = line.trim();
            if let Some(prop) = prop_re.captures(line) {
                let key = &prop[1];
                let value = &prop[2];
                match key {
                    "SUMMARY" => summary = value.to_string(),
                    "DTSTART" => {
                        dtstart = parse_ics_dt(value);
                    }
                    "DTEND" => {
                        dtend = parse_ics_dt(value);
                    }
                    "LOCATION" => location = Some(value.to_string()),
                    "DESCRIPTION" => description = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        if !summary.is_empty() {
            events.push(CalendarEvent {
                summary,
                start_time: dtstart,
                end_time: dtend,
                location,
                description,
            });
        }
    }

    events
}

fn demo_events(date: NaiveDate) -> Vec<CalendarEvent> {
    let weekday = date.weekday();
    let day_of_month = date.day();
    let now_hour = Local::now().hour();

    let mut events = vec![
        CalendarEvent {
            summary: "Launch day! \u{1f680}".into(),
            start_time: None,
            end_time: None,
            location: None,
            description: Some("Major product launch event".into()),
        },
        CalendarEvent {
            summary: "Product Review".into(),
            start_time: Some(
                date.and_hms_opt(
                    (now_hour + 7) % 24,
                    15,
                    0,
                )
                .unwrap(),
            ),
            end_time: Some(
                date.and_hms_opt(
                    (now_hour + 8) % 24,
                    30,
                    0,
                )
                .unwrap(),
            ),
            location: Some("Figma — Mobile Redesign".into()),
            description: Some("Review latest mockups and Figma designs".into()),
        },
        CalendarEvent {
            summary: "Design Sprint".into(),
            start_time: Some(
                date.and_hms_opt((now_hour + 2) % 24, 0, 0)
                    .unwrap(),
            ),
            end_time: Some(
                date.and_hms_opt((now_hour + 3) % 24, 0, 0)
                    .unwrap(),
            ),
            location: Some("Meeting Room A".into()),
            description: Some("Daily team sync via Google Meet".into()),
        },
        CalendarEvent {
            summary: "Lunch Break \u{1f355}".into(),
            start_time: Some(
                date.and_hms_opt((now_hour + 4) % 24, 0, 0)
                    .unwrap(),
            ),
            end_time: Some(
                date.and_hms_opt((now_hour + 5) % 24, 0, 0)
                    .unwrap(),
            ),
            location: None,
            description: None,
        },
    ];

    if day_of_month % 2 == 0
        || matches!(weekday, Weekday::Mon | Weekday::Wed | Weekday::Fri)
    {
        events.push(CalendarEvent {
            summary: "Sprint Planning".into(),
            start_time: Some(
                date.and_hms_opt((now_hour + 9) % 24, 0, 0)
                    .unwrap(),
            ),
            end_time: Some(
                date.and_hms_opt((now_hour + 10) % 24, 30, 0)
                    .unwrap(),
            ),
            location: Some("Google Docs — Sprint Notes".into()),
            description: Some("Plan the next sprint".into()),
        });
    } else {
        events.push(CalendarEvent {
            summary: "Design Review".into(),
            start_time: Some(
                date.and_hms_opt((now_hour + 9) % 24, 0, 0)
                    .unwrap(),
            ),
            end_time: Some(
                date.and_hms_opt((now_hour + 10) % 24, 0, 0)
                    .unwrap(),
            ),
            location: Some("Zoom".into()),
            description: Some("Review latest mockups".into()),
        });
    }

    if day_of_month % 5 == 0 {
        events.push(CalendarEvent {
            summary: "All-Hands Meeting".into(),
            start_time: Some(
                date.and_hms_opt((now_hour + 11) % 24, 0, 0)
                    .unwrap(),
            ),
            end_time: Some(
                date.and_hms_opt((now_hour + 12) % 24, 0, 0)
                    .unwrap(),
            ),
            location: Some("Main Auditorium".into()),
            description: Some("Monthly all-hands".into()),
        });
    }

    events
}
