use ratatui::{
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

pub fn add_cursor<'a>(s: String, c: usize) -> Line<'a> {
    Line::from(vec![
        Span::raw(s[..c].to_string()),
        Span::styled(
            s.clone().chars().nth(c).unwrap_or(' ').to_string(),
            Style::new().bg(Color::Yellow),
        ),
        Span::raw(match c == s.len() {
            true => String::new(),
            false => s[c + 1..].to_string(),
        }),
    ])
}

pub fn add_reveal_cursor<'a>(s: String, c: usize, cc: char) -> Line<'a> {
    Line::from(vec![
        Span::raw(s[..c].to_string()),
        Span::styled(cc.to_string(), Style::new().bg(Color::Yellow)),
        Span::raw(match c == s.len() {
            true => String::new(),
            false => s[c + 1..].to_string(),
        }),
    ])
}

#[derive(Clone)]
pub struct Input {
    pub secret: bool,
}

impl Input {
    pub fn new(secret: bool) -> Self {
        Self { secret }
    }
}

impl StatefulWidget for Input {
    type State = (usize, String);

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let (cursor, value) = state;
        let cursor = *cursor;

        let val = match self.secret {
            true => "*".repeat(value.len()),
            false => value.clone(),
        };

        let mut offset = 0;
        let slice = match area.columns().count() < val.len() + 3 && cursor > area.columns().count()
        {
            false => val.clone(),
            true => {
                // slice with the cursor at the end
                offset = cursor - area.columns().count() + 3;
                val[offset..].to_string()
            }
        };

        let block = Block::default().borders(Borders::ALL).yellow();

        Paragraph::new(match self.secret {
            false => add_cursor(slice, cursor - offset),
            true => add_reveal_cursor(
                slice,
                cursor - offset,
                value.clone().chars().nth(cursor - offset).unwrap_or(' '),
            ),
        })
        .block(block)
        .render(area, buf);
    }
}
