use crossterm::{
    cursor::{position, MoveTo},
    event::{poll, read, Event, KeyCode},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor, Stylize},
    terminal::{size, Clear, ClearType},
};
use std::error::Error;
use std::io::Write;
use std::io::{self};

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

    fn move_right(&mut self, boundary: u16) {
        self.x = std::cmp::min(boundary, self.x + 1);
    }

    fn move_down(&mut self, boundary: u16) {
        self.y = std::cmp::min(boundary, self.y + 1);
    }

    fn move_up(&mut self) {
        self.y = self.y.saturating_sub(1);
    }

    fn move_cursor_to_begin(&mut self) {
        self.x = 0;
        self.y = 0;
    }

    fn move_cursor_to_end(&mut self) {
        unimplemented!();
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
    EnterChar(char),
    NewLine,
    EnterInsertModeNext,
    AppendShortcutChar(char),
    ClearShortuctBuffer,
    BackspaceInInsertMode,
    EnterInsertModeInNewLine,
}

struct Editor {
    stdout: io::Stdout,
    columns: u16,
    rows: u16,
    cursor: Cursor,
    mode: EditorMode,
    quit: bool,
    lines: Vec<String>,
    command: String,
    shortcut_buffer: String,
}

impl Editor {
    fn new(columns: u16, rows: u16) -> Self {
        Self {
            stdout: io::stdout(),
            columns,
            rows,
            cursor: Cursor::new(),
            mode: EditorMode::Visual,
            quit: false,
            lines: vec![String::new()],
            command: String::new(),
            shortcut_buffer: String::new(),
        }
    }

    fn generate(&mut self) -> io::Result<()> {
        self.generate_editor_space()?;
        self.status_line()?;
        self.command_line()?;

        Ok(())
    }

    fn generate_editor_space(&mut self) -> io::Result<()> {
        let (cx, cy) = self.cursor.get_position();

        let mut is_cursor_drawed = false;

        for (row, line) in self.lines.iter().enumerate() {
            queue!(self.stdout, MoveTo(0, row as u16))?;
            for (col, c) in line.chars().enumerate() {
                if !is_cursor_drawed && (cx as usize, cy as usize) == (col, row) {
                    queue!(self.stdout, SetBackgroundColor(Color::Blue))?;
                    queue!(self.stdout, SetForegroundColor(Color::Black))?;
                    queue!(self.stdout, Print(c))?;
                    is_cursor_drawed = true;
                    queue!(self.stdout, ResetColor)?;
                } else {
                    queue!(self.stdout, Print(c))?;
                }
            }

            if row == cy as usize && !is_cursor_drawed {
                queue!(self.stdout, SetBackgroundColor(Color::Blue))?;
                queue!(self.stdout, Print(" "))?;
                is_cursor_drawed = true;
                queue!(self.stdout, ResetColor)?;
            }
        }

        Ok(())
    }

    fn status_line(&mut self) -> io::Result<()> {
        queue!(self.stdout, SetBackgroundColor(Color::DarkGrey))?;
        queue!(self.stdout, SetForegroundColor(Color::White))?;
        queue!(self.stdout, MoveTo(0, self.rows - 2))?;

        let (pos_x, _) = position()?;
        let n: usize = self.columns as usize - pos_x as usize;

        queue!(self.stdout, SetBackgroundColor(Color::DarkGrey))?;
        queue!(self.stdout, SetForegroundColor(Color::White))?;
        queue!(self.stdout, Print(" ".repeat(n)))?;

        queue!(self.stdout, SetBackgroundColor(Color::DarkGrey))?;
        queue!(self.stdout, SetForegroundColor(Color::White))?;
        queue!(self.stdout, MoveTo(self.columns - 20, self.rows - 2))?;
        let (pos_x, pos_y) = self.cursor.get_position();
        queue!(
            self.stdout,
            Print(format!(
                "{line},{column}",
                line = pos_y + 1,
                column = pos_x + 1
            ))
        )?;

        queue!(self.stdout, ResetColor)?;

        Ok(())
    }

