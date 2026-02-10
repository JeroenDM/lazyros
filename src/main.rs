use std::{sync::mpsc, thread};

use color_eyre::Result;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::{
        Color, Modifier, Style, Stylize,
        palette::tailwind::{BLUE, GREEN, SLATE},
    },
    symbols,
    text::Line,
    widgets::{
        Block, Borders, HighlightSpacing, List, ListItem, ListState, Padding, Paragraph,
        StatefulWidget, Widget, Wrap,
    },
};

const TODO_HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
const NORMAL_ROW_BG: Color = SLATE.c950;
const ALT_ROW_BG_COLOR: Color = SLATE.c900;
const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);
const TEXT_FG_COLOR: Color = SLATE.c200;
const COMPLETED_TEXT_FG_COLOR: Color = GREEN.c500;

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::default().run(terminal);
    ratatui::restore();
    app_result
}

///////////////////////////////////////////////////////////////////////////////
/// Event handling
///////////////////////////////////////////////////////////////////////////////

enum LREvent {
    // Input events
    Quit,
    Up,
    Down,
    Left,
    Right,
    Enter, // Select, activate, confirm, ...
    Home,  // Go to first item or start of line.
    End,   // Go to last item or end of line.

    // ROS2 events
    TopicList(Vec<String>),
}

fn run_input_loop(tx: mpsc::Sender<LREvent>) -> Result<()> {
    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            let event = match key.code {
                KeyCode::Char('q') | KeyCode::Esc => LREvent::Quit,
                KeyCode::Char('h') | KeyCode::Left => LREvent::Left,
                KeyCode::Char('j') | KeyCode::Down => LREvent::Down,
                KeyCode::Char('k') | KeyCode::Up => LREvent::Up,
                KeyCode::Char('g') | KeyCode::Home => LREvent::Home,
                KeyCode::Char('G') | KeyCode::End => LREvent::End,
                KeyCode::Char('l') | KeyCode::Right  => LREvent::Right,
                KeyCode::Enter => LREvent::Enter,
                _ => continue,
            };
            tx.send(event)?;
        };
    }
}

///////////////////////////////////////////////////////////////////////////////
/// ROS2 Command handling
///////////////////////////////////////////////////////////////////////////////

enum ROS2Command {
    TopicList,
}

fn run_cmd_loop(rx: mpsc::Receiver<ROS2Command>, tx: mpsc::Sender<LREvent>) -> Result<()> {
    loop {
        // Here you would run your ROS2 command and parse the output.
        // TODO now just sleep for a bit.
        let _cmd = rx.recv()?;
        // std::thread::sleep(std::time::Duration::from_secs(5));
        // let dummy_topics = vec![
        //     "topic1".to_string(),
        //     "topic2".to_string(),
        //     "topic3".to_string(),
        // ];
        let dummy_topics = (1..=100).map(|i| format!("topic{}", i)).collect();
        tx.send(LREvent::TopicList(dummy_topics))?;
    }
}

///////////////////////////////////////////////////////////////////////////////
/// app state
///////////////////////////////////////////////////////////////////////////////
struct App {
    should_exit: bool,
    topics: TopicList,
}

struct TopicList {
    items: Vec<TodoItem>,
    state: ListState,
}

#[derive(Debug)]
struct TodoItem {
    todo: String,
    info: String,
    status: Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Status {
    Todo,
    Completed,
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_exit: false,
            topics: TopicList::from_iter([]),
            // topics: TopicList::from_iter([
            //     (
            //         Status::Todo,
            //         "Rewrite everything with Rust!",
            //         "I can't hold my inner voice. He tells me to rewrite the complete universe with Rust",
            //     ),
            //     (
            //         Status::Completed,
            //         "Rewrite all of your tui apps with Ratatui",
            //         "Yes, you heard that right. Go and replace your tui with Ratatui.",
            //     ),
            //     (
            //         Status::Todo,
            //         "Pet your cat",
            //         "Minnak loves to be pet by you! Don't forget to pet and give some treats!",
            //     ),
            //     (
            //         Status::Todo,
            //         "Walk with your dog",
            //         "Max is bored, go walk with him!",
            //     ),
            //     (
            //         Status::Completed,
            //         "Pay the bills",
            //         "Pay the train subscription!!!",
            //     ),
            //     (
            //         Status::Completed,
            //         "Refactor list example",
            //         "If you see this info that means I completed this task!",
            //     ),
            // ]),
        }
    }
}

impl FromIterator<(Status, &'static str, &'static str)> for TopicList {
    fn from_iter<I: IntoIterator<Item = (Status, &'static str, &'static str)>>(iter: I) -> Self {
        let items = iter
            .into_iter()
            .map(|(status, todo, info)| TodoItem::new(status, todo, info))
            .collect();
        let state = ListState::default();
        Self { items, state }
    }
}

