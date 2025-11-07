use std::io::stdout;
use std::sync::{Arc, Mutex};

use ratatui::Terminal;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::EnterAlternateScreen;
use ratatui::prelude::CrosstermBackend;

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
        
    }
}