    fn command_line(&mut self) -> io::Result<()> {
        queue!(self.stdout, MoveTo(0, self.rows - 1))?;
        if self.mode == EditorMode::Insert {
            queue!(self.stdout, Print("-- INSERT --".bold()))?;
        }

        if self.mode == EditorMode::Command {
            queue!(self.stdout, Print(format!(":{}", self.command)))?;
        }

        if self.mode == EditorMode::Visual {
            queue!(self.stdout, Print(" ".repeat(self.rows.into())))?;
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

    editor.generate().map_err(|e| {
        eprintln!("Something goes wrong during editor generation: {}", e);
        e
    })?;
    stdout.flush().unwrap();
    while !editor.quit {
        if let Ok(event) = read() {
            execute!(stdout, Clear(ClearType::All)).unwrap();
            let action = match editor.mode {
                EditorMode::Insert => handle_insert_mode_event(&event),
                EditorMode::Visual => handle_visual_mode_event(&event),
                EditorMode::Command => handle_command_mode_event(&event),
            };

            match action {
                Action::Quit => editor.quit = true,
                Action::EnterInsertMode => editor.mode = EditorMode::Insert,
                Action::EnterInsertModeNext => {
                    editor.mode = EditorMode::Insert;
                    editor.cursor.move_right(u16::MAX);
                }
                Action::EnterInsertModeInNewLine => {
                    let (_, row) = editor.cursor.get_position();
                    if row as usize == editor.lines.len() - 1 {
                        editor.lines.push(String::new());
                    } else {
                        editor.lines.insert(row as usize + 1, String::new());
                    }

                    editor.mode = EditorMode::Insert;
                    editor.cursor.move_down(u16::MAX);

                }
                Action::EnterVisualMode => {
                    if editor.mode == EditorMode::Command {
                        editor.command.clear();
                    }
                    editor.mode = EditorMode::Visual;
                }
                Action::EnterCommandMode => editor.mode = EditorMode::Command,
                Action::MoveCursorLeft => editor.cursor.move_left(),
                Action::MoveCursorRight => {
                    let (_, cy) = editor.cursor.get_position();
                    if let Some(line) = editor.lines.get(cy as usize) {
                        if line.len() == 0 {
                            editor.cursor.move_right(0);
                        } else {
                            editor.cursor.move_right(line.len() as u16 - 1);
                        }
                    }
                }
                Action::MoveCursorDown => {
                    let rows = editor.lines.len();
                    if rows == 1 {
                        editor.cursor.move_down(0);
                    }
                    editor.cursor.move_down(rows as u16 - 1);
                }
                Action::MoveCursorUp => editor.cursor.move_up(),
                Action::EnterCommandChar(c) => editor.command.push(c),
                Action::ExecuteCommand => editor.execute_command(),
                Action::EnterChar(c) => {
                    let (x, y) = editor.cursor.get_position();
                    if let Some(line) = editor.lines.get_mut(y as usize) {
                        if line.is_empty() {
                            line.push(c);
                            editor.cursor.move_right(u16::MAX)
                        } else if x > line.len() as u16 {
                            line.push(c);
                            editor.cursor.move_right(u16::MAX)
                        } else {
                            line.insert(x as usize, c);
                            editor.cursor.move_right(u16::MAX)
                        }
                    }
                }
                Action::NewLine => {
                    editor.lines.push(String::new());
                    editor.cursor.move_down(u16::MAX);
                }
                Action::AppendShortcutChar(c) => {
                    editor.shortcut_buffer.push(c);
                    match editor.shortcut_buffer.as_str() {
                        "gg" => {
                            editor.cursor.move_cursor_to_begin();
                            editor.shortcut_buffer.clear();
                        }
                        "G" => {
                            editor.cursor.move_cursor_to_end();
                            editor.shortcut_buffer.clear();
                        }
                        "dd" => {
                            let (_, c_row) = editor.cursor.get_position();

                            if editor.lines.len() > 1 {
                                editor.lines.remove(c_row as usize);
                            } 
                            editor.shortcut_buffer.clear();
                        }
                        _ => {}
                    }
                }
                Action::BackspaceInInsertMode => {
                    let (c_row, c_col) = editor.cursor.get_position();

                    if c_col == 0 && c_row != 0 {
                        if let Some(line) = editor.lines.get_mut(c_col as usize) {
                            if line.is_empty() {
                                editor.lines.remove(c_col as usize);
                            }
                        };
                    }
                }
                Action::ClearShortuctBuffer => editor.shortcut_buffer.clear(),
                _ => {}
            }
        }

        editor.generate().map_err(|e| {
            eprintln!("Something goes wrong during editor generation: {}", e);
            e
        })?;

        stdout.flush().unwrap();
    }

    Ok(())
}

fn handle_insert_mode_event(event: &Event) -> Action {
    match event {
        Event::Key(key_event) => match key_event.code {
            KeyCode::Esc => Action::EnterVisualMode,
            KeyCode::Enter => Action::NewLine,
            KeyCode::Backspace => Action::BackspaceInInsertMode,
            KeyCode::Char(c) => Action::EnterChar(c),
            _ => Action::Unknown,
        },
        _ => Action::Unknown,
    }
}

fn handle_visual_mode_event(event: &Event) -> Action {
    match event {
        Event::Key(key_event) => match key_event.code {
            KeyCode::Esc => Action::ClearShortuctBuffer,
            KeyCode::Char('i') => Action::EnterInsertMode,
            KeyCode::Char('a') => Action::EnterInsertModeNext,
            KeyCode::Char('o') => Action::EnterInsertModeInNewLine,
            KeyCode::Char(':') => Action::EnterCommandMode,
            KeyCode::Char('h') => Action::MoveCursorLeft,
            KeyCode::Char('l') => Action::MoveCursorRight,
            KeyCode::Char('j') => Action::MoveCursorDown,
            KeyCode::Char('k') => Action::MoveCursorUp,
            KeyCode::Char(c) => Action::AppendShortcutChar(c),
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
