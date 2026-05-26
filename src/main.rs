mod data;
mod storage;
mod theme;

use chrono::{Datelike, Timelike};
use data::{Heatmap, Pomodoro, Task};
use iced::theme as iced_theme;
use iced::widget::{
    button, column, container, progress_bar, row, scrollable, text, text_input, Space,
};
use iced::{
    Alignment, Application, Command, Element, Length, Settings, Subscription, Theme,
};
use iced::time;
use storage::SaveData;
use theme::{
    AccentBtn, AppBg, DeleteBtn, DotCell, Flat, GhostBtn, HeatCell,
    Palette, ProgressStyle, Surface, TaskCheckBtn, TaskInput, TimeOfDay,
};

// ── State ─────────────────────────────────────────────────────────────────

struct App {
    tod: TimeOfDay,
    tasks: Vec<Task>,
    task_input: String,
    next_id: u64,
    timer: Pomodoro,
    heatmap: Heatmap,
}

#[derive(Debug, Clone)]
enum Message {
    TimerTick,
    TimerToggle,
    TimerReset,
    TimerSkip,
    TaskInputChanged(String),
    TaskAdd,
    TaskToggle(u64),
    TaskDelete(u64),
    RefreshTime,
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
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "study buddy".into()
    }

    fn theme(&self) -> Theme {
        self.tod.iced_theme()
    }

    fn style(&self) -> iced_theme::Application {
        iced_theme::Application::Custom(Box::new(AppBg(self.tod.palette())))
    }

    fn subscription(&self) -> Subscription<Message> {
        use std::time::Duration;
        let tick = if self.timer.running {
            time::every(Duration::from_secs(1)).map(|_| Message::TimerTick)
        } else {
            Subscription::none()
        };
        let clock = time::every(Duration::from_secs(60)).map(|_| Message::RefreshTime);
        Subscription::batch(vec![tick, clock])
    }

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::TimerTick => {
                if self.timer.tick() {
                    self.heatmap.add(25);
                    self.persist();
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
            Message::RefreshTime => {
                self.tod = TimeOfDay::now();
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let p = self.tod.palette();

        let content = column(vec![
            header_view(p, &self.tod),
            row(vec![
                tasks_panel(p, &self.tasks, &self.task_input),
                timer_panel(p, &self.timer),
            ])
            .spacing(12)
            .height(Length::Fill)
            .into(),
            heatmap_panel(p, &self.heatmap),
        ])
        .spacing(12)
        .padding(16);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(iced_theme::Container::Custom(Box::new(Flat)))
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

// ── View Helpers ──────────────────────────────────────────────────────────

fn header_view<'a>(p: Palette, _tod: &TimeOfDay) -> Element<'a, Message> {
    let now = chrono::Local::now();
    let time_str = format!("{:02}:{:02}", now.hour(), now.minute());

    let inner = row(vec![
        text("study buddy")
            .size(17)
            .style(iced_theme::Text::Color(p.text))
            .into(),
        Space::with_width(Length::Fill).into(),
        text(p.name)
            .size(12)
            .style(iced_theme::Text::Color(p.accent))
            .into(),
        text("  ").into(),
        text(time_str)
            .size(12)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
    ])
    .align_items(Alignment::Center);

    container(inner)
        .padding([9, 14])
        .width(Length::Fill)
        .style(iced_theme::Container::Custom(Box::new(Surface(p, false))))
        .into()
}

fn tasks_panel<'a>(p: Palette, tasks: &'a [Task], input: &'a str) -> Element<'a, Message> {
    let pending = tasks.iter().filter(|t| !t.done).count();
    let header_label = format!("Today  ·  {} remaining", pending);

    let task_items: Vec<Element<Message>> = tasks
        .iter()
        .map(|task| {
            let check = button(
                text(if task.done { "✓" } else { " " })
                    .size(11)
                    .style(iced_theme::Text::Color(if task.done { p.bg } else { p.subtext })),
            )
            .padding([3, 6])
            .style(iced_theme::Button::Custom(Box::new(TaskCheckBtn { p, done: task.done })))
            .on_press(Message::TaskToggle(task.id));

            let label = text(&task.text)
                .size(14)
                .style(iced_theme::Text::Color(if task.done { p.subtext } else { p.text }));

            let del = button(text("✕").size(11))
                .padding([3, 6])
                .style(iced_theme::Button::Custom(Box::new(DeleteBtn(p))))
                .on_press(Message::TaskDelete(task.id));

            row(vec![
                check.into(),
                container(label).padding([0, 8]).width(Length::Fill).into(),
                del.into(),
            ])
            .align_items(Alignment::Center)
            .into()
        })
        .collect();

    let task_list = scrollable(
        column(task_items)
            .spacing(6)
            .padding([4, 0]),
    )
    .height(Length::Fill);

    let add_input = text_input("add a task...", input)
        .on_input(Message::TaskInputChanged)
        .on_submit(Message::TaskAdd)
        .padding([8, 10])
        .size(13)
        .style(iced_theme::TextInput::Custom(Box::new(TaskInput(p))));

    let inner = column(vec![
        text(header_label)
            .size(13)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
        task_list.into(),
        add_input.into(),
    ])
    .spacing(10)
    .padding(14);

    container(inner)
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .style(iced_theme::Container::Custom(Box::new(Surface(p, false))))
        .into()
}

fn timer_panel<'a>(p: Palette, timer: &'a Pomodoro) -> Element<'a, Message> {
    let phase_label = text(timer.phase.label())
        .size(12)
        .style(iced_theme::Text::Color(p.subtext));

    let digits = text(timer.format())
        .font(iced::Font::MONOSPACE)
        .size(52)
        .style(iced_theme::Text::Color(p.text));

    let bar = progress_bar(0.0..=1.0, timer.progress())
        .height(Length::Fixed(4.0))
        .style(iced_theme::ProgressBar::Custom(Box::new(ProgressStyle(p))));

    let cycle_pos = (timer.done % 4) as usize;
    let dots: Vec<Element<Message>> = (0..4)
        .map(|i| {
            let color = if i < cycle_pos { p.accent } else { p.surface2 };
            container(Space::new(Length::Fixed(8.0), Length::Fixed(8.0)))
                .style(iced_theme::Container::Custom(Box::new(DotCell(color))))
                .into()
        })
        .collect();

    let session_info = row(vec![
        row(dots).spacing(6).into(),
        Space::with_width(8).into(),
        text(format!("session {}/4", cycle_pos + 1))
            .size(12)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
    ])
    .align_items(Alignment::Center);

    let toggle_label = if timer.running { "⏸  Pause" } else { "▶  Start" };
    let toggle_btn = button(text(toggle_label).size(13))
        .padding([9, 20])
        .style(iced_theme::Button::Custom(Box::new(AccentBtn(p))))
        .on_press(Message::TimerToggle);

    let reset_btn = button(text("↺").size(15))
        .padding([9, 14])
        .style(iced_theme::Button::Custom(Box::new(GhostBtn(p))))
        .on_press(Message::TimerReset);

    let skip_btn = button(text("⏭").size(15))
        .padding([9, 14])
        .style(iced_theme::Button::Custom(Box::new(GhostBtn(p))))
        .on_press(Message::TimerSkip);

    let controls = row(vec![toggle_btn.into(), reset_btn.into(), skip_btn.into()])
        .spacing(8)
        .align_items(Alignment::Center);

    let inner = column(vec![
        phase_label.into(),
        digits.into(),
        container(bar).width(Length::Fill).padding([0, 0]).into(),
        session_info.into(),
        controls.into(),
    ])
    .align_items(Alignment::Center)
    .spacing(14)
    .padding(18);

    container(inner)
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(iced_theme::Container::Custom(Box::new(Surface(p, false))))
        .into()
}

fn heatmap_panel<'a>(p: Palette, heatmap: &'a Heatmap) -> Element<'a, Message> {
    let today = chrono::Local::now().date_naive();
    let today_dow = today.weekday().num_days_from_monday() as i64;
    let week_start = today - chrono::Duration::days(today_dow);
    let grid_start = week_start - chrono::Duration::weeks(15);

    let day_labels = ["M", "T", "W", "T", "F", "S", "S"];

    let grid_rows: Vec<Element<Message>> = (0i64..7)
        .map(|day| {
            let mut cells: Vec<Element<Message>> = Vec::new();

            cells.push(
                text(day_labels[day as usize])
                    .size(10)
                    .style(iced_theme::Text::Color(p.subtext))
                    .width(Length::Fixed(12.0))
                    .into(),
            );

            for week in 0i64..16 {
                let date = grid_start + chrono::Duration::days(week * 7 + day);
                let color = if date > today {
                    iced::Color { a: 0.08, ..p.surface2 }
                } else {
                    theme::heat_color(heatmap.get(date), p)
                };
                cells.push(
                    container(Space::new(Length::Fixed(12.0), Length::Fixed(12.0)))
                        .style(iced_theme::Container::Custom(Box::new(HeatCell(color))))
                        .into(),
                );
            }

            row(cells).spacing(3).align_items(Alignment::Center).into()
        })
        .collect();

    let grid = column(grid_rows).spacing(3);

    let header = row(vec![
        text("Activity")
            .size(13)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
        Space::with_width(Length::Fill).into(),
        text("16 weeks")
            .size(11)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
    ])
    .align_items(Alignment::Center);

    let inner = column(vec![header.into(), grid.into()])
        .spacing(10)
        .padding(14);

    container(inner)
        .width(Length::Fill)
        .style(iced_theme::Container::Custom(Box::new(Surface(p, false))))
        .into()
}

// ── Entry Point ───────────────────────────────────────────────────────────

fn main() -> iced::Result {
    App::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(780.0, 560.0),
            min_size: Some(iced::Size::new(620.0, 480.0)),
            ..Default::default()
        },
        ..Default::default()
    })
}
