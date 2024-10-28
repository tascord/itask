use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader, Stdout},
    process::{Command, Stdio},
    sync::{Arc, RwLock},
    thread::spawn,
    time::Duration,
};

use anyhow::{bail, Context};
use itertools::Itertools;
use ratatui::{
    crossterm::{
        event::{self, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph, WidgetRef, Wrap},
    Frame, Terminal,
};
use ui::{main_menu, Prompt};
mod ui;

const BANNER: &str = include_str!("../banner");

#[derive(Default)]
struct Model {
    job1: RwLock<(String, Option<RwLock<VecDeque<String>>>)>,
    job2: RwLock<(String, Option<RwLock<VecDeque<String>>>)>,
    prompt: RwLock<Option<Prompt>>,
    menu: RwLock<Option<usize>>,
    quit: RwLock<bool>,
}

impl Model {
    // | ------- | --- |
    // |  Task 1 |  J  |
    // | ~~~~~~~ |  O  |
    // |  Task 2 |  B  |
    // | ------- | --- |

    fn start_job(
        job: RwLock<(String, Option<Arc<RwLock<VecDeque<String>>>>)>,
        c: Command,
    ) -> anyhow::Result<()> {
        if job.read().unwrap().1.is_some() {
            bail!("Job is already running.")
        }

        let mut c = c;
        let vdq = Arc::new(RwLock::new(VecDeque::new()));
        job.write().unwrap().1.replace(vdq.clone());

        job.write().unwrap().0 = {
            let mut title = format!("{c:?}").replace('"', "");
            if title.len() > 10 {
                title = format!("{}...", title.split_at(7).0);
            }
            title
        };

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
                        let mut lock = vdq.write().unwrap();
                        lock.push_back(l);
                        while lock.len() > 1000 {
                            lock.pop_front();
                        }
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
                let mut prompt = self.prompt.write().unwrap();
                if prompt.is_some() {
                    if key.code == KeyCode::Esc {
                        *prompt = None;
                    }

                    prompt.as_mut().inspect(|p| p.input(key.code));
                    return Ok(());
                }

                drop(prompt);
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
                            menu.replace(main_menu().enter(idx, self.clone()));
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

        if let Some(idx) = *self.menu.read().unwrap() {
            let mut idx = idx;
            frame.render_stateful_widget(main_menu(), main[1], &mut idx);
        }

        frame.render_widget(Block::new().hidden(), frame.area());
        self.render_jobs(main[0], frame);
        self.render_prompt(frame);
    }

    pub fn render_prompt(self: &Arc<Self>, frame: &mut Frame<'_>) {
        let area = frame.area();
        let prompt_area = Rect {
            x: area.width / 4,
            y: area.height / 2 - 3,
            width: area.width / 2,
            height: 6,
        };

        if let Some(prompt) = &*self.prompt.read().unwrap() {
            frame.render_widget(prompt.clone(), prompt_area);
        }
    }

    pub fn render_jobs(self: &Arc<Self>, area: Rect, frame: &mut Frame<'_>) {
        let j1 = self.job1.read().unwrap().1.is_some();
        let j2 = self.job2.read().unwrap().1.is_some();

        let jobs = Layout::new(
            ratatui::layout::Direction::Vertical,
            match (j1, j2) {
                (false, false) | (false, true) | (true, false) => vec![Constraint::Fill(1)],
                _ => Constraint::from_percentages([50, 50]),
            },
        )
        .split(area);

        if j1 {
            let job = self.job1.read().unwrap();
            let title = job.0.clone();
            let logs = job.1.as_ref().unwrap().read().unwrap();

            let text = logs
                .iter()
                .map(|i| Line::from(i.to_string()))
                .collect::<Vec<_>>();

            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(Block::bordered().title(title)),
                *jobs.first().unwrap(),
            );
        }

        if j2 {
            let job = self.job1.read().unwrap();
            let title = job.0.clone();
            let logs = job.1.as_ref().unwrap().read().unwrap();

            let text = logs
                .iter()
                .map(|i| Line::from(i.to_string()))
                .collect::<Vec<_>>();

            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(Block::bordered().title(title)),
                *jobs.last().unwrap(),
            );
        }

        if !(j1 || j2) {
            Self::banner(area, frame);
        }
    }

    pub fn banner(area: Rect, frame: &mut Frame<'_>) {
        let (title, help) = BANNER.split("---").collect_tuple().unwrap();

        let lay = Layout::new(
            Direction::Vertical,
            vec![
                Constraint::Fill(1),
                Constraint::Length(title.lines().count() as u16),
                Constraint::Length(help.lines().count() as u16),
                Constraint::Fill(1),
            ],
        )
        .split(area);

        frame.render_widget(Block::bordered(), area);
        frame.render_widget(
            Paragraph::new(title.lines().map(|l| Line::from(l)).collect::<Vec<_>>())
                .alignment(ratatui::layout::Alignment::Center),
            lay[1],
        );
        frame.render_widget(
            Paragraph::new(
                help.lines()
                    .map(|l| Line::from(l.trim()))
                    .collect::<Vec<_>>(),
            )
            .alignment(ratatui::layout::Alignment::Center),
            lay[2],
        );
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