impl TodoItem {
    fn new(status: Status, todo: &str, info: &str) -> Self {
        Self {
            status,
            todo: todo.to_string(),
            info: info.to_string(),
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
/// UPDATE
///////////////////////////////////////////////////////////////////////////////

fn update(app: &mut App, event: LREvent, tx: &mpsc::Sender<ROS2Command>) -> Option<ROS2Command> {
    match event {
        LREvent::Quit => app.should_exit = true,
        LREvent::Left => app.topics.state.select(None),
        LREvent::Down => app.topics.state.select_next(),
        LREvent::Up => app.topics.state.select_previous(),
        LREvent::Home => app.topics.state.select_first(),
        LREvent::End => app.topics.state.select_last(),
        LREvent::Right => app.toggle_status(),
        LREvent::Enter => tx.send(ROS2Command::TopicList).unwrap(), 
        LREvent::TopicList(topics) => {
            let items = topics
                .into_iter()
                .map(|topic| {
                    TodoItem {
                        status: Status::Todo,
                        todo: topic.clone(),
                        info: format!("Info for topic {}", topic),
                    }
                }).collect();
            let state = ListState::default();
            app.topics = TopicList { items, state };
        }
    }
    Some(ROS2Command::TopicList)
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // 2 background threads to handle user input and ros2 commands.
        let (tx, rx) = mpsc::channel();
        let tx_user_input = tx.clone();
        thread::spawn(move || -> Result<()> {
            run_input_loop(tx_user_input)?;
            Ok(())
        });

        // Feedback channel for ros2 commands.
        let (tx_ros2_cmd, rx_ros2_cmd) = mpsc::channel();
        thread::spawn(move || -> Result<()> {
            run_cmd_loop(rx_ros2_cmd, tx)?;
            Ok(())
        });

        // Draw initial screen because loop below waits for the first input.
        terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;

        // Kick off main event loop.
        while !self.should_exit {
            let event = rx.recv().unwrap();
            update(&mut self, event, &tx_ros2_cmd);
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
        }
        Ok(())
    }

    /// Changes the status of the selected list item
    fn toggle_status(&mut self) {
        if let Some(i) = self.topics.state.selected() {
            self.topics.items[i].status = match self.topics.items[i].status {
                Status::Completed => Status::Todo,
                Status::Todo => Status::Completed,
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////////
/// RENDER
///////////////////////////////////////////////////////////////////////////////
impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let [list_area, item_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(3)]).areas(main_area);

        App::render_header(header_area, buf);
        App::render_footer(footer_area, buf);
        self.render_list(list_area, buf);
        self.render_selected_item(item_area, buf);
    }
}

/// Rendering logic for the app
impl App {
    fn render_header(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Ratatui List Example")
            .bold()
            .centered()
            .render(area, buf);
    }

    fn render_footer(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Use ↓↑ to move, ← to unselect, → to change status, g/G to go top/bottom.")
            .centered()
            .render(area, buf);
    }

    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .title(Line::raw("TODO List").centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(TODO_HEADER_STYLE)
            .bg(NORMAL_ROW_BG);

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<ListItem> = self
            .topics
            .items
            .iter()
            .enumerate()
            .map(|(i, todo_item)| {
                let color = alternate_colors(i);
                ListItem::from(todo_item).bg(color)
            })
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        // We need to disambiguate this trait method as both `Widget` and `StatefulWidget` share the
        // same method name `render`.
        StatefulWidget::render(list, area, buf, &mut self.topics.state);
    }

    fn render_selected_item(&self, area: Rect, buf: &mut Buffer) {
        // We get the info depending on the item's state.
        let info = if let Some(i) = self.topics.state.selected() {
            match self.topics.items[i].status {
                Status::Completed => format!("✓ DONE: {}", self.topics.items[i].info),
                Status::Todo => format!("☐ TODO: {}", self.topics.items[i].info),
            }
        } else {
            "Nothing selected...".to_string()
        };

        // We show the list item's info under the list in this paragraph
        let block = Block::new()
            .title(Line::raw("TODO Info").centered())
            .borders(Borders::TOP)
            .border_set(symbols::border::EMPTY)
            .border_style(TODO_HEADER_STYLE)
            .bg(NORMAL_ROW_BG)
            .padding(Padding::horizontal(1));

        // We can now render the item info
        Paragraph::new(info)
            .block(block)
            .fg(TEXT_FG_COLOR)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}

const fn alternate_colors(i: usize) -> Color {
    if i % 2 == 0 {
        NORMAL_ROW_BG
    } else {
        ALT_ROW_BG_COLOR
    }
}

impl From<&TodoItem> for ListItem<'_> {
    fn from(value: &TodoItem) -> Self {
        let line = match value.status {
            Status::Todo => Line::styled(format!(" ☐ {}", value.todo), TEXT_FG_COLOR),
            Status::Completed => {
                Line::styled(format!(" ✓ {}", value.todo), COMPLETED_TEXT_FG_COLOR)
            }
        };
        ListItem::new(line)
    }
}
