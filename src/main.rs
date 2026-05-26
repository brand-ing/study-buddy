mod data;
mod storage;
mod theme;

use chrono::{Datelike, Timelike};
use data::{Heatmap, Pomodoro, Task};
use iced::theme as iced_theme;
use iced::widget::{
    button, column, container, mouse_area, progress_bar, row, scrollable, text, text_input, Space,
};
use iced::{
    Alignment, Application, Color, Command, Element, Length, Settings, Subscription, Theme,
};
use iced::{time, window};
use storage::SaveData;
use theme::{
    AccentBtn, AppBg, CloseBtn, DeleteBtn, DotCell, Flat, GhostBtn, HeatCell, OuterBorder,
    Palette, PinBtn, ProgressStyle, Sidebar, TabBtn, TaskCheckBtn, TaskInput, TimeOfDay,
};

const APP_NAME: &str = "focus";

// ── Navigation ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Timer,
    Tasks,
    Heatmap,
}

// ── State ─────────────────────────────────────────────────────────────────

struct App {
    tod: TimeOfDay,
    tasks: Vec<Task>,
    task_input: String,
    next_id: u64,
    timer: Pomodoro,
    heatmap: Heatmap,
    active_tab: Tab,
    always_on_top: bool,
    // Countdown in seconds: >0 = controls visible, 0 = hidden.
    // Reset to 6 on mouse-near-top; decrements each Tick.
    hide_in_ticks: u8,
    sidebar_collapsed: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    TimerToggle,
    TimerReset,
    TimerSkip,
    TaskInputChanged(String),
    TaskAdd,
    TaskToggle(u64),
    TaskDelete(u64),
    RefreshTime,
    TabSelected(Tab),
    TitleBarDrag,
    WindowClose,
    ToggleAlwaysOnTop,
    MouseMoved(iced::Point),
    MouseLeft,
    SidebarToggle,
}

// ── Application ───────────────────────────────────────────────────────────

impl Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_: ()) -> (Self, Command<Message>) {
        let s = storage::load();
        (
            Self {
                tod: TimeOfDay::now(),
                tasks: s.tasks,
                task_input: String::new(),
                next_id: s.next_id,
                timer: Pomodoro::new(s.pomodoros_done),
                heatmap: s.heatmap,
                active_tab: Tab::Timer,
                always_on_top: false,
                hide_in_ticks: 0,
                sidebar_collapsed: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        APP_NAME.into()
    }

    fn theme(&self) -> Theme {
        self.tod.iced_theme()
    }

    fn style(&self) -> iced_theme::Application {
        iced_theme::Application::Custom(Box::new(AppBg(self.tod.palette())))
    }

    fn subscription(&self) -> Subscription<Message> {
        use std::time::Duration;
        // Single 1s tick drives both the pomodoro and the controls countdown
        let needs_tick = self.timer.running || self.hide_in_ticks > 0;
        let tick = if needs_tick {
            time::every(Duration::from_secs(1)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        };
        let clock = time::every(Duration::from_secs(60)).map(|_| Message::RefreshTime);
        let mouse_events =
            iced::event::listen_with(|event, _status| match event {
                iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                    Some(Message::MouseMoved(position))
                }
                iced::Event::Mouse(iced::mouse::Event::CursorLeft) => Some(Message::MouseLeft),
                _ => None,
            });
        Subscription::batch(vec![tick, clock, mouse_events])
    }

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::Tick => {
                if self.timer.running && self.timer.tick() {
                    self.heatmap.add(25);
                    self.persist();
                }
                if self.hide_in_ticks > 0 {
                    self.hide_in_ticks -= 1;
                }
            }
            Message::TimerToggle => self.timer.running = !self.timer.running,
            Message::TimerReset => self.timer.reset(),
            Message::TimerSkip => self.timer.skip(),
            Message::TaskInputChanged(s) => self.task_input = s,
            Message::TaskAdd => {
                let t = self.task_input.trim().to_string();
                if !t.is_empty() {
                    self.tasks.push(Task::new(self.next_id, t));
                    self.next_id += 1;
                    self.task_input.clear();
                    self.persist();
                }
            }
            Message::TaskToggle(id) => {
                if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
                    t.done = !t.done;
                }
                self.persist();
            }
            Message::TaskDelete(id) => {
                self.tasks.retain(|t| t.id != id);
                self.persist();
            }
            Message::RefreshTime => self.tod = TimeOfDay::now(),
            Message::TabSelected(tab) => self.active_tab = tab,
            Message::TitleBarDrag => return window::drag(window::Id::MAIN),
            Message::WindowClose => return window::close(window::Id::MAIN),
            Message::ToggleAlwaysOnTop => {
                self.always_on_top = !self.always_on_top;
                let level = if self.always_on_top {
                    window::Level::AlwaysOnTop
                } else {
                    window::Level::Normal
                };
                return window::change_level(window::Id::MAIN, level);
            }
            // Reset 6-second countdown whenever the mouse is near the top
            Message::MouseMoved(pos) => {
                if pos.y < 50.0 {
                    self.hide_in_ticks = 6;
                }
            }
            // Mouse exits the window entirely: hide immediately
            Message::MouseLeft => self.hide_in_ticks = 0,
            Message::SidebarToggle => self.sidebar_collapsed = !self.sidebar_collapsed,
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let p = self.tod.palette();
        let show_controls = self.hide_in_ticks > 0;

