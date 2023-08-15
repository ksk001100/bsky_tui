use chrono::{DateTime, FixedOffset};

pub fn get_duration_string(t1: DateTime<FixedOffset>, t2: DateTime<FixedOffset>) -> String {
    let duration = t2 - t1;

    if duration.num_weeks() != 0 {
        format!("{}w", duration.num_weeks())
    } else if duration.num_days() != 0 {
        format!("{}d", duration.num_days())
    } else if duration.num_hours() != 0 {
        format!("{}h", duration.num_hours())
    } else if duration.num_minutes() != 0 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_seconds() != 0 {
        format!("{}s", duration.num_seconds())
    } else {
        format!("{}ms", duration.num_milliseconds())
    }
}
