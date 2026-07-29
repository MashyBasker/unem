use std::collections::VecDeque;

use ratatui::crossterm::event::{self, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout, Position};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;
use ratatui::{DefaultTerminal, Frame};
use color_eyre::Result;

// Cap scrollback like a terminal emulator so long sessions don't grow memory unbounded.
const MAX_EXCHANGES: usize = 1000;

const PROMPT_PREFIX: &str = "> ";

struct Exchange {
    prompt: String,
    response: String,
}

pub struct App {
    // Value in the input box
    input: String,
    // Position of the current character
    char_index: usize,
    // Current input mode
    input_mode: InputMode,
    // History of completed prompt/response exchanges (oldest first)
    history: VecDeque<Exchange>,
}

enum InputMode {
    Normal,
    Editing,
}

// Placeholder for real LLM integration — not implemented in this repo yet.
fn generate_response(prompt: &str) -> String {
    format!("\n{prompt}")
}

impl App {
    pub const fn new() -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            history: VecDeque::new(),
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
        let prompt = std::mem::take(&mut self.input);
        let response = generate_response(&prompt);
        self.history.push_back(Exchange { prompt, response });
        if self.history.len() > MAX_EXCHANGES {
            self.history.pop_front();
        }
        self.reset_cursor();
    }

    fn delete_character(&mut self) {
        let is_char_not_leftmost = self.char_index != 0;
        if is_char_not_leftmost {
            let current_index = self.char_index;
            let from_left_to_current_index = current_index - 1;
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);
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

    // Renders the transcript (past exchanges + the live input line, fenced by horizontal
    // rules) as one scrolling buffer, so the input box always sits right after the latest
    // output instead of occupying a fixed pane.
    fn transcript_lines(&self, width: u16) -> Vec<Line<'_>> {
        let mut lines = Vec::new();
        for exchange in &self.history {
            lines.push(Line::from(vec![
                PROMPT_PREFIX.bold().fg(Color::Cyan),
                exchange.prompt.as_str().into(),
            ]));
            for response_line in exchange.response.split('\n') {
                lines.push(Line::from(response_line).fg(Color::Gray));
            }
            lines.push(Line::default());
        }

        let rule = Line::from("─".repeat(width as usize)).fg(Color::DarkGray);
        let input_style = match self.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        };
        lines.push(rule.clone());
        lines.push(Line::from(vec![
            PROMPT_PREFIX.bold().fg(Color::Cyan),
            Span::styled(self.input.as_str(), input_style),
        ]));
        lines.push(rule);

        lines
    }

    fn render(&self, frame: &mut Frame) {
        let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]);

        let [help_area, transcript_area] = frame.area().layout(&layout);

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

        let lines = self.transcript_lines(transcript_area.width);
        let line_count = lines.len() as u16;
        let scroll = line_count.saturating_sub(transcript_area.height);
        let transcript = Paragraph::new(lines).scroll((scroll, 0));
        frame.render_widget(transcript, transcript_area);

        if matches!(self.input_mode, InputMode::Editing) {
            // The live input line sits one row above the trailing rule.
            let input_line_index = line_count.saturating_sub(2);
            let visible_row = input_line_index.saturating_sub(scroll);
            frame.set_cursor_position(Position::new(
                transcript_area.x + PROMPT_PREFIX.len() as u16 + self.char_index as u16,
                transcript_area.y + visible_row,
            ));
        }
    }
}