        let content: Element<Message> = match self.active_tab {
            Tab::Timer => timer_view(p, &self.timer),
            Tab::Tasks => tasks_view(p, &self.tasks, &self.task_input),
            Tab::Heatmap => heatmap_view(p, &self.heatmap),
        };

        let body = row(vec![
            nav_sidebar(p, self.active_tab, self.sidebar_collapsed),
            column(vec![
                top_bar(p, show_controls, self.always_on_top),
                content,
            ])
            .height(Length::Fill)
            .into(),
        ]);

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(iced_theme::Container::Custom(Box::new(OuterBorder(p))))
            .into()
    }
}

impl App {
    fn persist(&self) {
        storage::save(&SaveData {
            tasks: self.tasks.clone(),
            heatmap: self.heatmap.clone(),
            next_id: self.next_id,
            pomodoros_done: self.timer.done,
        });
    }
}

// ── Top Bar ───────────────────────────────────────────────────────────────
//
// Always-present 36px transparent strip at the top of the content column.
// Time-of-day + clock are always visible right-aligned.
// ⊤ · · · ✕ controls appear only while show_controls is true.

fn top_bar(p: Palette, show_controls: bool, always_on_top: bool) -> Element<'static, Message> {
    let now = chrono::Local::now();
    let time_str = format!("{:02}:{:02}", now.hour(), now.minute());

    let time_badge = row(vec![
        text(p.name)
            .size(11)
            .style(iced_theme::Text::Color(p.accent))
            .into(),
        Space::with_width(8).into(),
        text(time_str)
            .size(11)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
    ])
    .align_items(Alignment::Center);

    if !show_controls {
        // Invisible; whole strip is a drag zone, time badge stays visible
        return mouse_area(
            container(
                row(vec![
                    Space::with_width(Length::Fill).into(),
                    time_badge.into(),
                    Space::with_width(14).into(),
                ])
                .align_items(Alignment::Center)
                .height(Length::Fixed(36.0)),
            )
            .width(Length::Fill),
        )
        .on_press(Message::TitleBarDrag)
        .into();
    }

    // Always-on-top toggle (left)
    let pin = button(
        text("⊤")
            .size(14)
            .style(iced_theme::Text::Color(if always_on_top { p.accent } else { p.subtext })),
    )
    .padding([0, 12])
    .height(Length::Fixed(36.0))
    .style(iced_theme::Button::Custom(Box::new(PinBtn { p, active: always_on_top })))
    .on_press(Message::ToggleAlwaysOnTop);

    // Centre drag grip — 2 rows of 6 dots
    let dot_color = Color { a: 0.5, ..p.subtext };
    let make_dot = move || -> Element<'static, Message> {
        container(Space::new(Length::Fixed(4.0), Length::Fixed(4.0)))
            .style(iced_theme::Container::Custom(Box::new(DotCell(dot_color))))
            .into()
    };
    let top_dots: Vec<Element<Message>> = (0..6).map(|_| make_dot()).collect();
    let bot_dots: Vec<Element<Message>> = (0..6).map(|_| make_dot()).collect();

    let grip = column(vec![
        row(top_dots).spacing(4).into(),
        Space::with_height(4).into(),
        row(bot_dots).spacing(4).into(),
    ])
    .align_items(Alignment::Center);

    let drag_handle = mouse_area(
        container(grip)
            .center_x()
            .center_y()
            .padding([0, 20])
            .height(Length::Fixed(36.0)),
    )
    .on_press(Message::TitleBarDrag);

    // Close button (right, after the time badge)
    let close = button(
        text("✕")
            .size(11)
            .style(iced_theme::Text::Color(p.subtext)),
    )
    .padding([0, 14])
    .height(Length::Fixed(36.0))
    .style(iced_theme::Button::Custom(Box::new(CloseBtn(p))))
    .on_press(Message::WindowClose);

    // Transparent container — no background, floats over window
    container(
        row(vec![
            pin.into(),
            Space::with_width(Length::Fill).into(),
            drag_handle.into(),
            Space::with_width(Length::Fill).into(),
            time_badge.into(),
            Space::with_width(8).into(),
            close.into(),
        ])
        .align_items(Alignment::Center)
        .height(Length::Fixed(36.0)),
    )
    .width(Length::Fill)
    .style(iced_theme::Container::Custom(Box::new(Flat)))
    .into()
}

