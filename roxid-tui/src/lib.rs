mod app;
mod events;
mod ui;

use app::App;
use color_eyre::Result;

pub async fn run() -> Result<()> {
    let terminal = ratatui::init();
    let result = App::new().await?.run(terminal).await;
    ratatui::restore();
    result
}
