use color_eyre::Result;

mod tui;

fn main() -> Result<()> {
    color_eyre::install()?;
    ratatui::run(|terminal| tui::App::new().run(terminal))
}
