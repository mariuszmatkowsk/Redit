use crossterm::{
    cursor::{position, MoveTo},
    event::{poll, read, Event, KeyCode},
    queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, Stylize},
    terminal::size,
};
use std::cmp;
use std::error::Error;
use std::io::Write;
use std::io::{self};
use std::time::Duration;

mod screen_state;
use screen_state::ScreenState;

#[derive(Clone, PartialEq, Eq)]
enum EditorMode {
    Insert,
    Visual,
    Command,
}

struct Cursor {
    x: u16,
    y: u16,
}

impl Cursor {
    fn new() -> Self {
        Self { x: 0, y: 0 }
    }

    fn move_left(&mut self) {
        self.x = self.x.saturating_sub(1);
    }

    fn move_right(&mut self) {
        self.x += 1;
    }

    fn move_down(&mut self) {
        self.y += 1;
    }

    fn move_up(&mut self) {
        self.y = self.y.saturating_sub(1);
    }

    fn get_position(&self) -> (u16, u16) {
        (self.x, self.y)
    }
}

enum Action {
    EnterInsertMode,
    EnterVisualMode,
    EnterCommandMode,
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorDown,
    MoveCursorUp,
    Quit,
    Unknown,
    EnterCommandChar(char),
    ExecuteCommand,
}

struct Editor {
    columns: u16,
    rows: u16,
    cursor: Cursor,
    mode: EditorMode,
    quit: bool,
    lines: Vec<String>,
    command: String,
}

impl Editor {
    fn new(columns: u16, rows: u16) -> Self {
        Self {
            columns,
            rows,
            cursor: Cursor::new(),
            mode: EditorMode::Visual,
            quit: false,
            lines: Vec::new(),
            command: String::new(),
        }
    }

    fn status_line(&self, writer: &mut impl std::io::Write) -> io::Result<()> {
        queue!(writer, SetBackgroundColor(Color::DarkGrey))?;
        queue!(writer, SetForegroundColor(Color::White))?;
        queue!(writer, MoveTo(0, self.rows - 2))?;
        // if self.mode == EditorMode::Insert {
        //     queue!(writer, Print("-- INSERT --".bold()))?;
        // }

        let (pos_x, _) = position()?;
        let n: usize = self.columns as usize - pos_x as usize;

        queue!(writer, SetBackgroundColor(Color::DarkGrey))?;
        queue!(writer, SetForegroundColor(Color::White))?;
        queue!(writer, Print(" ".repeat(n)))?;

        queue!(writer, SetBackgroundColor(Color::DarkGrey))?;
        queue!(writer, SetForegroundColor(Color::White))?;
        queue!(writer, MoveTo(self.columns - 20, self.rows - 2))?;
        let (pos_x, pos_y) = self.cursor.get_position();
        queue!(
            writer,
            Print(format!(
                "{line},{column}",
                line = pos_y + 1,
                column = pos_x + 1
            ))
        )?;

        queue!(writer, ResetColor)?;

        Ok(())
    }

    fn command_line(&self, writer: &mut impl std::io::Write) -> io::Result<()> {
        queue!(writer, MoveTo(0, self.rows - 1))?;
        if self.mode == EditorMode::Insert {
            queue!(writer, Print("-- INSERT --".bold()))?;
        }

        if self.mode == EditorMode::Command {
            queue!(writer, Print(format!(":{}", self.command)))?;
        }

        if self.mode == EditorMode::Visual {
            queue!(writer, Print(" ".repeat(self.rows.into())))?;
        }

        Ok(())
    }

    fn execute_command(&mut self) {
        match self.command.as_str() {
            "q" => self.quit = true,
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let _screen = ScreenState::init();

    let mut stdout = io::stdout();

    let (columns, rows) = size()?;
    let mut editor = Editor::new(columns, rows);

    while !editor.quit {
        editor.status_line(&mut stdout).map_err(|e| {
            eprintln!("Could not generate status line: {}", e);
            e
        })?;
        editor.command_line(&mut stdout).map_err(|e| {
            eprintln!("Could not generate command line: {}", e);
            e
        })?;
        if let Ok(true) = poll(Duration::ZERO) {
            if let Ok(event) = read() {
                let action = match editor.mode {
                    EditorMode::Insert => handle_insert_mode_event(&event),
                    EditorMode::Visual => handle_visual_mode_event(&event),
                    EditorMode::Command => handle_command_mode_event(&event),
                };

                match action {
                    Action::Quit => editor.quit = true,
                    Action::EnterInsertMode => editor.mode = EditorMode::Insert,
                    Action::EnterVisualMode => {
                        if editor.mode == EditorMode::Command {
                            editor.command.clear();
                        }
                        editor.mode = EditorMode::Visual;
                    }
                    Action::EnterCommandMode => editor.mode = EditorMode::Command,
                    Action::MoveCursorLeft => editor.cursor.move_left(),
                    Action::MoveCursorRight => editor.cursor.move_right(),
                    Action::MoveCursorDown => editor.cursor.move_down(),
                    Action::MoveCursorUp => editor.cursor.move_up(),
                    Action::EnterCommandChar(c) => editor.command.push(c),
                    Action::ExecuteCommand => editor.execute_command(),
                    _ => {}
                }
            }
        }
        stdout.flush().unwrap();
    }

    Ok(())
}

fn handle_insert_mode_event(event: &Event) -> Action {
    match event {
        Event::Key(key_event) => match key_event.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Esc => Action::EnterVisualMode,
            _ => Action::Unknown,
        },
        _ => Action::Unknown,
    }
}

fn handle_visual_mode_event(event: &Event) -> Action {
    match event {
        Event::Key(key_event) => match key_event.code {
            KeyCode::Char('i') => Action::EnterInsertMode,
            KeyCode::Char(':') => Action::EnterCommandMode,
            KeyCode::Char('h') => Action::MoveCursorLeft,
            KeyCode::Char('l') => Action::MoveCursorRight,
            KeyCode::Char('j') => Action::MoveCursorDown,
            KeyCode::Char('k') => Action::MoveCursorUp,
            _ => Action::Unknown,
        },
        _ => Action::Unknown,
    }
}

fn handle_command_mode_event(event: &Event) -> Action {
    match event {
        Event::Key(key_event) => match key_event.code {
            KeyCode::Esc => Action::EnterVisualMode,
            KeyCode::Enter => Action::ExecuteCommand,
            KeyCode::Char(c) => Action::EnterCommandChar(c),
            _ => Action::Unknown,
        },
        _ => Action::Unknown,
    }
}
