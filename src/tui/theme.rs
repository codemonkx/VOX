use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    Catppuccin,
    Nord,
    Dracula,
    TokyoNight,
    Gruvbox,
    Cyberpunk,
}

impl ThemeKind {
    pub const ALL: &'static [ThemeKind] = &[
        ThemeKind::Catppuccin,
        ThemeKind::Nord,
        ThemeKind::Dracula,
        ThemeKind::TokyoNight,
        ThemeKind::Gruvbox,
        ThemeKind::Cyberpunk,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ThemeKind::Catppuccin => "Catppuccin Mocha",
            ThemeKind::Nord => "Nord",
            ThemeKind::Dracula => "Dracula",
            ThemeKind::TokyoNight => "Tokyo Night",
            ThemeKind::Gruvbox => "Gruvbox Dark",
            ThemeKind::Cyberpunk => "Cyberpunk Neon",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "nord" => ThemeKind::Nord,
            "dracula" => ThemeKind::Dracula,
            "tokyonight" | "tokyo night" | "tokyo_night" => ThemeKind::TokyoNight,
            "gruvbox" => ThemeKind::Gruvbox,
            "cyberpunk" => ThemeKind::Cyberpunk,
            _ => ThemeKind::Catppuccin,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            ThemeKind::Catppuccin => "catppuccin",
            ThemeKind::Nord => "nord",
            ThemeKind::Dracula => "dracula",
            ThemeKind::TokyoNight => "tokyonight",
            ThemeKind::Gruvbox => "gruvbox",
            ThemeKind::Cyberpunk => "cyberpunk",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThemePalette {
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub accent: Color,
    pub accent_secondary: Color,
    pub _bg_highlight: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_dim: Color,
    pub text_highlight: Color,
    pub title: Color,
    pub progress_fill: Color,
    pub progress_track: Color,
    pub visualizer_low: Color,
    pub visualizer_mid: Color,
    pub visualizer_high: Color,
}

impl ThemePalette {
    pub fn for_kind(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::Catppuccin => Self {
                border_focused: Color::Rgb(203, 166, 247), // Mauve
                border_unfocused: Color::Rgb(88, 91, 112),  // Surface2
                accent: Color::Rgb(137, 180, 250),          // Blue
                accent_secondary: Color::Rgb(148, 226, 213),// Teal
                _bg_highlight: Color::Rgb(203, 166, 247),    // Mauve
                text_primary: Color::Rgb(205, 214, 244),    // Text
                text_secondary: Color::Rgb(186, 194, 222),  // Subtext1
                text_dim: Color::Rgb(108, 112, 134),        // Overlay0
                text_highlight: Color::Rgb(17, 17, 27),     // Crust
                title: Color::Rgb(245, 194, 231),           // Pink
                progress_fill: Color::Rgb(137, 180, 250),   // Blue
                progress_track: Color::Rgb(49, 50, 68),     // Surface0
                visualizer_low: Color::Rgb(148, 226, 213),  // Teal
                visualizer_mid: Color::Rgb(137, 180, 250),  // Blue
                visualizer_high: Color::Rgb(203, 166, 247), // Mauve
            },
            ThemeKind::Nord => Self {
                border_focused: Color::Rgb(136, 192, 208), // Frost Cyan
                border_unfocused: Color::Rgb(76, 86, 106),  // Polar Night
                accent: Color::Rgb(129, 161, 193),          // Frost Blue
                accent_secondary: Color::Rgb(143, 188, 187),// Frost Green
                _bg_highlight: Color::Rgb(136, 192, 208),
                text_primary: Color::Rgb(236, 239, 244),    // Snow Storm
                text_secondary: Color::Rgb(229, 233, 240),
                text_dim: Color::Rgb(94, 129, 172),
                text_highlight: Color::Rgb(46, 52, 64),
                title: Color::Rgb(136, 192, 208),
                progress_fill: Color::Rgb(129, 161, 193),
                progress_track: Color::Rgb(59, 66, 82),
                visualizer_low: Color::Rgb(163, 190, 140),  // Aurora Green
                visualizer_mid: Color::Rgb(136, 192, 208),  // Frost Cyan
                visualizer_high: Color::Rgb(180, 142, 173), // Aurora Purple
            },
            ThemeKind::Dracula => Self {
                border_focused: Color::Rgb(255, 121, 198), // Pink
                border_unfocused: Color::Rgb(98, 114, 164), // Comment
                accent: Color::Rgb(189, 147, 249),          // Purple
                accent_secondary: Color::Rgb(139, 233, 253),// Cyan
                _bg_highlight: Color::Rgb(255, 121, 198),
                text_primary: Color::Rgb(248, 248, 242),    // Foreground
                text_secondary: Color::Rgb(189, 147, 249),
                text_dim: Color::Rgb(98, 114, 164),
                text_highlight: Color::Rgb(40, 42, 54),
                title: Color::Rgb(241, 250, 140),           // Yellow
                progress_fill: Color::Rgb(255, 121, 198),
                progress_track: Color::Rgb(68, 71, 90),
                visualizer_low: Color::Rgb(80, 250, 123),   // Green
                visualizer_mid: Color::Rgb(139, 233, 253),  // Cyan
                visualizer_high: Color::Rgb(255, 121, 198), // Pink
            },
            ThemeKind::TokyoNight => Self {
                border_focused: Color::Rgb(122, 162, 247), // Blue
                border_unfocused: Color::Rgb(65, 72, 104),
                accent: Color::Rgb(187, 154, 247),          // Magenta
                accent_secondary: Color::Rgb(125, 207, 255),// Cyan
                _bg_highlight: Color::Rgb(122, 162, 247),
                text_primary: Color::Rgb(192, 202, 245),
                text_secondary: Color::Rgb(169, 177, 214),
                text_dim: Color::Rgb(86, 95, 137),
                text_highlight: Color::Rgb(26, 27, 38),
                title: Color::Rgb(255, 158, 100),           // Orange
                progress_fill: Color::Rgb(187, 154, 247),
                progress_track: Color::Rgb(36, 40, 59),
                visualizer_low: Color::Rgb(158, 206, 106),  // Green
                visualizer_mid: Color::Rgb(122, 162, 247),  // Blue
                visualizer_high: Color::Rgb(247, 118, 142), // Red/Pink
            },
            ThemeKind::Gruvbox => Self {
                border_focused: Color::Rgb(250, 189, 47),  // Yellow
                border_unfocused: Color::Rgb(102, 92, 84),
                accent: Color::Rgb(254, 128, 25),           // Orange
                accent_secondary: Color::Rgb(142, 192, 124),// Aqua
                _bg_highlight: Color::Rgb(250, 189, 47),
                text_primary: Color::Rgb(235, 219, 178),
                text_secondary: Color::Rgb(213, 196, 161),
                text_dim: Color::Rgb(146, 131, 116),
                text_highlight: Color::Rgb(40, 40, 40),
                title: Color::Rgb(251, 73, 52),             // Red
                progress_fill: Color::Rgb(250, 189, 47),
                progress_track: Color::Rgb(60, 56, 54),
                visualizer_low: Color::Rgb(184, 187, 38),   // Green
                visualizer_mid: Color::Rgb(250, 189, 47),   // Yellow
                visualizer_high: Color::Rgb(254, 128, 25),  // Orange
            },
            ThemeKind::Cyberpunk => Self {
                border_focused: Color::Rgb(0, 255, 240),    // Electric Cyan
                border_unfocused: Color::Rgb(60, 0, 80),
                accent: Color::Rgb(255, 0, 127),            // Neon Pink
                accent_secondary: Color::Rgb(255, 240, 0),  // Neon Yellow
                _bg_highlight: Color::Rgb(0, 255, 240),
                text_primary: Color::Rgb(255, 255, 255),
                text_secondary: Color::Rgb(0, 255, 240),
                text_dim: Color::Rgb(100, 100, 150),
                text_highlight: Color::Rgb(20, 0, 40),
                title: Color::Rgb(255, 240, 0),
                progress_fill: Color::Rgb(255, 0, 127),
                progress_track: Color::Rgb(40, 0, 60),
                visualizer_low: Color::Rgb(0, 255, 240),
                visualizer_mid: Color::Rgb(255, 240, 0),
                visualizer_high: Color::Rgb(255, 0, 127),
            },
        }
    }
}
