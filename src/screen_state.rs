use crossterm::{
    cursor::{Hide, Show},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::io;

pub struct ScreenState {}

impl ScreenState {
    pub fn init() -> io::Result<Self> {
        execute!(io::stdout(), EnterAlternateScreen)?;
        enable_raw_mode()?;
        execute!(io::stdout(), Hide)?;
        Ok(Self {})
    }
}

impl Drop for ScreenState {
    fn drop(&mut self) {
        execute!(io::stdout(), LeaveAlternateScreen).expect("Should leave alternative screen");
        disable_raw_mode().expect("Should disable raw mode");
        execute!(io::stdout(), Show).expect("Should show cursor");
    }
}
