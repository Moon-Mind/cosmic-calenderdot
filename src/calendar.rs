use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Weekday};
use cosmic::iced::Color;
use regex::Regex;
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub summary: String,
    pub start_time: Option<NaiveDateTime>,
    pub end_time: Option<NaiveDateTime>,
    pub all_day: bool,
    pub color: Color,
}

const PALETTE: [Color; 8] = [
    Color::from_rgb(0.384, 0.627, 0.918), // #62a0ea / blue
    Color::from_rgb(1.000, 0.749, 0.435), // #ffbf70 / orange
    Color::from_rgb(0.667, 0.804, 0.467), // #aacd77 / green
    Color::from_rgb(0.957, 0.475, 0.475), // #f47979 / red
    Color::from_rgb(0.580, 0.584, 0.902), // #9495e6 / purple
    Color::from_rgb(0.467, 0.800, 0.780), // #77ccc7 / teal
    Color::from_rgb(0.961, 0.635, 0.525), // #f5a286 / salmon
    Color::from_rgb(0.322, 0.792, 0.843), // #52cad7 / cyan
];

fn hash_to_color(hash: &str) -> Color {
    let h: u64 = hash.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    PALETTE[(h as usize) % PALETTE.len()]
}

/// Computes the first date visible in the calendar grid for a given month.
pub fn get_calendar_first(year: i32, month: u32, first_weekday: Weekday) -> NaiveDate {
    let first_of_month = NaiveDate::from_ymd_opt(year, month, 1).expect("valid date");
    let current = first_of_month.weekday().num_days_from_sunday() as i64;
    let target = first_weekday.num_days_from_sunday() as i64;
    let diff = (current - target + 7) % 7;
    first_of_month - Duration::days(diff)
}

pub fn all_cache_db_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for dir in calendar_paths().into_iter().flatten() {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let sub = entry.path();
                if sub.is_dir() {
                    let cache_db = sub.join("cache.db");
                    if cache_db.is_file() {
                        paths.push(cache_db);
                    }
                }
            }
        }
        let cache_db = dir.join("cache.db");
        if cache_db.is_file() {
            paths.push(cache_db);
        }
    }
    paths
}

pub fn cache_db_modified(path: &PathBuf) -> Option<SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

/// Search EDS (Evolution Data Server) cache for calendar events in [start, end].
pub fn fetch_events(start: NaiveDate, end: NaiveDate) -> Vec<CalendarEvent> {
    let mut events = Vec::new();

    for dir in calendar_paths().into_iter().flatten() {
        if !dir.is_dir() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let sub = entry.path();
                if sub.is_dir() {
                    let cache_db = sub.join("cache.db");
                    if cache_db.is_file() {
                        let dir_hash = sub
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("default");
                        scan_cache_db(&cache_db, &mut events, start, end, dir_hash);
                    }
                }
            }
        }

        let cache_db = dir.join("cache.db");
        if cache_db.is_file() {
            scan_cache_db(&cache_db, &mut events, start, end, "default");
        }

        scan_ics_files(&dir, &mut events, start, end);
    }

    events.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    events
}

