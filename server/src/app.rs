use std::io::stdout;
use std::sync::{Arc, Mutex};

use ratatui::Terminal;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::EnterAlternateScreen;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::CrosstermBackend;
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
}

impl App {
    pub fn new() -> Self {
        Self {
            recipients: Arc::new(Mutex::new(Vec::new())),
            selected: 0,
            input: String::new(),
        }
    }

    pub fn add_recipient(&self, r: Recipient) {
        self.recipients.lock().unwrap().push(r);
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        loop {
            terminal.draw(|f| self.draw(f))?;
        }
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
    }
}
