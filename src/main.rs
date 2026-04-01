mod airgapped;
mod app;
mod ui;
mod utils;

use app::App;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Check if running as airgapped binary and setup if needed
    let is_airgapped = airgapped::is_airgapped_binary()?;
    if is_airgapped {
        airgapped::setup().await?;
        println!(
            "Installer running in offline mode (images from embedded payload only, no pull from internet)."
        );
    }

    let terminal = ratatui::init();
    let result = App::new(is_airgapped).run(terminal).await;
    ratatui::restore();
    result
}