// ── Sidebar Drawer ────────────────────────────────────────────────────────

fn nav_sidebar(p: Palette, active: Tab, collapsed: bool) -> Element<'static, Message> {
    let w = if collapsed { 44.0 } else { 120.0 };

    let arrow = if collapsed { "›" } else { "‹" };
    let toggle_btn = button(
        text(arrow)
            .size(14)
            .style(iced_theme::Text::Color(p.subtext)),
    )
    .padding([4, 8])
    .style(iced_theme::Button::Custom(Box::new(GhostBtn(p))))
    .on_press(Message::SidebarToggle);

    let header: Element<Message> = if collapsed {
        container(toggle_btn)
            .padding([20, 6])
            .center_x()
            .width(Length::Fill)
            .into()
    } else {
        container(
            row(vec![
                text(APP_NAME)
                    .size(15)
                    .style(iced_theme::Text::Color(p.accent))
                    .into(),
                Space::with_width(Length::Fill).into(),
                toggle_btn.into(),
            ])
            .align_items(Alignment::Center),
        )
        .padding([20, 10, 16, 16])
        .into()
    };

    let nav: Element<Message> = if collapsed {
        Space::with_height(0).into()
    } else {
        column(vec![
            nav_tab("timer",    Tab::Timer,   active, p),
            nav_tab("tasks",    Tab::Tasks,   active, p),
            nav_tab("activity", Tab::Heatmap, active, p),
        ])
        .into()
    };

    container(
        column(vec![
            header,
            nav,
            Space::with_height(Length::Fill).into(),
        ]),
    )
    .width(Length::Fixed(w))
    .height(Length::Fill)
    .style(iced_theme::Container::Custom(Box::new(Sidebar(p))))
    .into()
}

