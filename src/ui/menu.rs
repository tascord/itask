use std::{
    fmt::Debug,
    ops::Deref,
    sync::{Arc, RwLock},
    vec,
};

use ratatui::{
    buffer::Buffer,
    crossterm::event::KeyCode,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Style, Stylize},
    widgets::{Block, Clear, Paragraph, StatefulWidget, Widget, WidgetRef},
};

use crate::Model;

use super::Prompt;

#[derive(Clone)]
pub enum MenuItem {
    Section {
        title: String,
        children: Vec<usize>,
        parent: Option<usize>,
    },

    Item {
        title: String,
        handler: Arc<Box<dyn Fn(Arc<Model>)>>,
        parent: Option<usize>,
    },
}

impl Debug for MenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Section {
                title,
                children,
                parent,
            } => f
                .debug_struct("Section")
                .field("title", title)
                .field("children", children)
                .field("parent", parent)
                .finish(),
            Self::Item { title, parent, .. } => f
                .debug_struct("Item")
                .field("title", title)
                .field("handler", &"|| {{}}")
                .field("parent", parent)
                .finish(),
        }
    }
}

impl MenuItem {
    pub fn parent(&self) -> Option<usize> {
        match self {
            MenuItem::Section { parent, .. } => parent.clone(),
            MenuItem::Item { parent, .. } => parent.clone(),
        }
    }

    pub fn items(&self) -> Vec<usize> {
        match self {
            MenuItem::Item { .. } => vec![],
            MenuItem::Section { children, .. } => children.clone(),
        }
    }

    pub fn title(&self) -> String {
        match self {
            MenuItem::Item { title, .. } => title.clone(),
            MenuItem::Section { title, .. } => title.clone(),
        }
    }
}

impl PartialEq for MenuItem {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Section {
                    title: l_title,
                    children: l_children,
                    parent: l_parent,
                },
                Self::Section {
                    title: r_title,
                    children: r_children,
                    parent: r_parent,
                },
            ) => l_title == r_title && l_children == r_children && l_parent == r_parent,
            (
                Self::Item {
                    title: l_title,
                    parent: l_parent,
                    ..
                },
                Self::Item {
                    title: r_title,
                    parent: r_parent,
                    ..
                },
            ) => l_title == r_title && l_parent == r_parent,
            _ => false,
        }
    }
}

//

pub struct Menu(pub Vec<MenuItem>);
impl Menu {
    pub fn with_item(
        &mut self,
        title: &str,
        handler: impl Fn(Arc<Model>) + 'static,
        p: Option<usize>,
    ) -> usize {
        self.0.push(MenuItem::Item {
            title: title.to_string(),
            handler: Arc::new(Box::new(handler)),
            parent: p,
        });

        let len = self.0.len() - 1;
        p.inspect(|p| match self.0.get_mut(*p).unwrap() {
            MenuItem::Section { children, .. } => {
                children.push(len);
            }
            _ => panic!("Nope"),
        });

        len
    }

    pub fn with_section(&mut self, title: &str, p: Option<usize>) -> usize {
        self.0.push(MenuItem::Section {
            title: title.to_string(),
            children: Vec::new(),
            parent: p,
        });

        let len = self.0.len() - 1;
        p.inspect(|p| match self.0.get_mut(*p).unwrap() {
            MenuItem::Section { children, .. } => {
                children.push(len);
            }
            _ => panic!("Nope"),
        });

        len
    }

    //

    pub fn up(&self, idx: usize) -> usize {
        let items = self
            .0
            .get(self.0.get(idx).unwrap().parent().unwrap())
            .unwrap()
            .items();
        items
            .get(
                items
                    .iter()
                    .position(|i| *i == idx)
                    .unwrap()
                    .saturating_sub(1),
            )
            .copied()
            .unwrap_or(idx)
    }

    pub fn down(&self, idx: usize) -> usize {
        let items = self
            .0
            .get(self.0.get(idx).unwrap().parent().unwrap())
            .unwrap()
            .items();
        items
            .get(items.iter().position(|i| *i == idx).unwrap() + 1)
            .copied()
            .unwrap_or(idx)
    }

