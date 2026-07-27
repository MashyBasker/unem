use ratatui::crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Paragraph};
use ratatui::{DefaultTerminal, Frame};
use color_eyre::Result;

pub struct App {
    // Value in the input box
    input: String,
    // Position of the current character
    char_index: usize,
    // Current input mode
    input_mode: InputMode,
    // Message
    message: String,
}

enum InputMode {
    Normal,
    Editing,
}

impl App {
    pub const fn new() -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            message: String::new(),
            char_index: 0,
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved = self.char_index.saturating_sub(1);
        self.char_index = self.clamp_cursor(cursor_moved);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved = self.char_index.saturating_add(1);
        self.char_index = self.clamp_cursor(cursor_moved);
    }

    fn enter_character(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    fn reset_cursor(&mut self) {
        self.char_index = 0;
    }

    fn submit_message(&mut self) {
        self.message = self.input.clone();
        self.input.clear();
        self.reset_cursor();
    }

    fn delete_character(&mut self) {
        let is_char_not_leftmost = self.char_index != 0;
        if is_char_not_leftmost {
            let current_index = self.char_index;
            let from_left_to_current_index = current_index - 1;
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(from_left_to_current_index);
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.char_index)
            .unwrap_or(self.input.len())
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            if let Some(key) = event::read()?.as_key_press_event() {
                match self.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('i') => {
                            self.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter => self.submit_message(),
                        KeyCode::Char(to_insert) => self.enter_character(to_insert),
                        KeyCode::Backspace => self.delete_character(),
                        KeyCode::Left => self.move_cursor_left(),
                        KeyCode::Right => self.move_cursor_right(),
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        _ => {}
                    },
                    InputMode::Editing => {}
                }
            }
        }
    }

    fn render(&self, frame: &mut Frame) {
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
        ]);

        let [help_area, input_area, message_area] = frame.area().layout(&layout);

        let (msg, style) = match self.input_mode {
            InputMode::Normal => (
                vec![
                    "Press".into(),
                    " q".into(),
                    " to exit, ".into(),
                    "i".bold(),
                    " to start editing.".bold(),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Editing => (
                vec![
                    "Press".into(),
                    " Esc".bold(),
                    " to stop editing, ".into(),
                    "Enter".bold(),
                    " to record the message".into(),
                ],
                Style::default(),
            ),
        };
        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);

        let input = Paragraph::new(self.input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, input_area);
        match self.input_mode {
            InputMode::Normal => {}

            InputMode::Editing => frame.set_cursor_position(Position::new(
                input_area.x + self.char_index as u16 + 1,
                input_area.y + 1,
            )),
        }

        frame.render_widget(self.message.clone(), message_area);
    }
}