fn nav_tab(label: &'static str, tab: Tab, active: Tab, p: Palette) -> Element<'static, Message> {
    button(text(label).size(13))
        .padding([10, 16])
        .width(Length::Fill)
        .style(iced_theme::Button::Custom(Box::new(TabBtn { p, active: tab == active })))
        .on_press(Message::TabSelected(tab))
        .into()
}

// ── Timer Tab ─────────────────────────────────────────────────────────────

fn timer_view(p: Palette, timer: &Pomodoro) -> Element<Message> {
    let phase = text(timer.phase.label())
        .size(13)
        .style(iced_theme::Text::Color(p.subtext));

    let digits = text(timer.format())
        .font(iced::Font::MONOSPACE)
        .size(72)
        .style(iced_theme::Text::Color(p.text));

    let bar = progress_bar(0.0..=1.0, timer.progress())
        .height(Length::Fixed(4.0))
        .style(iced_theme::ProgressBar::Custom(Box::new(ProgressStyle(p))));

    let cycle_pos = (timer.done % 4) as usize;
    let dots: Vec<Element<Message>> = (0..4)
        .map(|i| {
            let color = if i < cycle_pos { p.accent } else { p.surface2 };
            container(Space::new(Length::Fixed(9.0), Length::Fixed(9.0)))
                .style(iced_theme::Container::Custom(Box::new(DotCell(color))))
                .into()
        })
        .collect();

    let session_row = row(vec![
        row(dots).spacing(7).into(),
        Space::with_width(10).into(),
        text(format!("session {}/4", cycle_pos + 1))
            .size(12)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
    ])
    .align_items(Alignment::Center);

    let toggle_label = if timer.running { "⏸  Pause" } else { "▶  Start" };
    let toggle = button(text(toggle_label).size(14))
        .padding([10, 28])
        .style(iced_theme::Button::Custom(Box::new(AccentBtn(p))))
        .on_press(Message::TimerToggle);

    let reset = button(text("↺").size(16))
        .padding([10, 16])
        .style(iced_theme::Button::Custom(Box::new(GhostBtn(p))))
        .on_press(Message::TimerReset);

    let skip = button(text("⏭").size(16))
        .padding([10, 16])
        .style(iced_theme::Button::Custom(Box::new(GhostBtn(p))))
        .on_press(Message::TimerSkip);

    let controls = row(vec![toggle.into(), reset.into(), skip.into()])
        .spacing(10)
        .align_items(Alignment::Center);

    let bar_wrapper = container(bar).width(Length::Fixed(240.0));

    let inner = column(vec![
        phase.into(),
        Space::with_height(4).into(),
        digits.into(),
        Space::with_height(16).into(),
        bar_wrapper.into(),
        Space::with_height(20).into(),
        session_row.into(),
        Space::with_height(28).into(),
        controls.into(),
    ])
    .align_items(Alignment::Center);

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(iced_theme::Container::Custom(Box::new(Flat)))
        .into()
}

// ── Tasks Tab ─────────────────────────────────────────────────────────────

fn tasks_view<'a>(p: Palette, tasks: &'a [Task], input: &'a str) -> Element<'a, Message> {
    let pending = tasks.iter().filter(|t| !t.done).count();

    let header = row(vec![
        text("Today")
            .size(20)
            .style(iced_theme::Text::Color(p.text))
            .into(),
        Space::with_width(10).into(),
        text(format!("· {} remaining", pending))
            .size(14)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
    ])
    .align_items(Alignment::End);

    let items: Vec<Element<Message>> = tasks
        .iter()
        .map(|task| {
            let check = button(
                text(if task.done { "✓" } else { " " })
                    .size(12)
                    .style(iced_theme::Text::Color(if task.done { p.bg } else { p.subtext })),
            )
            .padding([4, 8])
            .style(iced_theme::Button::Custom(Box::new(TaskCheckBtn { p, done: task.done })))
            .on_press(Message::TaskToggle(task.id));

            let label = text(&task.text)
                .size(15)
                .style(iced_theme::Text::Color(if task.done { p.subtext } else { p.text }));

            let del = button(text("✕").size(11))
                .padding([4, 7])
                .style(iced_theme::Button::Custom(Box::new(DeleteBtn(p))))
                .on_press(Message::TaskDelete(task.id));

            container(
                row(vec![
                    check.into(),
                    container(label).padding([0, 12]).width(Length::Fill).into(),
                    del.into(),
                ])
                .align_items(Alignment::Center),
            )
            .padding([6, 4])
            .into()
        })
        .collect();

    let list = scrollable(column(items).spacing(2).padding([4, 0])).height(Length::Fill);

    let add_input = text_input("add a task...", input)
        .on_input(Message::TaskInputChanged)
        .on_submit(Message::TaskAdd)
        .padding([10, 14])
        .size(14)
        .style(iced_theme::TextInput::Custom(Box::new(TaskInput(p))));

    let inner = column(vec![
        header.into(),
        Space::with_height(16).into(),
        list.into(),
        Space::with_height(12).into(),
        add_input.into(),
    ])
    .padding([24, 28]);

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced_theme::Container::Custom(Box::new(Flat)))
        .into()
}

