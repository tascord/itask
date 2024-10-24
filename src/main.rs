use std::{
    io::{self, BufRead, BufReader, Stdout},
    process::{Command, Stdio},
    sync::{Arc, RwLock},
    thread::{self, spawn, Thread},
    time::Duration,
};

use anyhow::{bail, Context};
use menu::main_menu;
use ratatui::{
    crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{
            self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
        },
    },
    layout::{Constraint, Layout, Margin},
    prelude::CrosstermBackend,
    style::{Color, Stylize},
    widgets::{Block, Paragraph, Widget, Wrap},
    Frame, Terminal,
};
mod menu;

#[derive(Default)]
struct Model {
    job1: RwLock<Option<std::sync::mpsc::Receiver<String>>>,
    job2: RwLock<Option<std::sync::mpsc::Receiver<String>>>,
    menu: RwLock<Option<usize>>,
    quit: RwLock<bool>,
}

impl Model {
    // | ------- | --- |
    // |  Task 1 |  J  |
    // | ~~~~~~~ |  O  |
    // |  Task 2 |  B  |
    // | ------- | --- |

    fn start_j1(self: &Arc<Self>, c: Command) -> anyhow::Result<()> {
        if self.job1.read().unwrap().is_some() {
            bail!("Job 1 is already running.")
        }

        let mut c = c;
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        self.job1.write().unwrap().replace(rx);

        spawn(move || {
            let child = c
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();

            let buf = BufReader::new(child.stdout.unwrap());
            for line in buf.lines() {
                match line {
                    Ok(l) => {
                        let _ = tx.send(l);
                    }
                    Err(e) => {
                        println!("Failed reading output: {:?}", e);
                        return;
                    }
                }
            }
        });
        Ok(())
    }

    //

    fn keys(self: &Arc<Self>) -> anyhow::Result<()> {
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                let mut menu = self.menu.write().unwrap();
                match key.code {
                    KeyCode::Char('j') => {
                        if menu.is_none() {
                            *menu = Some(main_menu().first());
                        } else {
                            *menu = None
                        }
                    }

                    KeyCode::Char('q') => {
                        *self.quit.write().unwrap() = true;
                    }

                    KeyCode::Esc => {
                        if let Some(idx) = *menu {
                            *menu = main_menu().back(idx);
                        }
                    }

                    KeyCode::Up => {
                        if let Some(idx) = *menu {
                            menu.replace(main_menu().up(idx));
                        }
                    }

                    KeyCode::Down => {
                        if let Some(idx) = *menu {
                            menu.replace(main_menu().down(idx));
                        }
                    }

                    KeyCode::Enter => {
                        if let Some(idx) = *menu {
                            menu.replace(main_menu().enter(idx));
                        }
                    }

                    _ => {}
                }
            }
        }

        Ok(())
    }

    //

    pub fn render(self: &Arc<Self>, frame: &mut Frame<'_>) {
        self.keys().unwrap();

        let main = Layout::new(ratatui::layout::Direction::Horizontal, {
            match self.menu.read().unwrap().is_some() {
                true => Constraint::from_maxes([170, 30]),
                false => Constraint::from_percentages([100]),
            }
        })
        .split(frame.area());

        let jobs = Layout::new(
            ratatui::layout::Direction::Vertical,
            Constraint::from_percentages([50, 50]),
        )
        .split(main[0]);

        if let Some(idx) = *self.menu.read().unwrap() {
            let mut idx = idx;
            frame.render_stateful_widget(main_menu(), main[1], &mut idx);
            frame.render_widget(
                Paragraph::new(format!("#{idx} :: {:?}", main_menu().0)).wrap(Wrap { trim: true }),
                jobs[0].inner(Margin::new(1, 1)),
            );
        }

        frame.render_widget(Block::new().hidden(), frame.area());

        frame.render_widget(Block::bordered().title("Job One"), jobs[0]);
        frame.render_widget(Block::bordered().title("Job Two"), jobs[1]);
    }
}

#[tokio::main]
async fn main() {
    let mut t = setup_terminal().unwrap();
    let m = Arc::new(Model::default());

    loop {
        t.draw(|f| m.render(f)).unwrap();
        if *m.quit.read().unwrap() {
            break;
        }
    }

    restore_terminal(&mut t).unwrap();
}

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = io::stdout();
    enable_raw_mode().context("failed to enable raw mode")?;
    execute!(stdout, EnterAlternateScreen).context("unable to enter alternate screen")?;
    Terminal::new(CrosstermBackend::new(stdout)).context("creating terminal failed")
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("unable to switch to main screen")?;
    terminal.show_cursor().context("unable to show cursor")
}
