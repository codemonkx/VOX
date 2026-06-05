pub fn format_duration(secs: f64) -> String {
    let secs = secs as u64;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

pub fn format_bitrate(bitrate: u32) -> String {
    format!("{bitrate} kbps")
}

pub fn format_sample_rate(hz: u32) -> String {
    if hz >= 1000 {
        format!("{:.1} kHz", hz as f64 / 1000.0)
    } else {
        format!("{hz} Hz")
    }
}
