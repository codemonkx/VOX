use std::fs;
use std::path::Path;

use lofty::file::TaggedFileExt;
use lofty::read_from_path;
use lofty::tag::ItemKey;

#[derive(Debug, Clone)]
pub struct LrcLine {
    pub time_secs: f64,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct LrcFile {
    pub lines: Vec<LrcLine>,
}

impl LrcFile {
    pub fn load_for_track(audio_path: &str) -> Option<Self> {
        let path = Path::new(audio_path);

        // 1. Check if the song file has embedded lyrics in its metadata tags
        if let Ok(tagged_file) = read_from_path(path) {
            for tag in tagged_file.tags() {
                if let Some(lyrics_str) = tag.get_string(&ItemKey::Lyrics) {
                    if !lyrics_str.trim().is_empty() {
                        let parsed = Self::parse(lyrics_str);
                        if !parsed.lines.is_empty() {
                            return Some(parsed);
                        }
                    }
                }
                for item in tag.items() {
                    let k = format!("{:?}", item.key()).to_lowercase();
                    if k.contains("lyric") || k.contains("uslt") || k.contains("sylt") {
                        if let lofty::tag::ItemValue::Text(s) = item.value() {
                            if !s.trim().is_empty() {
                                let parsed = Self::parse(s);
                                if !parsed.lines.is_empty() {
                                    return Some(parsed);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Fallback to searching for .lrc file in track folder
        let lrc_path = path.with_extension("lrc");
        if lrc_path.exists() {
            if let Ok(content) = fs::read_to_string(&lrc_path) {
                let parsed = Self::parse(&content);
                if !parsed.lines.is_empty() {
                    return Some(parsed);
                }
            }
        }

        if let Some(parent) = path.parent() {
            if let Some(stem) = path.file_stem() {
                let candidate = parent.join(format!("{}.lrc", stem.to_string_lossy()));
                if candidate.exists() {
                    if let Ok(content) = fs::read_to_string(&candidate) {
                        let parsed = Self::parse(&content);
                        if !parsed.lines.is_empty() {
                            return Some(parsed);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn parse(content: &str) -> Self {
        let mut lines = Vec::new();
        let mut has_timestamps = false;
        let raw_lines: Vec<&str> = content.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

        for line in &raw_lines {
            let mut rest = *line;
            let mut timestamps = Vec::new();
            while rest.starts_with('[') {
                if let Some(close_idx) = rest.find(']') {
                    let timestamp_str = &rest[1..close_idx];
                    rest = &rest[close_idx + 1..];
                    if let Some(secs) = parse_timestamp(timestamp_str) {
                        timestamps.push(secs);
                        has_timestamps = true;
                    }
                } else {
                    break;
                }
            }
            let text = rest.trim().to_string();
            for ts in timestamps {
                lines.push(LrcLine {
                    time_secs: ts,
                    text: text.clone(),
                });
            }
        }

        if !has_timestamps {
            // Unsynchronized plain text lyrics (embedded in file tag)
            for (idx, line) in raw_lines.iter().enumerate() {
                lines.push(LrcLine {
                    time_secs: idx as f64 * 3.0,
                    text: line.to_string(),
                });
            }
        } else {
            lines.sort_by(|a, b| a.time_secs.partial_cmp(&b.time_secs).unwrap_or(std::cmp::Ordering::Equal));
        }

        Self { lines }
    }

    pub fn current_line_index(&self, current_pos: f64) -> Option<usize> {
        if self.lines.is_empty() {
            return None;
        }
        let mut active = None;
        for (i, line) in self.lines.iter().enumerate() {
            if line.time_secs <= current_pos {
                active = Some(i);
            } else {
                break;
            }
        }
        active
    }
}

fn parse_timestamp(ts: &str) -> Option<f64> {
    let parts: Vec<&str> = ts.split(':').collect();
    if parts.len() == 2 {
        let mins: f64 = parts[0].parse().ok()?;
        let sec_parts: Vec<&str> = parts[1].split(&['.', ','][..]).collect();
        let secs: f64 = sec_parts[0].parse().ok()?;
        let frac: f64 = if sec_parts.len() > 1 {
            let digits = sec_parts[1].len();
            let val: f64 = sec_parts[1].parse().ok()?;
            val / 10f64.powi(digits as i32)
        } else {
            0.0
        };
        Some(mins * 60.0 + secs + frac)
    } else {
        None
    }
}
