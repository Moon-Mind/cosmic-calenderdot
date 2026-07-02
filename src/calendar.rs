use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Weekday};
use regex::Regex;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub summary: String,
    pub start_time: Option<NaiveDateTime>,
    pub end_time: Option<NaiveDateTime>,
    pub all_day: bool,
}

/// Computes the first date visible in the calendar grid for a given month,
/// starting from `first_weekday` (e.g. Sunday).
pub fn get_calendar_first(year: i32, month: u32, first_weekday: Weekday) -> NaiveDate {
    let first_of_month = NaiveDate::from_ymd_opt(year, month, 1).expect("valid date");
    let current = first_of_month.weekday().num_days_from_sunday() as i64;
    let target = first_weekday.num_days_from_sunday() as i64;
    let diff = (current - target + 7) % 7;
    first_of_month - Duration::days(diff)
}

/// Search standard paths for vdirsyncer calendar collections and return all events
/// in the given date range [start, end].
pub fn fetch_events(start: NaiveDate, end: NaiveDate) -> Vec<CalendarEvent> {
    let mut events = Vec::new();

    for dir in vdirsyncer_paths().into_iter().flatten() {
        if dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "ics") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Some(event) = parse_ics_event(&content) {
                                let date = event.start_time.map(|t| t.date());
                                if date.map_or(false, |d| d >= start && d <= end)
                                    || (event.all_day && date.map_or(false, |d| d <= end))
                                {
                                    events.push(event);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    events.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    events
}

fn vdirsyncer_paths() -> Vec<Option<PathBuf>> {
    vec![
        dirs::data_dir().map(|d| d.join("vdirsyncer")),
        dirs::data_local_dir().map(|d| d.join("vdirsyncer")),
        dirs::data_dir().map(|d| d.join("calendars")),
        dirs::data_local_dir().map(|d| d.join("calendars")),
        dirs::config_dir().map(|d| d.join("calendar")),
    ]
}

fn parse_ics_event(content: &str) -> Option<CalendarEvent> {
    let re = Regex::new(r"(?s)BEGIN:VEVENT\n(.+?)\nEND:VEVENT").ok()?;
    let cap = re.captures(content)?;
    let block = &cap[1];

    let prop_re = Regex::new(r"^([^:;]+)(?:;[^:]*)?:(.*)$").ok()?;

    let mut summary = String::new();
    let mut dtstart = None;
    let mut dtend = None;
    let mut all_day = false;

    for line in block.lines() {
        let line = line.trim();
        if let Some(prop) = prop_re.captures(line) {
            let key = &prop[1];
            let value = &prop[2];
            match key {
                "SUMMARY" => summary = value.to_string(),
                "DTSTART" => {
                    if !value.contains('T') && !value.contains('Z') {
                        all_day = true;
                    }
                    dtstart = parse_ics_dt(value);
                }
                "DTSTART;VALUE=DATE" => {
                    all_day = true;
                    dtstart = parse_ics_dt(value);
                }
                "DTEND" => {
                    dtend = parse_ics_dt(value);
                }
                "DTEND;VALUE=DATE" => {
                    dtend = parse_ics_dt(value);
                }
                _ => {}
            }
        }
    }

    if summary.is_empty() {
        return None;
    }

    Some(CalendarEvent {
        summary,
        start_time: dtstart,
        end_time: dtend,
        all_day,
    })
}

fn parse_ics_dt(dt_str: &str) -> Option<NaiveDateTime> {
    let len = dt_str.len();
    if len >= 8 {
        let year: i32 = dt_str[0..4].parse().ok()?;
        let month: u32 = dt_str[4..6].parse().ok()?;
        let day: u32 = dt_str[6..8].parse().ok()?;
        if len >= 15 {
            // Has time component
            let hour: u32 = dt_str[9..11].parse().ok()?;
            let min: u32 = dt_str[11..13].parse().ok()?;
            let sec: u32 = dt_str[13..15].parse().ok()?;
            NaiveDate::from_ymd_opt(year, month, day)?
                .and_hms_opt(hour, min, sec)
        } else {
            // Date-only
            NaiveDate::from_ymd_opt(year, month, day)?
                .and_hms_opt(0, 0, 0)
        }
    } else {
        None
    }
}
