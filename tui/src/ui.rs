mod components;
mod layout;

pub use components::{render_counter, render_footer, render_header, render_items};

use ratatui::Frame;

use crate::app::App;

pub fn render(app: &App, frame: &mut Frame) {
    let chunks = layout::create_layout(frame.area());

    render_header(frame, chunks[0]);
    render_main(app, frame, chunks[1]);
    render_footer(frame, chunks[2]);
}

fn render_main(app: &App, frame: &mut Frame, area: ratatui::layout::Rect) {
    let main_chunks = layout::create_horizontal_layout(area);
    render_counter(app.counter, frame, main_chunks[0]);
    render_items(&app.items, frame, main_chunks[1]);
}
