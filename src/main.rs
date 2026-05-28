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
    Palette, PinBtn, ProgressStyle, TaskCheckBtn, TaskInput, TimeOfDay,
};

const APP_NAME: &str = "focus";

// ── Navigation ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Timer,
    Tasks,
    Heatmap,
}

impl Tab {
    fn prev(self) -> Self {
        match self {
            Tab::Timer => Tab::Heatmap,
            Tab::Tasks => Tab::Timer,
            Tab::Heatmap => Tab::Tasks,
        }
    }
    fn next(self) -> Self {
        match self {
            Tab::Timer => Tab::Tasks,
            Tab::Tasks => Tab::Heatmap,
            Tab::Heatmap => Tab::Timer,
        }
    }
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
    hide_in_ticks: u8,
    hover_left: bool,
    hover_right: bool,
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
    HoverLeft(bool),
    HoverRight(bool),
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
                hover_left: false,
                hover_right: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String { APP_NAME.into() }

    fn theme(&self) -> Theme { self.tod.iced_theme() }

    fn style(&self) -> iced_theme::Application {
        iced_theme::Application::Custom(Box::new(AppBg(self.tod.palette())))
    }

    fn subscription(&self) -> Subscription<Message> {
        use std::time::Duration;
        let needs_tick = self.timer.running || self.hide_in_ticks > 0;
        let tick = if needs_tick {
            time::every(Duration::from_secs(1)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        };
        let clock = time::every(Duration::from_secs(60)).map(|_| Message::RefreshTime);
        let mouse_events = iced::event::listen_with(|event, _status| match event {
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
            Message::MouseMoved(pos) => {
                if pos.y < 40.0 {
                    self.hide_in_ticks = 6;
                }
            }
            Message::MouseLeft => {
                self.hide_in_ticks = 0;
                self.hover_left = false;
                self.hover_right = false;
            }
            Message::HoverLeft(v) => self.hover_left = v,
            Message::HoverRight(v) => self.hover_right = v,
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

        let content_row = row(vec![
            mouse_area(nav_arrow(p, "‹", self.hover_left))
                .on_enter(Message::HoverLeft(true))
                .on_exit(Message::HoverLeft(false))
                .on_press(Message::TabSelected(self.active_tab.prev()))
                .into(),
            content,
            mouse_area(nav_arrow(p, "›", self.hover_right))
                .on_enter(Message::HoverRight(true))
                .on_exit(Message::HoverRight(false))
                .on_press(Message::TabSelected(self.active_tab.next()))
                .into(),
        ])
        .height(Length::Fill);

        let body = column(vec![
            top_bar(p, show_controls, self.always_on_top),
            content_row.into(),
            page_dots(p, self.active_tab),
        ])
        .height(Length::Fill);

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
// 30px strip at the top. Time badge always visible right-aligned.
// Pin + drag + close appear only while the mouse is near the top (hide_in_ticks > 0).

fn top_bar(p: Palette, show_controls: bool, always_on_top: bool) -> Element<'static, Message> {
    let now = chrono::Local::now();
    let time_str = format!("{:02}:{:02}", now.hour(), now.minute());

    // "focus  HH:MM" badge — always visible
    let make_badge = |time: String| -> Element<'static, Message> {
        row(vec![
            text(APP_NAME)
                .size(10)
                .style(iced_theme::Text::Color(p.accent))
                .into(),
            Space::with_width(6).into(),
            text(time)
                .size(10)
                .style(iced_theme::Text::Color(p.subtext))
                .into(),
        ])
        .align_items(Alignment::Center)
        .into()
    };

    if !show_controls {
        return mouse_area(
            container(
                row(vec![
                    Space::with_width(Length::Fill).into(),
                    make_badge(time_str),
                    Space::with_width(10).into(),
                ])
                .align_items(Alignment::Center)
                .height(Length::Fixed(30.0)),
            )
            .width(Length::Fill),
        )
        .on_press(Message::TitleBarDrag)
        .into();
    }

    let pin = button(
        text("⊤")
            .size(11)
            .style(iced_theme::Text::Color(
                if always_on_top { p.accent } else { p.subtext },
            )),
    )
    .padding([0, 10])
    .height(Length::Fixed(30.0))
    .style(iced_theme::Button::Custom(Box::new(PinBtn { p, active: always_on_top })))
    .on_press(Message::ToggleAlwaysOnTop);

    let drag_zone = mouse_area(
        container(Space::new(Length::Fill, Length::Fixed(30.0))),
    )
    .on_press(Message::TitleBarDrag);

    let close = button(
        text("✕")
            .size(9)
            .style(iced_theme::Text::Color(p.subtext)),
    )
    .padding([0, 12])
    .height(Length::Fixed(30.0))
    .style(iced_theme::Button::Custom(Box::new(CloseBtn(p))))
    .on_press(Message::WindowClose);

    container(
        row(vec![
            pin.into(),
            drag_zone.into(),
            make_badge(time_str),
            Space::with_width(4).into(),
            close.into(),
        ])
        .align_items(Alignment::Center)
        .height(Length::Fixed(30.0)),
    )
    .width(Length::Fill)
    .style(iced_theme::Container::Custom(Box::new(Flat)))
    .into()
}

// ── Nav Arrows ────────────────────────────────────────────────────────────
//
// Transparent 30px-wide strips on either side of the content. The arrow glyph
// fades in (alpha 0→0.7) when the cursor enters the zone.

fn nav_arrow(p: Palette, symbol: &'static str, visible: bool) -> Element<'static, Message> {
    let alpha: f32 = if visible { 0.7 } else { 0.0 };
    container(
        text(symbol)
            .size(20)
            .style(iced_theme::Text::Color(Color { a: alpha, ..p.text })),
    )
    .width(Length::Fixed(30.0))
    .height(Length::Fill)
    .center_x()
    .center_y()
    .into()
}

// ── Page Dots ─────────────────────────────────────────────────────────────

fn page_dots(p: Palette, active: Tab) -> Element<'static, Message> {
    let tabs = [Tab::Timer, Tab::Tasks, Tab::Heatmap];
    let dots: Vec<Element<Message>> = tabs
        .iter()
        .map(|&tab| {
            let color = if tab == active {
                p.accent
            } else {
                Color { a: 0.3, ..p.subtext }
            };
            container(Space::new(Length::Fixed(5.0), Length::Fixed(5.0)))
                .style(iced_theme::Container::Custom(Box::new(DotCell(color))))
                .into()
        })
        .collect();

    container(row(dots).spacing(5).align_items(Alignment::Center))
        .center_x()
        .width(Length::Fill)
        .padding([0, 0, 10, 0])
        .into()
}

// ── Timer View ────────────────────────────────────────────────────────────

fn timer_view(p: Palette, timer: &Pomodoro) -> Element<Message> {
    let phase = text(timer.phase.label())
        .size(10)
        .style(iced_theme::Text::Color(p.subtext));

    let digits = text(timer.format())
        .font(iced::Font::MONOSPACE)
        .size(52)
        .style(iced_theme::Text::Color(p.text));

    let bar = progress_bar(0.0..=1.0, timer.progress())
        .height(Length::Fixed(3.0))
        .style(iced_theme::ProgressBar::Custom(Box::new(ProgressStyle(p))));

    let cycle_pos = (timer.done % 4) as usize;
    let dots: Vec<Element<Message>> = (0..4)
        .map(|i| {
            let color = if i < cycle_pos { p.accent } else { p.surface2 };
            container(Space::new(Length::Fixed(7.0), Length::Fixed(7.0)))
                .style(iced_theme::Container::Custom(Box::new(DotCell(color))))
                .into()
        })
        .collect();

    let dot_row = row(dots).spacing(6).align_items(Alignment::Center);

    let toggle_label = if timer.running { "⏸  Pause" } else { "▶  Start" };
    let toggle = button(text(toggle_label).size(12))
        .padding([7, 20])
        .style(iced_theme::Button::Custom(Box::new(AccentBtn(p))))
        .on_press(Message::TimerToggle);

    let reset = button(text("↺").size(14))
        .padding([7, 12])
        .style(iced_theme::Button::Custom(Box::new(GhostBtn(p))))
        .on_press(Message::TimerReset);

    let skip = button(text("⏭").size(14))
        .padding([7, 12])
        .style(iced_theme::Button::Custom(Box::new(GhostBtn(p))))
        .on_press(Message::TimerSkip);

    let controls = row(vec![toggle.into(), reset.into(), skip.into()])
        .spacing(8)
        .align_items(Alignment::Center);

    let bar_wrapper = container(bar).width(Length::Fixed(180.0));

    let inner = column(vec![
        phase.into(),
        Space::with_height(2).into(),
        digits.into(),
        Space::with_height(10).into(),
        bar_wrapper.into(),
        Space::with_height(12).into(),
        dot_row.into(),
        Space::with_height(16).into(),
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

// ── Tasks View ────────────────────────────────────────────────────────────

fn tasks_view<'a>(p: Palette, tasks: &'a [Task], input: &'a str) -> Element<'a, Message> {
    let pending = tasks.iter().filter(|t| !t.done).count();

    let header = row(vec![
        text("today")
            .size(13)
            .style(iced_theme::Text::Color(p.text))
            .into(),
        Space::with_width(6).into(),
        text(format!("· {}", pending))
            .size(11)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
    ])
    .align_items(Alignment::End);

    let items: Vec<Element<Message>> = tasks
        .iter()
        .map(|task| {
            let check = button(
                text(if task.done { "✓" } else { " " })
                    .size(10)
                    .style(iced_theme::Text::Color(
                        if task.done { p.bg } else { p.subtext },
                    )),
            )
            .padding([3, 6])
            .style(iced_theme::Button::Custom(Box::new(TaskCheckBtn {
                p,
                done: task.done,
            })))
            .on_press(Message::TaskToggle(task.id));

            let label = text(&task.text)
                .size(12)
                .style(iced_theme::Text::Color(
                    if task.done { p.subtext } else { p.text },
                ));

            let del = button(text("✕").size(9))
                .padding([3, 5])
                .style(iced_theme::Button::Custom(Box::new(DeleteBtn(p))))
                .on_press(Message::TaskDelete(task.id));

            container(
                row(vec![
                    check.into(),
                    container(label).padding([0, 8]).width(Length::Fill).into(),
                    del.into(),
                ])
                .align_items(Alignment::Center),
            )
            .padding([4, 2])
            .into()
        })
        .collect();

    let list = scrollable(column(items).spacing(1).padding([2, 0])).height(Length::Fill);

    let add_input = text_input("add a task...", input)
        .on_input(Message::TaskInputChanged)
        .on_submit(Message::TaskAdd)
        .padding([7, 10])
        .size(12)
        .style(iced_theme::TextInput::Custom(Box::new(TaskInput(p))));

    let inner = column(vec![
        header.into(),
        Space::with_height(8).into(),
        list.into(),
        Space::with_height(6).into(),
        add_input.into(),
    ])
    .padding([14, 14]);

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced_theme::Container::Custom(Box::new(Flat)))
        .into()
}

// ── Heatmap View ──────────────────────────────────────────────────────────
//
// Shows 16 weeks × 7 days at 10px cells to fit the mini window width.

fn heatmap_view<'a>(p: Palette, heatmap: &'a Heatmap) -> Element<'a, Message> {
    let today = chrono::Local::now().date_naive();
    let today_dow = today.weekday().num_days_from_monday() as i64;
    let week_start = today - chrono::Duration::days(today_dow);
    let grid_start = week_start - chrono::Duration::weeks(15);

    let day_labels = ["M", "T", "W", "T", "F", "S", "S"];

    let grid_rows: Vec<Element<Message>> = (0i64..7)
        .map(|day| {
            let mut cells: Vec<Element<Message>> = vec![text(day_labels[day as usize])
                .size(9)
                .style(iced_theme::Text::Color(p.subtext))
                .width(Length::Fixed(12.0))
                .into()];

            for week in 0i64..16 {
                let date = grid_start + chrono::Duration::days(week * 7 + day);
                let color = if date > today {
                    iced::Color { a: 0.07, ..p.surface2 }
                } else {
                    theme::heat_color(heatmap.get(date), p)
                };
                cells.push(
                    container(Space::new(Length::Fixed(10.0), Length::Fixed(10.0)))
                        .style(iced_theme::Container::Custom(Box::new(HeatCell(color))))
                        .into(),
                );
            }

            row(cells).spacing(3).align_items(Alignment::Center).into()
        })
        .collect();

    let total_mins: u32 = heatmap.data.values().sum();
    let total_sessions = total_mins / 25;

    let header = row(vec![
        text("activity")
            .size(13)
            .style(iced_theme::Text::Color(p.text))
            .into(),
        Space::with_width(Length::Fill).into(),
        text(format!("{} sessions", total_sessions))
            .size(10)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
    ])
    .align_items(Alignment::Center);

    let inner = column(vec![
        header.into(),
        Space::with_height(12).into(),
        column(grid_rows).spacing(3).into(),
    ])
    .padding([14, 10]);

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced_theme::Container::Custom(Box::new(Flat)))
        .into()
}

// ── App Icon ──────────────────────────────────────────────────────────────
//
// 32×32 clock-face drawn from raw RGBA: dark fill, blue ring, light hands.

fn create_app_icon() -> Option<window::Icon> {
    const S: u32 = 32;
    let n = S as usize;
    let mut rgba = vec![0u8; n * n * 4];
    let c = 15.5_f32;

    for y in 0..n {
        for x in 0..n {
            let dx = x as f32 - c;
            let dy = y as f32 - c;
            let r = (dx * dx + dy * dy).sqrt();
            let i = (y * n + x) * 4;

            if r > 14.5 {
                // transparent outside circle
                continue;
            }

            if r >= 11.0 {
                // blue ring
                rgba[i]   = 96;
                rgba[i+1] = 165;
                rgba[i+2] = 250;
                rgba[i+3] = 255;
            } else {
                // dark interior
                rgba[i]   = 15;
                rgba[i+1] = 17;
                rgba[i+2] = 23;
                rgba[i+3] = 255;

                // clock hands: 12-o'clock (up) and 3-o'clock (right)
                let hand_up    = dx.abs() < 1.2 && dy < 0.0 && dy > -8.5;
                let hand_right = dy.abs() < 1.2 && dx > 0.0 && dx < 6.5;
                let center_dot = r < 1.8;
                if hand_up || hand_right || center_dot {
                    rgba[i]   = 200;
                    rgba[i+1] = 225;
                    rgba[i+2] = 245;
                    rgba[i+3] = 255;
                }
            }
        }
    }

    window::icon::from_rgba(rgba, S, S).ok()
}

// ── Entry Point ───────────────────────────────────────────────────────────

fn main() -> iced::Result {
    let mut fonts: Vec<std::borrow::Cow<'static, [u8]>> = vec![];
    if let Ok(bytes) = std::fs::read("C:\\Windows\\Fonts\\seguisym.ttf") {
        fonts.push(std::borrow::Cow::Owned(bytes));
    }

    App::run(Settings {
        fonts,
        default_font: iced::Font {
            family: iced::font::Family::Name("Segoe UI Symbol"),
            ..iced::Font::DEFAULT
        },
        window: window::Settings {
            size: iced::Size::new(320.0, 260.0),
            resizable: false,
            decorations: false,
            transparent: true,
            icon: create_app_icon(),
            ..Default::default()
        },
        ..Default::default()
    })
}
