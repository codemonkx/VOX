use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
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
    Folder,
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
    folder_input: String,
    remove_paths: Vec<String>,
    remove_path_selection: usize,
    search_query: String,
    search_results: Vec<Track>,
    status_msg: String,
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

        let mut app = Self {
            library,
            player,
            config,
            current_meta: None,
            all_tracks,
            album_names,
            album_tracks: Vec::new(),
            selected_album: 0,
            prev_album: 0,
            selected_track: 0,
            focus: Focus::Albums,
            current_path: String::new(),
            track_ended: false,
            exit: false,
            input_mode: InputMode::None,
            folder_input: String::new(),
            remove_paths,
            remove_path_selection: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            status_msg: String::new(),
        };
        app.load_album_tracks();
        Ok(app)
    }

    fn load_album_tracks(&mut self) {
        if self.selected_album == self.prev_album {
            return;
        }
        self.prev_album = self.selected_album;
        if let Some(album) = self.album_names.get(self.selected_album) {
            self.album_tracks = self
                .all_tracks
                .iter()
                .filter(|t| t.album == *album)
                .cloned()
                .collect();
            self.selected_track = 0;
        }
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
        self.prev_album = usize::MAX;
        self.load_album_tracks();
        if self.selected_album >= self.album_names.len() {
            self.selected_album = self.album_names.len().saturating_sub(1);
        }
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
            InputMode::Folder => {
                format!(" Add folder path: {}", self.folder_input)
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
            InputMode::Folder => Style::default().fg(Color::Black).bg(Color::Cyan),
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
                ListItem::new(format!("{prefix}{p}")).style(style)
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
            InputMode::Folder => " [Enter] confirm  [Esc] cancel  type/paste folder path",
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
            Event::Mouse(_m) => {
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

            // Handle folder input mode
            if matches!(self.input_mode, InputMode::Folder) {
                match key.code {
                    KeyCode::Esc => {
                        self.input_mode = InputMode::None;
                        self.folder_input.clear();
                    }
                    KeyCode::Enter => {
                        let path = self.folder_input.trim().to_string();
                        self.folder_input.clear();
                        self.input_mode = InputMode::None;
                        if !path.is_empty() {
                            let p = std::path::Path::new(&path);
                            if p.exists() {
                                self.status_msg = format!(" Scanning {path}...");
                                match self.library.scan(p) {
                                    Ok(n) => {
                                        let path_str = path.clone();
                                        if !self.config.music_dirs.iter().any(|d| d.to_string_lossy() == path_str) {
                                            self.config.music_dirs.push(p.to_path_buf());
                                            self.config.save().ok();
                                        }
                                        self.status_msg = format!(" Added {n} tracks from {path}");
                                        self.refresh_library();
                                    }
                                    Err(e) => {
                                        self.status_msg = format!(" Error: {e}");
                                    }
                                }
                            } else {
                                self.status_msg = format!(" Path not found: {path}");
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        self.folder_input.pop();
                    }
                    KeyCode::Char(c) => {
                        self.folder_input.push(c);
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
                        let paths = self.remove_paths.clone();
                        self.input_mode = InputMode::None;
                        if let Some(p) = paths.get(self.remove_path_selection) {
                            let pp = std::path::Path::new(p);
                            let prefix = pp.to_string_lossy().to_string();
                            let n = self.library.remove_by_prefix(&prefix).unwrap_or(0);
                            self.status_msg = format!(" Removed {n} tracks from library");
                            if n > 0 {
                                let _ = self.config.music_dirs.retain(|d| d.to_string_lossy() != prefix);
                                let _ = self.config.save();
                                self.refresh_library();
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
                    self.input_mode = InputMode::Folder;
                    self.folder_input.clear();
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
        self.player.play(pref, Some(track.duration)).ok();
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
