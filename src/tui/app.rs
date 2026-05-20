use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
    MouseButton, MouseEventKind,
};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;
use ratatui::Terminal;

use crate::audio::Player;
use crate::config::Config;
use crate::library::Library;
use crate::metadata::Track;
use crate::utils;

enum InputMode {
    None,
    Browse,
    Search,
    RemovePath,
}

pub struct App {
    library: Library,
    player: Player,
    config: Config,

    all_tracks: Vec<Track>,
    album_names: Vec<String>,
    album_tracks: Vec<Track>,
    selected_album: usize,
    prev_album: usize,
    selected_track: usize,
    focus: Focus,

    current_path: String,
    current_meta: Option<Track>,
    track_ended: bool,
    exit: bool,

    input_mode: InputMode,
    remove_paths: Vec<String>,
    remove_path_selection: usize,
    search_query: String,
    search_results: Vec<Track>,
    status_msg: String,

    browser_cwd: PathBuf,
    browser_entries: Vec<(String, bool)>,
    browser_selection: usize,

    last_click_col: u16,
    last_click_row: u16,
    last_click_time: Instant,
}

enum Focus {
    Albums,
    Tracks,
}

impl App {
    pub fn new(config: Config, db: Arc<crate::db::Database>, player: Player) -> Result<Self> {
        let library = Library::new(db);
        let all_tracks = library.list_tracks().unwrap_or_default();
        let album_names = library.album_names().unwrap_or_default();
        let remove_paths = config.music_dirs.iter().map(|p| p.to_string_lossy().to_string()).collect();

        let browser_cwd = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));

        let mut app = Self {
            library,
            player,
            config,
            current_meta: None,
            all_tracks,
            album_names,
            album_tracks: Vec::new(),
            selected_album: 0,
            prev_album: usize::MAX,
            selected_track: 0,
            focus: Focus::Albums,
            current_path: String::new(),
            track_ended: false,
            exit: false,
            input_mode: InputMode::None,
            remove_paths,
            remove_path_selection: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            status_msg: String::new(),
            browser_cwd,
            browser_entries: Vec::new(),
            browser_selection: 0,
            last_click_col: 0,
            last_click_row: 0,
            last_click_time: Instant::now(),
        };
        app.load_album_tracks();
        app.load_browser();
        Ok(app)
    }

    fn load_browser(&mut self) {
        self.browser_entries.clear();
        self.browser_selection = 0;

        if self.browser_cwd.parent().is_some() {
            self.browser_entries.push(("..".to_string(), true));
        }

        let mut dirs: Vec<(String, bool)> = match std::fs::read_dir(&self.browser_cwd) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .map(|e| (e.file_name().to_string_lossy().to_string(), true))
                .collect(),
            Err(_) => Vec::new(),
        };
        dirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        self.browser_entries.append(&mut dirs);
    }

    fn load_album_tracks(&mut self) {
        if self.selected_album == self.prev_album {
            return;
        }
        self.prev_album = self.selected_album;
        self.album_tracks = match self.album_names.get(self.selected_album) {
            Some(album) => self
                .all_tracks
                .iter()
                .filter(|t| t.album == *album)
                .cloned()
                .collect(),
            None => Vec::new(),
        };
        self.selected_track = 0;
    }

    pub fn run(&mut self) -> Result<()> {
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        stdout.execute(EnterAlternateScreen)?;
        stdout.execute(EnableMouseCapture)?;
        let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;

        while !self.exit {
            terminal.draw(|f| self.render(f))?;

            self.check_track_end();
            self.handle_events()?;
            self.update_metadata();
        }

        disable_raw_mode()?;
        let mut stdout = std::io::stdout();
        stdout.execute(DisableMouseCapture)?;
        stdout.execute(LeaveAlternateScreen)?;
        println!("Bye!");
        Ok(())
    }

    fn check_track_end(&mut self) {
        if self.current_meta.is_none() {
            return;
        }
        if self.player.is_empty() && !self.current_path.is_empty() && !self.player.is_paused() {
            if !self.track_ended {
                self.track_ended = true;
                self.next_track();
            }
        } else {
            self.track_ended = false;
        }
    }

    fn update_metadata(&mut self) {
        if self.player.is_playing() || self.player.is_paused() {
            let path = self.current_path.clone();
            let needs = match &self.current_meta {
                None => true,
                Some(m) => m.path != path,
            };
            if needs && !path.is_empty() {
                let p = std::path::Path::new(&path);
                self.current_meta = crate::metadata::read_track(p).ok();
            }
        }
    }

    fn refresh_library(&mut self) {
        self.all_tracks = self.library.list_tracks().unwrap_or_default();
        self.album_names = self.library.album_names().unwrap_or_default();
        if self.selected_album >= self.album_names.len() {
            self.selected_album = self.album_names.len().saturating_sub(1);
        }
        self.prev_album = usize::MAX;
        self.load_album_tracks();
    }

    fn render(&mut self, f: &mut Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(area);

        self.render_top_bar(f, chunks[0]);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
            .split(chunks[1]);

        self.render_left_panel(f, main_chunks[0]);
        self.render_right_panel(f, main_chunks[1]);
        self.render_bottom_bar(f, chunks[2]);
        self.render_help_bar(f, chunks[3]);
    }

    fn render_top_bar(&self, f: &mut Frame, area: Rect) {
        let msg = match self.input_mode {
            InputMode::Browse => {
                format!(" Select folder: {}", self.browser_cwd.display())
            }
            InputMode::RemovePath => {
                format!(" Select path to remove ({}/{})", self.remove_path_selection + 1, self.remove_paths.len())
            }
            InputMode::Search => {
                format!(" Search: {}", self.search_query)
            }
            InputMode::None => {
                if !self.status_msg.is_empty() {
                    format!(" {}", self.status_msg)
                } else {
                    let mut parts: Vec<String> = vec![" [/] add folder  [x] remove path  [F] search".into()];
                    if self.config.repeat {
                        parts.push(" 🔁 repeat".into());
                    }
                    if self.config.shuffle {
                        parts.push(" 🔀 shuffle".into());
                    }
                    parts.push(" | Tab to switch | Q to quit".into());
                    parts.join("")
                }
            }
        };
        let style = match self.input_mode {
            InputMode::Browse => Style::default().fg(Color::Black).bg(Color::Cyan),
            InputMode::RemovePath => Style::default().fg(Color::Black).bg(Color::Red),
            InputMode::Search => Style::default().fg(Color::Black).bg(Color::Yellow),
            InputMode::None => Style::default().fg(Color::DarkGray),
        };
        let bar = Paragraph::new(Line::from(Span::styled(msg, style)));
        f.render_widget(bar, area);
    }

    fn render_left_panel(&self, f: &mut Frame, area: Rect) {
        if matches!(self.input_mode, InputMode::RemovePath) {
            self.render_remove_path_list(f, area);
            return;
        }
        if matches!(self.input_mode, InputMode::Browse) {
            self.render_browser(f, area);
            return;
        }
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(10), Constraint::Min(1)])
            .split(area);

        self.render_meta_panel(f, chunks[0]);
        self.render_track_list(f, chunks[1]);
    }

    fn render_meta_panel(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let meta = self
            .current_meta
            .as_ref()
            .or_else(|| self.album_tracks.get(self.selected_track));

        let fields: Vec<(&str, String, Color)> = match meta {
            Some(t) => vec![
                ("Name", t.title.clone(), Color::White),
                ("Album", t.album.clone(), Color::Yellow),
                ("Length", utils::format_duration(t.duration), Color::DarkGray),
                ("Bitrate", utils::format_bitrate(t.bitrate), Color::DarkGray),
                ("Sample", utils::format_sample_rate(t.sample_rate), Color::DarkGray),
                ("Artist", t.artist.clone(), Color::Cyan),
            ],
            None => vec![],
        };

        let label_w = fields.iter().map(|(l, _, _)| l.len()).max().unwrap_or(0);
        let mut rows: Vec<Line> = Vec::new();
        for (label, value, color) in &fields {
            let pad = " ".repeat(label_w - label.len());
            let lbl = format!("  {pad}{label}:");
            rows.push(Line::from(vec![
                Span::styled(lbl, Style::default().fg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(value.clone(), Style::default().fg(*color).add_modifier(Modifier::BOLD)),
            ]));
        }
        if rows.is_empty() {
            rows.push(Line::from(Span::styled(
                "  No track selected",
                Style::default().fg(Color::DarkGray),
            )));
        }
        f.render_widget(Paragraph::new(rows), inner);
    }

    fn render_track_list(&self, f: &mut Frame, area: Rect) {
        let in_search = matches!(self.input_mode, InputMode::Search);
        let tracks: &[Track] = if in_search { &self.search_results } else { &self.album_tracks };
        let sel = if in_search { self.selected_track.min(tracks.len().saturating_sub(1)) } else { self.selected_track };

        let title = if in_search {
            " Search results".into()
        } else {
            self.album_names
                .get(self.selected_album)
                .map(|a| format!(" Tracks — {a}"))
                .unwrap_or_default()
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .style(if matches!(self.focus, Focus::Tracks) || in_search {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let inner = block.inner(area);
        f.render_widget(block, area);

        if tracks.is_empty() {
            let empty = Paragraph::new(if in_search { " No results" } else { " No tracks" })
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty, inner);
            return;
        }

        let visible = inner.height as usize;
        let scroll = if sel >= visible {
            sel - visible + 1
        } else {
            0
        };

        let title_w = inner.width.saturating_sub(14) as usize;

        let items: Vec<ListItem> = tracks
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible)
            .map(|(i, t)| {
                let prefix = if i == sel { "▸ " } else { "  " };
                let num = format!("{:02}.", i + 1);
                let dur = utils::format_duration(t.duration);
                let now = if t.path == self.current_path { " ♪" } else { "" };
                let content = format!("{prefix}{num} {:<title_w$} {:>5}{now}", t.title, dur, title_w = title_w);
                let style = if i == sel {
                    if matches!(self.focus, Focus::Tracks) || in_search {
                        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Cyan)
                    }
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(content).style(style)
            })
            .collect();

        f.render_widget(List::new(items), inner);
    }

    fn render_right_panel(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Albums ")
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .style(if matches!(self.focus, Focus::Albums) {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            });
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.album_names.is_empty() {
            let empty = Paragraph::new(" No albums found. Run `music scan <folder>`")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty, inner);
            return;
        }

        let visible = inner.height as usize;
        let scroll = if self.selected_album >= visible {
            self.selected_album - visible + 1
        } else {
            0
        };

        let items: Vec<ListItem> = self
            .album_names
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible)
            .map(|(i, name)| {
                let count = self.all_tracks.iter().filter(|t| t.album == *name).count();
                let prefix = if i == self.selected_album { "▸ " } else { "  " };
                let now = if self.all_tracks.iter().any(|t| t.album == *name && t.path == self.current_path)
                {
                    " ♪"
                } else {
                    ""
                };
                let content = format!("{prefix}{name}  ({count} tracks){now}");
                let style = if i == self.selected_album {
                    if matches!(self.focus, Focus::Albums) {
                        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Cyan)
                    }
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(content).style(style)
            })
            .collect();

        f.render_widget(List::new(items), inner);
    }

    fn render_remove_path_list(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Remove tracked paths ")
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .style(Style::default().fg(Color::Red));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if self.remove_paths.is_empty() {
            let empty = Paragraph::new(" No tracked paths. Add one with [/].")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(empty, inner);
            return;
        }

        let sel = self.remove_path_selection.min(self.remove_paths.len().saturating_sub(1));
        let visible = inner.height as usize;
        let scroll = if sel >= visible { sel - visible + 1 } else { 0 };

        let items: Vec<ListItem> = self
            .remove_paths
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible)
            .map(|(i, p)| {
                let prefix = if i == sel { "▸ " } else { "  " };
                let style = if i == sel {
                    Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let max_w = inner.width.saturating_sub(3) as usize;
                let display = truncate_path(p, max_w);
                ListItem::new(format!("{prefix}{display}")).style(style)
            })
            .collect();

        f.render_widget(List::new(items), inner);
    }

    fn render_browser(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Browse folders ")
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let sel = self.browser_selection.min(self.browser_entries.len().saturating_sub(1));
        let visible = inner.height as usize;
        let scroll = if sel >= visible { sel - visible + 1 } else { 0 };

        let items: Vec<ListItem> = self
            .browser_entries
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible)
            .map(|(i, (name, is_dir))| {
                let prefix = if i == sel { "▸ " } else { "  " };
                let display = if *is_dir { format!("{}/", name) } else { name.clone() };
                let style = if i == sel {
                    Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if *is_dir {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(format!("{prefix}{display}")).style(style)
            })
            .collect();

        f.render_widget(List::new(items), inner);
    }

    fn render_bottom_bar(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(border::ROUNDED)
            .style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let tcount = self.all_tracks.len();
        let codec = self.current_meta.as_ref().map(|m| m.codec.as_str()).unwrap_or("--");
        let elapsed = utils::format_duration(self.player.current_position());
        let total = utils::format_duration(self.player.current_duration());
        let pos = self.player.current_position();
        let dur = self.player.current_duration();
        let bar_w = inner.width.saturating_sub(48) as usize;
        let bar = utils::progress_bar(pos, dur, bar_w);

        let playing = match &self.current_meta {
            Some(m) => format!("{} — {}", m.title, m.artist),
            None => "No track playing".into(),
        };

        let vol = self.player.get_volume();
        let vol_pct = (vol * 100.0) as u8;
        let vol_str = if self.player.is_muted() {
            " MUTED ".to_string()
        } else {
            format!(" Vol: {vol_pct}% ")
        };

        let left = format!(" {tcount} items ");
        let center = format!(" {playing} ");
        let right = format!(" {vol_str}{codec} | {elapsed} {bar} {total} ");

        let l = inner.width as usize;
        let l_len = left.len();
        let c_len = center.len();
        let r_len = right.len();

        let c_start = (l.saturating_sub(c_len)) / 2;
        let r_start = l.saturating_sub(r_len);

        let mut spans = vec![Span::styled(&left, Style::default().fg(Color::DarkGray))];
        if c_start > l_len {
            spans.push(Span::raw(" ".repeat(c_start - l_len)));
        }
        spans.push(Span::styled(&center, Style::default().fg(Color::Cyan)));
        if r_start > c_start + c_len {
            spans.push(Span::raw(" ".repeat(r_start - c_start - c_len)));
        }
        spans.push(Span::styled(&right, Style::default().fg(Color::DarkGray)));

        f.render_widget(Paragraph::new(Line::from(spans)), inner);
    }

    fn render_help_bar(&self, f: &mut Frame, area: Rect) {
        let msg = match self.input_mode {
            InputMode::Browse => " [↑↓] navigate  [Enter] open folder  [s] scan this folder  [Esc] go up / cancel",
            InputMode::RemovePath => " [↑↓] select  [Enter] remove from library  [Esc] cancel",
            InputMode::Search => " [Esc] cancel search  [↑↓] results  [Enter] play  type to search",
            InputMode::None => match self.focus {
                Focus::Albums => " [/] add folder  [F] search  [D] remove album  [←→/Tab] switch  [↑↓] browse  [Enter] select  [k/Space] pause  [Q] quit",
                Focus::Tracks => " [/] add folder  [F] search  [D] remove album  [←→/Tab] switch  [↑↓] tracks  [Enter] play  [j]←5s [l]→5s  [k/Space] pause  [N]next  [B]prev  [+/-]vol  [M]mute  [Q]quit",
            },
        };
        let bar = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(bar, area);
    }

    fn handle_events(&mut self) -> Result<()> {
        if !event::poll(Duration::from_millis(100))? {
            return Ok(());
        }

        let event = event::read()?;
        match event {
            Event::Mouse(m) => {
                if m.kind == MouseEventKind::Down(MouseButton::Left) {
                    let now = Instant::now();
                    let is_double = now.duration_since(self.last_click_time) < Duration::from_millis(400)
                        && m.column == self.last_click_col
                        && m.row == self.last_click_row;
                    self.last_click_time = now;
                    self.last_click_col = m.column;
                    self.last_click_row = m.row;

                    let (term_w, term_h) = size().unwrap_or((80, 24));
                    let main_h = term_h.saturating_sub(5);
                    let left_w = term_w * 38 / 100;

                    let col = m.column;
                    let row = m.row;

                    if row >= 1 && row < 1 + main_h {
                        if col >= left_w {
                            // Right panel → album list
                            self.focus = Focus::Albums;
                            let inner_top = 2;
                            if row >= inner_top {
                                let visible = (main_h - 2) as usize;
                                let scroll = if self.selected_album >= visible.saturating_add(1) {
                                    self.selected_album - visible + 1
                                } else {
                                    0
                                };
                                let idx = scroll + (row - inner_top) as usize;
                                if idx < self.album_names.len() {
                                    self.selected_album = idx;
                                    self.load_album_tracks();
                                }
                            }
                        } else {
                            // Left panel → track list
                            self.focus = Focus::Tracks;
                            let meta_h = 10u16;
                            let track_top = 1 + meta_h;
                            let inner_top = track_top + 1;
                            if row >= inner_top {
                                let track_area = (main_h - meta_h - 2) as usize;
                                let scroll = if self.selected_track >= track_area.saturating_add(1) {
                                    self.selected_track - track_area + 1
                                } else {
                                    0
                                };
                                let idx = scroll + (row - inner_top) as usize;
                                if idx < self.album_tracks.len() {
                                    self.selected_track = idx;
                                }
                            }
                        }
                    }

                    if is_double {
                        self.play_selected_track();
                    }
                }
                return Ok(());
            }
            Event::Key(key) => {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
                self.exit = true;
                return Ok(());
            }

            if key.code == KeyCode::Char('r') && key.modifiers == KeyModifiers::CONTROL {
                self.rescan_paths();
                return Ok(());
            }

            self.status_msg.clear();

            // Handle search input mode
            if matches!(self.input_mode, InputMode::Search) {
                match key.code {
                    KeyCode::Esc => {
                        self.input_mode = InputMode::None;
                        self.search_query.clear();
                        self.search_results.clear();
                        self.load_album_tracks();
                    }
                    KeyCode::Enter => {
                        self.play_selected_track();
                    }
                    KeyCode::Backspace => {
                        self.search_query.pop();
                        self.run_search();
                    }
                    KeyCode::Char(c) => {
                        self.search_query.push(c);
                        self.run_search();
                    }
                    KeyCode::Up => {
                        if self.selected_track > 0 {
                            self.selected_track -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.selected_track + 1 < self.search_results.len() {
                            self.selected_track += 1;
                        }
                    }
                    _ => {}
                }
                return Ok(());
            }

            // Handle browse mode
            if matches!(self.input_mode, InputMode::Browse) {
                match key.code {
                    KeyCode::Esc => {
                        if self.browser_cwd.parent().is_some() {
                            self.browser_cwd.pop();
                            self.load_browser();
                        } else {
                            self.input_mode = InputMode::None;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some((name, true)) = self.browser_entries.get(self.browser_selection) {
                            if name == ".." {
                                self.browser_cwd.pop();
                            } else {
                                self.browser_cwd.push(name);
                            }
                            self.load_browser();
                        }
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        let path = self.browser_cwd.clone();
                        self.input_mode = InputMode::None;
                        self.status_msg = format!(" Scanning {}...", path.display());
                        if let Err(e) = self.scan_path(&path) {
                            self.status_msg = format!(" Error: {e}");
                        }
                    }
                    KeyCode::Up => {
                        if self.browser_selection > 0 {
                            self.browser_selection -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.browser_selection + 1 < self.browser_entries.len() {
                            self.browser_selection += 1;
                        }
                    }
                    _ => {}
                }
                return Ok(());
            }

            // Handle remove path input mode
            if matches!(self.input_mode, InputMode::RemovePath) {
                match key.code {
                    KeyCode::Esc => {
                        self.input_mode = InputMode::None;
                    }
                    KeyCode::Up => {
                        if self.remove_path_selection > 0 {
                            self.remove_path_selection -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.remove_path_selection + 1 < self.remove_paths.len() {
                            self.remove_path_selection += 1;
                        }
                    }
                    KeyCode::Enter => {
                        let sel = self.remove_path_selection;
                        self.input_mode = InputMode::None;
                        if let Some(p) = self.remove_paths.get(sel) {
                            let prefix = std::path::Path::new(p).to_string_lossy().to_string();
                            let n = self.library.remove_by_prefix(&prefix).unwrap_or(0);
                            self.refresh_library();
                            self.config.music_dirs.retain(|d| d.to_string_lossy() != prefix);
                            self.config.save().ok();
                            self.remove_paths.retain(|x| *x != prefix);
                            self.status_msg = format!(" Removed {n} tracks");
                            if self.remove_path_selection >= self.remove_paths.len() {
                                self.remove_path_selection = self.remove_paths.len().saturating_sub(1);
                            }
                        }
                    }
                    _ => {}
                }
                return Ok(());
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => self.exit = true,

                KeyCode::Char('/') => {
                    self.input_mode = InputMode::Browse;
                    self.browser_cwd = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
                    self.load_browser();
                    self.status_msg.clear();
                }

                KeyCode::Char('x') | KeyCode::Char('X') => {
                    self.input_mode = InputMode::RemovePath;
                    self.remove_paths = self.config.music_dirs.iter().map(|p| p.to_string_lossy().to_string()).collect();
                    self.remove_path_selection = 0;
                    self.status_msg.clear();
                }

                KeyCode::Char('f') | KeyCode::Char('F') => {
                    self.input_mode = InputMode::Search;
                    self.search_query.clear();
                    self.search_results = self.all_tracks.clone();
                    self.selected_track = 0;
                    self.focus = Focus::Tracks;
                }

                KeyCode::Tab => {
                    self.focus = match self.focus {
                        Focus::Albums => Focus::Tracks,
                        Focus::Tracks => Focus::Albums,
                    };
                }

                KeyCode::Up => match self.focus {
                    Focus::Albums => {
                        if self.selected_album > 0 {
                            self.selected_album -= 1;
                        }
                    }
                    Focus::Tracks => {
                        if self.selected_track > 0 {
                            self.selected_track -= 1;
                        }
                    }
                },

                KeyCode::Down => match self.focus {
                    Focus::Albums => {
                        if self.selected_album + 1 < self.album_names.len() {
                            self.selected_album += 1;
                        }
                    }
                    Focus::Tracks => {
                        if self.selected_track + 1 < self.album_tracks.len() {
                            self.selected_track += 1;
                        }
                    }
                },

                KeyCode::Left | KeyCode::Right => {
                    self.focus = match self.focus {
                        Focus::Albums => Focus::Tracks,
                        Focus::Tracks => Focus::Albums,
                    };
                }

                KeyCode::Enter => match self.focus {
                    Focus::Albums => {
                        self.load_album_tracks();
                        self.focus = Focus::Tracks;
                    }
                    Focus::Tracks => {
                        self.play_selected_track();
                    }
                },

                KeyCode::Char(' ') | KeyCode::Char('k') | KeyCode::Char('K') => {
                    if self.player.is_paused() {
                        self.player.resume();
                    } else {
                        self.player.pause();
                    }
                }

                KeyCode::Char('n') | KeyCode::Char('N') => self.next_track(),

                KeyCode::Char('b') | KeyCode::Char('B') => self.previous_track(),

                KeyCode::Char('p') | KeyCode::Char('P') => {
                    self.player.seek(0.0);
                }

                KeyCode::Char('=') | KeyCode::Char('+') => {
                    let v = (self.player.get_volume() + 0.05).min(1.0);
                    self.player.set_volume(v);
                }
                KeyCode::Char('-') | KeyCode::Char('_') => {
                    let v = (self.player.get_volume() - 0.05).max(0.0);
                    self.player.set_volume(v);
                }

                KeyCode::Char('m') | KeyCode::Char('M') => {
                    if self.player.is_muted() {
                        self.player.unmute();
                    } else {
                        self.player.mute();
                    }
                }

                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.config.repeat = !self.config.repeat;
                    self.config.save().ok();
                }

                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.config.shuffle = !self.config.shuffle;
                    self.config.save().ok();
                }

                KeyCode::Char('D') => {
                    if let Some(album) = self.album_names.get(self.selected_album) {
                        let paths: Vec<String> = self.all_tracks.iter()
                            .filter(|t| t.album == *album)
                            .map(|t| t.path.clone())
                            .collect();
                        if let Some(first) = paths.first() {
                            if let Some(parent) = std::path::Path::new(first).parent() {
                                let prefix = parent.to_string_lossy().to_string();
                                let n = self.library.remove_by_prefix(&prefix).unwrap_or(0);
                                self.status_msg = format!(" Removed {n} tracks");
                                self.refresh_library();
                            }
                        }
                    }
                }

                _ => {}
            }

            if matches!(self.focus, Focus::Albums) {
                self.load_album_tracks();
            }
        }
            _ => {}
        }

        Ok(())
    }

    fn scan_path(&mut self, path: &std::path::Path) -> Result<()> {
        let (n, errs) = self.library.scan(path)?;
        let path_str = path.to_string_lossy().to_string();
        if !self.config.music_dirs.iter().any(|d| d.to_string_lossy() == path_str) {
            self.config.music_dirs.push(path.to_path_buf());
            self.config.save().ok();
        }
        let mut msg = format!(" Added {n} tracks from {}", path.display());
        if errs > 0 {
            msg.push_str(&format!(" ({errs} skipped)"));
        }
        self.status_msg = msg;
        self.refresh_library();
        Ok(())
    }

    fn rescan_paths(&mut self) {
        let mut total_removed = 0;
        let mut total_added = 0;
        let mut total_errors = 0;
        for dir in &self.config.music_dirs {
            if !dir.exists() {
                continue;
            }
            let prefix = dir.to_string_lossy().to_string();
            total_removed += self.library.remove_by_prefix(&prefix).unwrap_or(0);
            match self.library.scan(dir) {
                Ok((added, errors)) => {
                    total_added += added;
                    total_errors += errors;
                }
                Err(_) => {}
            }
        }
        let mut msg = format!(
            " Rescanned — {total_removed} tracks refreshed, {total_added} new"
        );
        if total_errors > 0 {
            msg.push_str(&format!(" ({total_errors} skipped)"));
        }
        self.status_msg = msg;
        self.refresh_library();
    }

    fn play_selected_track(&mut self) {
        let in_search = matches!(self.input_mode, InputMode::Search);
        let tracks = if in_search { &self.search_results } else { &self.album_tracks };
        let idx = if in_search { self.selected_track.min(tracks.len().saturating_sub(1)) } else { self.selected_track };

        if idx >= tracks.len() {
            return;
        }
        let track = &tracks[idx];
        let pref = std::path::Path::new(&track.path);
        if !pref.exists() {
            return;
        }
        self.current_path = track.path.clone();
        self.current_meta = Some(track.clone());
        match self.player.play(pref, Some(track.duration)) {
            Ok(()) => {}
            Err(e) => {
                self.status_msg = format!(" Playback error: {e}");
            }
        }
    }

    fn next_track(&mut self) {
        self.player.stop();
        if self.album_tracks.is_empty() {
            return;
        }
        let next = if self.config.shuffle {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::time::Instant::now().hash(&mut hasher);
            (hasher.finish() as usize) % self.album_tracks.len()
        } else if self.selected_track + 1 < self.album_tracks.len() {
            self.selected_track + 1
        } else if self.config.repeat {
            0
        } else {
            return;
        };
        self.selected_track = next;
        self.play_selected_track();
    }

    fn previous_track(&mut self) {
        if self.album_tracks.is_empty() {
            return;
        }
        self.player.stop();
        if self.selected_track > 0 {
            self.selected_track -= 1;
        } else {
            self.selected_track = self.album_tracks.len() - 1;
        }
        self.play_selected_track();
    }

    fn run_search(&mut self) {
        let q = self.search_query.to_lowercase();
        self.search_results = if q.is_empty() {
            self.all_tracks.clone()
        } else {
            self.all_tracks
                .iter()
                .filter(|t| {
                    t.title.to_lowercase().contains(&q)
                        || t.artist.to_lowercase().contains(&q)
                        || t.album.to_lowercase().contains(&q)
                })
                .cloned()
                .collect()
        };
        self.selected_track = 0;
    }
}

fn truncate_path(p: &str, max_w: usize) -> String {
    if max_w < 3 || p.len() <= max_w {
        return p.to_string();
    }
    let keep = max_w.saturating_sub(2);
    format!("..{}", &p[p.len().saturating_sub(keep)..])
}
