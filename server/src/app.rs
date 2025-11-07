use std::io::stdout;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ratatui::Terminal;
use ratatui::crossterm::event::{Event, KeyCode};
use ratatui::crossterm::{event, execute};
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
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
    selected: usize,
    input: String,
    last_tick: Instant,
}

impl App {
    pub fn new() -> Self {
        Self {
            recipients: Arc::new(Mutex::new(Vec::new())),
            selected: 0,
            input: String::new(),
            last_tick: Instant::now(),
        }
    }

    pub fn add_recipient(&self, r: Recipient) {
        self.recipients.lock().unwrap().push(r);
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let tick_rate = Duration::from_millis(100);

        loop {
            terminal.draw(|f| self.draw(f))?;

            let timeout = tick_rate
                .checked_sub(self.last_tick.elapsed())
                .unwrap_or(Duration::from_secs(0));

            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => break,
                        KeyCode::Char(c) => self.handle_char(c),
                        KeyCode::Backspace => {
                            self.input.pop();
                        }
                        KeyCode::Enter => self.submit_message(),
                        _ => {},
                    }
                }
            }

            if self.last_tick.elapsed() >= tick_rate {
                self.last_tick = Instant::now();
            }
        }

        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        Ok(())
    }

    fn draw(&mut self, f: &mut ratatui::Frame) {
        let recipients = self.recipients.lock().unwrap();

        let size = f.area();
        let block = Block::default().title("icmpsh").borders(Borders::ALL);
        f.render_widget(block, size);

        if recipients.is_empty() {
            let msg = Paragraph::new("(waiting for connections...)").alignment(Alignment::Center);
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
                let label = if i == self.selected {
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

        if let Some(rec) = recipients.get(self.selected) {
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
                self.input.clone()
            };

            let input_box = Paragraph::new(placeholder)
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input_box, chat_chunks[1]);
        }
    }

    fn handle_char(&mut self, c: char) {
        let recipients = self.recipients.lock().unwrap();
        if let Some(r) = recipients.get(self.selected) {
            if !r.blocked {
                drop(recipients);
                self.input.push(c);
            }
        }
    }

    fn submit_message(&mut self) {
        let mut recipients = self.recipients.lock().unwrap();
        if let Some(r) = recipients.get_mut(self.selected) {
            if !r.blocked && !self.input.is_empty() {
                r.queued = self.input.clone();
                r.history.push(format!("> {}", self.input));
                r.blocked = true;
                self.input.clear();
            }
        }
    }
}
