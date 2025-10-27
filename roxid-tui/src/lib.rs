mod app;
mod events;
mod ui;

use app::App;
use color_eyre::Result;

pub async fn run() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new().run(terminal).await;
    ratatui::restore();
    result
}
