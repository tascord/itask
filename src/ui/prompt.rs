use std::sync::{Arc, RwLock};

use ratatui::{
    crossterm::event::KeyCode,
    layout::{Constraint, Direction, Layout, Margin},
    style::Stylize,
    widgets::{Block, Clear, Paragraph, StatefulWidget, Widget},
};

use super::Input;

#[derive(Clone)]
pub struct Prompt {
    title: String,
    secret: bool,
    handler: Arc<Box<dyn Fn(String) -> Result<(), String>>>,
    state: Arc<RwLock<(usize, String, String)>>,
}

impl Prompt {
    pub fn new(title: &str, handler: impl Fn(String) -> Result<(), String> + 'static) -> Self {
        Self {
            secret: false,
            title: title.to_string(),
            handler: Arc::new(Box::new(handler)),
            state: Default::default(),
        }
    }

    pub fn secret(title: &str, handler: impl Fn(String) -> Result<(), String> + 'static) -> Self {
        Self {
            secret: true,
            title: title.to_string(),
            handler: Arc::new(Box::new(handler)),
            state: Default::default(),
        }
    }

    pub fn input(&self, k: KeyCode) {
        let mut state = self.state.write().unwrap();
        let (cursor, value, error) = &mut *state;

        match k {
            KeyCode::Backspace => {
                if *cursor > 0 {
                    value.remove(*cursor - 1);
                    *cursor -= 1;
                }
            }
            KeyCode::Left => {
                if *cursor > 0 {
                    *cursor -= 1;
                }
            }
            KeyCode::Right => {
                if *cursor < value.len() {
                    *cursor += 1;
                }
            }
            KeyCode::Home => {
                *cursor = 0;
            }
            KeyCode::End => {
                *cursor = value.len();
            }
            KeyCode::Delete => {
                value.remove(*cursor);
            }
            KeyCode::Char(c) => {
                value.insert(*cursor, c);
                *cursor += 1;
            }

            KeyCode::Enter => match (self.handler.clone())(value.to_string()) {
                Err(e) => *error = e,
                _ => {}
            },
            _ => {}
        }
    }
}

impl Widget for Prompt {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let mut state = self.state.read().unwrap();
        let state = &mut state;

        Clear.render(area, buf);
        Block::bordered()
            .title(self.title.clone())
            .render(area, buf);
        let error = state.2.clone();

        let lay = Layout::new(
            Direction::Vertical,
            vec![
                Constraint::Fill(1),
                Constraint::Length(match error.is_empty() {
                    false => 1,
                    true => 0,
                }),
                Constraint::Length(3),
                Constraint::Fill(1),
            ],
        )
        .split(area.inner(Margin::new(1, 1)));

        if !error.is_empty() {
            Paragraph::new(error).red().render(lay[1], buf);
        }

        Input::new(self.secret).render(lay[2], buf, &mut (state.0, state.1.clone()));
    }
}
