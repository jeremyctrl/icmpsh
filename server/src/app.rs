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
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

pub struct Recipient {
    pub label: String,
    pub blocked: bool,
    pub queued: String,
    pub history: Vec<String>,
}

impl Recipient {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            blocked: false,
            queued: String::new(),
            history: Vec::new(),
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
        let recipients = self.recipients.lock().unwrap();
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

        if let Some(rec) = recipients.get(selected) {
            let chat_text: Vec<Line> = rec
                .history
                .iter()
                .map(|m| Line::from(Span::raw(m.clone())))
                .collect();

            let chat_box = Paragraph::new(chat_text).block(
                Block::default()
                    .title(rec.label.clone())
                    .borders(Borders::ALL),
            );
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
}
