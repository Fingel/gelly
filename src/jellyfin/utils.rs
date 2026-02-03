pub fn format_duration(ticks: u64) -> String {
    // Jellyfin ticks are in 100-nanosecond intervals
    // 1 second = 10,000,000 ticks
    let seconds = ticks / 10_000_000;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let remaining_seconds = seconds % 60;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes % 60, remaining_seconds)
    } else {
        format!("{}:{:02}", minutes, remaining_seconds)
    }
}
