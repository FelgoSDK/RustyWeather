use chrono::{DateTime, Datelike, TimeZone};

pub fn get_day_from_datetime(date: DateTime<chrono::offset::Utc>) -> String {
    if date.day() == chrono::offset::Local::now().day() {
        // TODO: translations
        return "Today".to_string();
    }
    date.weekday().to_string()    
}

pub fn timestamp_to_string(timestamp: i64) -> String {
    if let Some(time) = chrono::Local.timestamp_opt(timestamp, 0).single() {
        return time.to_string();
    }
    "Unknown".to_string()
}