    pub fn enter(&self, idx: usize, model: Arc<Model>) -> usize {
        match self.0.get(idx).unwrap() {
            MenuItem::Section { children, .. } => children.first().copied().unwrap_or(idx),
            MenuItem::Item { handler, .. } => {
                handler(model);
                idx
            }
        }
    }

    pub fn back(&self, idx: usize) -> Option<usize> {
        let parent = self.0.get(idx).unwrap().parent();
        parent
            .map(|p| self.0.get(p).unwrap().parent().map(|_| p))
            .flatten()
    }

    //

    pub fn first(&self) -> usize {
        *self
            .0
            .iter()
            .find(|i| !i.items().is_empty())
            .unwrap()
            .items()
            .first()
            .unwrap()
    }
}

impl StatefulWidget for Menu {
    type State = usize;

    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let containter = self.0.get(*state).unwrap().parent();
        let container = self.0.get(containter.unwrap()).unwrap();

        Block::bordered().title(container.title()).render(area, buf);
        let area = Layout::new(ratatui::layout::Direction::Vertical, {
            let mut constraints = container
                .items()
                .iter()
                .map(|_| Constraint::Length(1))
                .collect::<Vec<_>>();

            constraints.extend(vec![Constraint::Fill(1)]);
            constraints
        })
        .split(area.inner(Margin::new(1, 1)));

        self.0
            .iter()
            .enumerate()
            .filter(|(i, _)| container.items().contains(i))
            .enumerate()
            .for_each(|(idx, (i, e))| e.clone().render(area[idx], buf, &mut (i == *state)));
    }
}

impl StatefulWidget for MenuItem {
    type State = bool;
    fn render(
        self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &mut Self::State,
    ) {
        let name = match self {
            MenuItem::Section { title, .. } => format!("📂 {title}"),
            MenuItem::Item { title, .. } => format!("🧭 {title}"),
        };

        Paragraph::new(name)
            .alignment(ratatui::layout::Alignment::Left)
            .style(match state {
                true => Style::new().on_white().black().bold(),
                false => Style::new().bold().white(),
            })
            .render(area, buf);
    }
}

macro_rules! menu {
    // Rule to create a root menu section
    ($menu:ident, $name:literal => { $($sub:tt)* }) => {
        let root = $menu.with_section($name, None);
        menu!(@subsections $menu, root, $($sub)*);
    };

    // Rule to create a nested menu section within a parent
    (@subsections $menu:ident, $parent:expr, $name:literal => { $($sub:tt)* } , $($rest:tt)*) => {
        let section = $menu.with_section($name, Some($parent));
        menu!(@subsections $menu, section, $($sub)*);
        menu!(@subsections $menu, $parent, $($rest)*);
    };

    // Rule to create a single menu item within a section
    (@subsections $menu:ident, $parent:expr, $name:literal => $action:expr, $($rest:tt)*) => {
        $menu.with_item($name, $action, Some($parent));
        menu!(@subsections $menu, $parent, $($rest)*);
    };

    // End of a section without more subsections
    (@subsections $menu:ident, $parent:expr, $name:literal => { $($sub:tt)* }) => {
        let section = $menu.with_section($name, Some($parent));
        menu!(@subsections $menu, section, $($sub)*);
    };

    // End of an item without more subsections
    (@subsections $menu:ident, $parent:expr, $name:literal => $action:expr) => {
        $menu.with_item($name, $action, Some($parent));
    };

    // Empty rule to stop recursion
    (@subsections $menu:ident, $parent:expr,) => {};
}

pub fn main_menu() -> Menu {
    let mut menu = Menu(vec![]);

    menu! {
        menu,
        "Jobs" => {
            "Run (Server)" => {
                "Sites (bin)" => |_| {},
            },
            "Build Frontend" => {
                "Sites (wasm)" => |_| {},
                "Something (wasm+elm)" => |_| {},
            },
            "Configure iTask" => {
                "Set ENV" => |m| {
                    *m.prompt.write().unwrap() =
                        Some(Prompt::secret("Enter your Yubikey pin", |_pin| {
                            return Err("Invalid pin".to_string());
                        }));
                },
            },
        }
    };

    menu
}

//