fn scan_cache_db(
    path: &PathBuf,
    events: &mut Vec<CalendarEvent>,
    start: NaiveDate,
    end: NaiveDate,
    dir_hash: &str,
) {
    let color = hash_to_color(dir_hash);
    if let Ok(db) = Connection::open(path) {
        let mut count = 0;
        let mut parsed = 0;
        let mut in_range = 0;
        if let Ok(mut stmt) = db.prepare("SELECT ECacheOBJ, summary FROM ECacheObjects") {
            if let Ok(rows) = stmt.query_map([], |row| {
                let obj: String = row.get::<_, String>(0).unwrap_or_default();
                let summary: String = row.get(1).unwrap_or_default();
                Ok((obj, summary))
            }) {
                for row in rows.flatten() {
                    count += 1;
                    let (obj, summary) = row;
                    if let Some(mut event) = parse_ics_event(&obj) {
                        parsed += 1;
                        if event.summary.is_empty() && !summary.is_empty() {
                            event.summary = summary;
                        }
                        event.color = color;
                        if event_in_range(&event, start, end) {
                            in_range += 1;
                            events.push(event);
                        }
                    } else if !summary.is_empty() {
                        tracing::debug!("fallback to occur_start for: {}", summary);
                        if let Ok(mut dt_stmt) = db.prepare(
                            "SELECT occur_start, occur_end FROM ECacheObjects WHERE summary = ?1",
                        ) {
                            if let Ok(dt_rows) =
                                dt_stmt.query_map([&summary], |row| {
                                    let s: Option<String> = row.get(0).ok();
                                    let e: Option<String> = row.get(1).ok();
                                    Ok((s, e))
                                })
                            {
                                for dt_row in dt_rows.flatten() {
                                    let (start_str, end_str) = dt_row;
                                    let start_dt = start_str
                                        .as_deref()
                                        .and_then(parse_iso_datetime);
                                    let end_dt = end_str
                                        .as_deref()
                                        .and_then(parse_iso_datetime);
                                    let event = CalendarEvent {
                                        summary: summary.clone(),
                                        start_time: start_dt,
                                        end_time: end_dt,
                                        all_day: false,
                                        color,
                                    };
                                    if event_in_range(&event, start, end) {
                                        in_range += 1;
                                        events.push(event);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        tracing::debug!("scan_cache_db: {} rows, {} parsed, {} in range", count, parsed, in_range);
    }
}

fn event_in_range(event: &CalendarEvent, start: NaiveDate, end: NaiveDate) -> bool {
    event.start_time.map_or(false, |t| {
        let d = t.date();
        d >= start && d <= end
    })
}

fn calendar_paths() -> Vec<Option<PathBuf>> {
    vec![
        dirs::cache_dir().map(|d| d.join("evolution").join("calendar")),
        dirs::data_dir().map(|d| d.join("evolution").join("calendar")),
    ]
}

fn scan_ics_files(
    dir: &PathBuf,
    events: &mut Vec<CalendarEvent>,
    start: NaiveDate,
    end: NaiveDate,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "ics") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(event) = parse_ics_event(&content) {
                        if event_in_range(&event, start, end) {
                            events.push(event);
                        }
                    }
                }
            }
            if path.is_dir() {
                if let Ok(sub) = fs::read_dir(&path) {
                    for sub_entry in sub.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.extension().map_or(false, |e| e == "ics") {
                            if let Ok(content) = fs::read_to_string(&sub_path) {
                                if let Some(event) = parse_ics_event(&content) {
                                    if event_in_range(&event, start, end) {
                                        events.push(event);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn parse_ics_event(content: &str) -> Option<CalendarEvent> {
    let re = Regex::new(r"(?s)BEGIN:VEVENT\r?\n(.+?)\r?\nEND:VEVENT").ok()?;
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
                "DTEND" => {
                    dtend = parse_ics_dt(value);
                }
                _ => {}
            }
        }
    }

    if summary.is_empty() && dtstart.is_none() {
        return None;
    }

    Some(CalendarEvent {
        summary,
        start_time: dtstart,
        end_time: dtend,
        all_day,
        color: Color::from_rgb(0.322, 0.792, 0.843),
    })
}

fn parse_ics_dt(dt_str: &str) -> Option<NaiveDateTime> {
    let dt_str = dt_str.trim();
    let len = dt_str.len();
    if len >= 8 {
        let year: i32 = dt_str[0..4].parse().ok()?;
        let month: u32 = dt_str[4..6].parse().ok()?;
        let day: u32 = dt_str[6..8].parse().ok()?;
        if len >= 15 && dt_str.as_bytes()[8] == b'T' {
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

fn parse_iso_datetime(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").ok()
        .or_else(|| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").ok())
}
