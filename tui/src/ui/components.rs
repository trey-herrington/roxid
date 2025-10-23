use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn render_header(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new("Rust TUI Application")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title("Header"));
    frame.render_widget(title, area);
}

pub fn render_counter(counter: i32, frame: &mut Frame, area: Rect) {
    let counter_text = format!("Counter: {}", counter);
    let counter_widget = Paragraph::new(counter_text)
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Counter")
                .border_style(Style::default().fg(Color::White)),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(counter_widget, area);
}

pub fn render_items(items: &[String], frame: &mut Frame, area: Rect) {
    let list_items: Vec<ListItem> = items
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

    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).title("Items"))
        .style(Style::default().fg(Color::White));
    frame.render_widget(list, area);
}

pub fn render_footer(frame: &mut Frame, area: Rect) {
    let info = Paragraph::new("Press 'q' to quit | 'j'/'k' to change counter | 'a' to add item")
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL).title("Help"));
    frame.render_widget(info, area);
}
