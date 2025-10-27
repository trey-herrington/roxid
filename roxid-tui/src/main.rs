use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    roxid_tui::run().await
}