// ── Activity Tab ──────────────────────────────────────────────────────────

fn heatmap_view<'a>(p: Palette, heatmap: &'a Heatmap) -> Element<'a, Message> {
    let today = chrono::Local::now().date_naive();
    let today_dow = today.weekday().num_days_from_monday() as i64;
    let week_start = today - chrono::Duration::days(today_dow);
    let grid_start = week_start - chrono::Duration::weeks(15);

    let day_labels = ["M", "T", "W", "T", "F", "S", "S"];

    let grid_rows: Vec<Element<Message>> = (0i64..7)
        .map(|day| {
            let mut cells: Vec<Element<Message>> = vec![text(day_labels[day as usize])
                .size(11)
                .style(iced_theme::Text::Color(p.subtext))
                .width(Length::Fixed(16.0))
                .into()];

            for week in 0i64..16 {
                let date = grid_start + chrono::Duration::days(week * 7 + day);
                let color = if date > today {
                    iced::Color { a: 0.07, ..p.surface2 }
                } else {
                    theme::heat_color(heatmap.get(date), p)
                };

                cells.push(
                    container(Space::new(Length::Fixed(16.0), Length::Fixed(16.0)))
                        .style(iced_theme::Container::Custom(Box::new(HeatCell(color))))
                        .into(),
                );
            }

            row(cells).spacing(4).align_items(Alignment::Center).into()
        })
        .collect();

    let total_mins: u32 = heatmap.data.values().sum();
    let total_sessions = total_mins / 25;

    let header = row(vec![
        text("Activity")
            .size(20)
            .style(iced_theme::Text::Color(p.text))
            .into(),
        Space::with_width(Length::Fill).into(),
        text(format!("{} sessions total", total_sessions))
            .size(12)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
    ])
    .align_items(Alignment::Center);

    let legend = row(vec![
        text("less")
            .size(11)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
        Space::with_width(6).into(),
        legend_cell(0, p),
        legend_cell(15, p),
        legend_cell(40, p),
        legend_cell(70, p),
        legend_cell(120, p),
        Space::with_width(6).into(),
        text("more")
            .size(11)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
    ])
    .spacing(3)
    .align_items(Alignment::Center);

    let inner = column(vec![
        header.into(),
        Space::with_height(24).into(),
        column(grid_rows).spacing(4).into(),
        Space::with_height(16).into(),
        legend.into(),
    ])
    .padding([24, 28]);

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced_theme::Container::Custom(Box::new(Flat)))
        .into()
}

fn legend_cell(mins: u32, p: Palette) -> Element<'static, Message> {
    container(Space::new(Length::Fixed(14.0), Length::Fixed(14.0)))
        .style(iced_theme::Container::Custom(Box::new(HeatCell(theme::heat_color(mins, p)))))
        .into()
}

// ── Entry Point ───────────────────────────────────────────────────────────

const NOTO_SANS: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");

fn main() -> iced::Result {
    App::run(Settings {
        fonts: vec![NOTO_SANS.into()],
        default_font: iced::Font {
            family: iced::font::Family::Name("Noto Sans"),
            ..iced::Font::DEFAULT
        },
        window: window::Settings {
            size: iced::Size::new(780.0, 540.0),
            min_size: Some(iced::Size::new(620.0, 440.0)),
            decorations: false,
            transparent: true,
            ..Default::default()
        },
        ..Default::default()
    })
}
