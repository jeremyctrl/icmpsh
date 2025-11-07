use std::io::stdout;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::crossterm::event::{Event, KeyCode};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::crossterm::{event, execute};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::CrosstermBackend;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

pub struct Recipient {
    pub label: String,
    pub blocked: bool,
    pub queued: String,
    pub history: Vec<String>,
    pub scroll: u16,
    pub auto_scroll: bool,
}

impl Recipient {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            blocked: false,
            queued: String::new(),
            history: Vec::new(),
            scroll: 0,
            auto_scroll: true,
        }
    }

    pub fn add_message(&mut self, msg: &str) {
        self.history.push(format!("{}: {}", self.label, msg));
    }
}

pub struct App {
    pub recipients: Arc<Mutex<Vec<Recipient>>>,
    selected: Mutex<usize>,
    input: Mutex<String>,
    last_tick: Mutex<Instant>,
}

impl App {
    pub fn new() -> Self {
        Self {
            recipients: Arc::new(Mutex::new(Vec::new())),
            selected: Mutex::new(0),
            input: Mutex::new(String::new()),
            last_tick: Mutex::new(Instant::now()),
        }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        enable_raw_mode()?;

        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let tick_rate = Duration::from_millis(100);

        loop {
            terminal.draw(|f| self.draw(f))?;

            let last_tick = self.last_tick.lock().unwrap();
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or(Duration::from_secs(0));
            drop(last_tick);

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => break,
                        KeyCode::Char(c) => self.handle_char(c),
                        KeyCode::Backspace => {
                            self.input.lock().unwrap().pop();
                        }
                        KeyCode::Enter => self.submit_message(),
                        KeyCode::Up => self.navigate_up(),
                        KeyCode::Down => self.navigate_down(),
                        KeyCode::PageUp => self.scroll_up(),
                        KeyCode::PageDown => self.scroll_down(),

                        _ => {}
                    }
                }
            }

            let mut last_tick = self.last_tick.lock().unwrap();
            if last_tick.elapsed() >= tick_rate {
                *last_tick = Instant::now();
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        Ok(())
    }

    fn draw(&self, f: &mut ratatui::Frame) {
        let mut recipients = self.recipients.lock().unwrap();
        let selected = *self.selected.lock().unwrap();
        let input = self.input.lock().unwrap();

        let size = f.area();
        let block = Block::default().title("icmpsh").borders(Borders::ALL);
        f.render_widget(block, size);

        if recipients.is_empty() {
            let msg =
                Paragraph::new("(...waiting for connections...)").alignment(Alignment::Center);
            f.render_widget(msg, size);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(size);

        let list_items: Vec<ListItem> = recipients
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let label = if i == selected {
                    format!("> {}", r.label)
                } else {
                    r.label.clone()
                };
                ListItem::new(label)
            })
            .collect();

        let recipient_list =
            List::new(list_items).block(Block::default().title("Recipients").borders(Borders::ALL));
        f.render_widget(recipient_list, chunks[0]);

        let chat_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(chunks[1]);

        if let Some(rec) = recipients.get_mut(selected) {
            let chat_text: Vec<Line> = rec
                .history
                .iter()
                .map(|m| Line::from(Span::raw(m.clone())))
                .collect();

            let chat_area = chat_chunks[0];
            let inner_height = chat_area.height.saturating_sub(2);
            let inner_width = chat_area.width.saturating_sub(2);

            if inner_height == 0 || inner_width == 0 {
                let chat_box = Paragraph::new(chat_text)
                    .block(
                        Block::default()
                            .title(rec.label.clone())
                            .borders(Borders::ALL),
                    )
                    .wrap(Wrap { trim: false });
                f.render_widget(chat_box, chat_chunks[0]);

                let placeholder = if rec.blocked {
                    "waiting for response...".to_string()
                } else {
                    input.clone()
                };

                let input_box = Paragraph::new(placeholder)
                    .block(Block::default().borders(Borders::ALL).title("Input"));
                f.render_widget(input_box, chat_chunks[1]);

                return;
            }

            let text_height = self.measure_text_height(&chat_text, inner_width);

            if text_height <= inner_height {
                rec.scroll = 0;
                rec.auto_scroll = true;
            } else {
                let max_scroll = text_height - inner_height;

                if rec.auto_scroll {
                    rec.scroll = max_scroll;
                } else {
                    if rec.scroll > max_scroll {
                        rec.scroll = max_scroll;
                    }
                }

                if rec.scroll == max_scroll {
                    rec.auto_scroll = true;
                }
            }

            let chat_box = Paragraph::new(chat_text)
                .block(
                    Block::default()
                        .title(rec.label.clone())
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false })
                .scroll((rec.scroll, 0));
            f.render_widget(chat_box, chat_chunks[0]);

            let placeholder = if rec.blocked {
                "waiting for response...".to_string()
            } else {
                input.clone()
            };

            let input_box = Paragraph::new(placeholder)
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input_box, chat_chunks[1]);
        }
    }

    fn measure_text_height(&self, lines: &Vec<Line>, inner_width: u16) -> u16 {
        if inner_width == 0 {
            return 0;
        }

        let mut total: u32 = 0;
        let width = inner_width as u32;

        for line in lines {
            let mut buf = String::new();
            for span in &line.spans {
                buf.push_str(span.content.as_ref());
            }

            let w = UnicodeWidthStr::width(buf.as_str()) as u32;

            let line_height = if w == 0 { 1 } else { (w + width - 1) / width };

            total = total.saturating_add(line_height);
            if total >= u16::MAX as u32 {
                return u16::MAX;
            }
        }

        total as u16
    }

    fn handle_char(&self, c: char) {
        let can_type = {
            let recipients = self.recipients.lock().unwrap();
            recipients
                .get(*self.selected.lock().unwrap())
                .map_or(false, |r| !r.blocked)
        };
        if can_type {
            self.input.lock().unwrap().push(c);
        }
    }

    fn submit_message(&self) {
        let mut recipients = self.recipients.lock().unwrap();
        let selected = *self.selected.lock().unwrap();
        let mut input = self.input.lock().unwrap();

        if let Some(r) = recipients.get_mut(selected) {
            if !r.blocked && !input.is_empty() {
                r.queued = input.clone();
                r.history.push(format!("> {}", input));
                r.blocked = true;

                r.auto_scroll = true;

                input.clear();
            }
        }
    }

    fn navigate_up(&self) {
        let mut selected = self.selected.lock().unwrap();
        if !self.recipients.lock().unwrap().is_empty() {
            *selected = selected.saturating_sub(1);
        }
    }

    fn navigate_down(&self) {
        let mut selected = self.selected.lock().unwrap();
        if *selected + 1 < self.recipients.lock().unwrap().len() {
            *selected += 1;
        }
    }

    fn scroll_up(&self) {
        let mut recipients = self.recipients.lock().unwrap();
        let selected = *self.selected.lock().unwrap();
        if let Some(rec) = recipients.get_mut(selected) {
            if rec.scroll > 0 {
                rec.scroll = rec.scroll.saturating_sub(1);
                rec.auto_scroll = false;
            }
        }
    }

    fn scroll_down(&self) {
        let mut recipients = self.recipients.lock().unwrap();
        let selected = *self.selected.lock().unwrap();
        if let Some(rec) = recipients.get_mut(selected) {
            rec.scroll = rec.scroll.saturating_add(1);
            rec.auto_scroll = false;
        }
    }
}
