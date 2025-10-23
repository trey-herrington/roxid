use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    DefaultTerminal, Frame,
};

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal);
    ratatui::restore();
    result
}

#[derive(Debug, Default)]
struct App {
    counter: i32,
    items: Vec<String>,
    should_quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            counter: 0,
            items: vec![
                "Welcome to your Rust TUI app!".to_string(),
                "Press 'j' to increment counter".to_string(),
                "Press 'k' to decrement counter".to_string(),
                "Press 'a' to add item".to_string(),
                "Press 'q' to quit".to_string(),
            ],
            should_quit: false,
        }
    }

    fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(frame.area());

        self.render_header(frame, chunks[0]);
        self.render_main(frame, chunks[1]);
        self.render_footer(frame, chunks[2]);
    }

    fn render_header(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let title = Paragraph::new("Rust TUI Application")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("Header"));
        frame.render_widget(title, area);
    }

    fn render_main(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        let counter_text = format!("Counter: {}", self.counter);
        let counter = Paragraph::new(counter_text)
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Counter")
                    .border_style(Style::default().fg(Color::White)),
            )
            .wrap(Wrap { trim: true });
        frame.render_widget(counter, main_chunks[0]);

        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let content = Line::from(Span::styled(
                    format!("{}. {}", i + 1, item),
                    Style::default().fg(Color::Green),
                ));
                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Items"))
            .style(Style::default().fg(Color::White));
        frame.render_widget(list, main_chunks[1]);
    }

    fn render_footer(&self, frame: &mut Frame, area: ratatui::layout::Rect) {
        let info = Paragraph::new("Press 'q' to quit | 'j'/'k' to change counter | 'a' to add item")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL).title("Help"));
        frame.render_widget(info, area);
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') => self.counter += 1,
            KeyCode::Char('k') => self.counter -= 1,
            KeyCode::Char('a') => {
                let item = format!("New item {}", self.items.len() - 4);
                self.items.push(item);
            }
            _ => {}
        }
    }
}
