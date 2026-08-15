use ratatui::buffer::Buffer;
use ratatui::style::Style;
use ratatui::text::Span;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

/// Computes the true terminal display width for any Unicode grapheme cluster.
/// In terminal emulators (GNOME Terminal, VTE, Kitty, Alacritty), Tamil grapheme clusters
/// occupy exactly 1 column, whereas CJK ideographs/fullwidth characters occupy 2 columns,
/// and control/format marks occupy 0 columns.
pub fn grapheme_width(g: &str) -> usize {
    if g.is_empty() {
        return 0;
    }

    // Check if the grapheme contains Tamil characters (U+0B80..=U+0BFF)
    let has_tamil = g.chars().any(|c| ('\u{0B80}'..='\u{0BFF}').contains(&c));
    if has_tamil {
        // In terminal grids (VTE/GNOME Terminal), every Tamil grapheme cluster (e.g. கா, டு, கு, னோ, ழ்)
        // is rendered in exactly 1 terminal cell.
        return 1;
    }

    // Check if it's other Indic or complex scripts that render in 1 cell per cluster
    let has_indic = g.chars().any(|c| ('\u{0900}'..='\u{0DFF}').contains(&c));
    if has_indic {
        return 1;
    }

    // For other scripts (Latin, CJK, Emoji), determine width from base characters
    let mut total = 0;
    for c in g.chars() {
        if let Some(w) = c.width() {
            total += w;
        }
    }

    if total > 2 {
        let first_char_w = g.chars().next().and_then(|c| c.width()).unwrap_or(1);
        first_char_w.max(1)
    } else if total == 0 {
        0
    } else {
        total
    }
}

/// Returns the total visual display width of a string in terminal columns.
pub fn str_display_width(s: &str) -> usize {
    s.graphemes(true).map(grapheme_width).sum()
}

/// Safely truncates a string to fit within `max_w` display columns, never splitting a grapheme cluster.
#[allow(dead_code)]
pub fn truncate_to_width(s: &str, max_w: usize) -> String {
    let mut cur_w = 0;
    let mut result = String::new();

    for g in s.graphemes(true) {
        let w = grapheme_width(g);
        if cur_w + w > max_w {
            break;
        }
        result.push_str(g);
        cur_w += w;
    }

    result
}

/// Safely truncates a file path from the beginning, never splitting UTF-8 characters or grapheme clusters.
pub fn truncate_path_safe(p: &str, max_w: usize) -> String {
    let total_w = str_display_width(p);
    if max_w < 4 || total_w <= max_w {
        return p.to_string();
    }

    let target_w = max_w.saturating_sub(2);
    let mut cur_w = 0;
    let graphemes: Vec<&str> = p.graphemes(true).collect();
    let mut keep_idx = graphemes.len();

    for (i, g) in graphemes.iter().enumerate().rev() {
        let w = grapheme_width(g);
        if cur_w + w > target_w {
            break;
        }
        cur_w += w;
        keep_idx = i;
    }

    format!("..{}", graphemes[keep_idx..].concat())
}

#[allow(dead_code)]
pub fn draw_text_to_buffer(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    max_w: u16,
    text: &str,
    style: Style,
) -> usize {
    let mut cur_x = x;
    let end_x = x.saturating_add(max_w);

    for g in text.graphemes(true) {
        let w = grapheme_width(g);
        if w == 0 {
            continue;
        }
        if cur_x + (w as u16) > end_x {
            break;
        }
        if let Some(cell) = buf.cell_mut((cur_x, y)) {
            cell.set_symbol(g);
            cell.set_style(style);
        }
        cur_x += w as u16;
    }

    (cur_x - x) as usize
}

/// Renders a series of Spans into a Ratatui buffer as contiguous text units to prevent
/// cursor jump desynchronization in terminal emulators.
pub fn draw_spans_to_buffer(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    max_w: u16,
    spans: &[Span],
) -> usize {
    let mut cur_x = x;
    let end_x = x.saturating_add(max_w);

    for span in spans {
        if cur_x >= end_x || span.content.is_empty() {
            continue;
        }

        let total_span_w = str_display_width(&span.content) as u16;
        if cur_x + total_span_w > end_x {
            let available = (end_x - cur_x) as usize;
            let truncated = truncate_to_width(&span.content, available);
            let trunc_w = str_display_width(&truncated) as u16;
            if let Some(cell) = buf.cell_mut((cur_x, y)) {
                cell.set_symbol(&truncated);
                cell.set_style(span.style);
            }
            for i in 1..trunc_w {
                if let Some(cell) = buf.cell_mut((cur_x + i, y)) {
                    cell.set_symbol("");
                    cell.set_style(span.style);
                }
            }
            cur_x += trunc_w;
            break;
        } else {
            if let Some(cell) = buf.cell_mut((cur_x, y)) {
                cell.set_symbol(&span.content);
                cell.set_style(span.style);
            }
            for i in 1..total_span_w {
                if let Some(cell) = buf.cell_mut((cur_x + i, y)) {
                    cell.set_symbol("");
                    cell.set_style(span.style);
                }
            }
            cur_x += total_span_w;
        }
    }

    (cur_x - x) as usize
}
