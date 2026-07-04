#![windows_subsystem = "windows"]

mod data;
mod platform;
mod sound;
mod storage;
mod theme;

use chrono::{Datelike, NaiveDate, Timelike};
use data::{Heatmap, Phase, Pomodoro, SessionConfig, SoundOption, Task, TimerPreset};
use iced::theme as iced_theme;
use iced::widget::{
    button, column, container, mouse_area, progress_bar, row, scrollable, text, text_input, Space,
};
use iced::{
    Alignment, Application, Color, Command, Element, Length, Settings, Subscription, Theme,
};
use iced::futures::SinkExt;
use iced::{subscription, time, window};
use sound::AudioPlayer;
use storage::SaveData;
use theme::{
    AccentBtn, AppBg, CloseBtn, DeleteBtn, DotCell, DragRow, Flat, GhostBtn, HeatCell,
    OuterBorder, Palette, PinBtn, ProgressStyle, SettingsBtn, TaskCheckBtn,
    TaskInput, TimeOfDay,
};

const APP_NAME: &str = "focus";
const CURRENT_VERSION: &str = "0.1.1";

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
    hovered_heat_date: Option<NaiveDate>,
    hovered_task_id: Option<u64>,
    drag_task_id: Option<u64>,
    drag_target_idx: usize,
    audio: Option<AudioPlayer>,
    // task editing
    editing_task_id: Option<u64>,
    edit_text: String,
    last_task_press: Option<(u64, std::time::Instant)>,
    // settings
    show_settings: bool,
    custom_work_input: String,
    custom_short_input: String,
    custom_long_input: String,
    sound_option: SoundOption,
    // shortcuts panel
    show_shortcuts: bool,
    // system
    autostart: bool,
    // changelog
    show_changelog: bool,
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
    TaskClearDone,
    TaskDragStart { id: u64, idx: usize },
    TaskHovered(Option<u64>),
    TaskPressed(u64),
    TaskEditChanged(String),
    TaskEditSubmit,
    MouseReleased,
    RefreshTime,
    TabSelected(Tab),
    TitleBarDrag,
    WindowClose,
    ToggleAlwaysOnTop,
    MouseMoved(iced::Point),
    MouseLeft,
    HoverLeft(bool),
    HoverRight(bool),
    HeatCellEntered(NaiveDate),
    HeatCellLeft,
    ToggleSettings,
    SetPreset(TimerPreset),
    CustomWorkChanged(String),
    CustomShortChanged(String),
    CustomLongChanged(String),
    ApplyCustomPreset,
    SetSound(SoundOption),
    // tray
    HideToTray,
    TrayShow,
    TrayQuit,
    // global hotkey
    GlobalHotkeyFired,
    // keyboard
    KeySpace,
    KeyR,
    KeyS,
    KeyLeft,
    KeyRight,
    KeyEscape,
    // panels
    ToggleShortcuts,
    ToggleAutostart,
    DismissChangelog,
    // tray actions
    TrayCheckUpdate,
}

// ── Application ───────────────────────────────────────────────────────────

impl Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_: ()) -> (Self, Command<Message>) {
        let s = storage::load();
        let cfg = s.session_config;
        (
            Self {
                tod: TimeOfDay::now(),
                tasks: s.tasks,
                task_input: String::new(),
                next_id: s.next_id,
                timer: Pomodoro::new(s.pomodoros_done, cfg),
                heatmap: s.heatmap,
                active_tab: Tab::Timer,
                always_on_top: false,
                hide_in_ticks: 0,
                hover_left: false,
                hover_right: false,
                hovered_heat_date: None,
                hovered_task_id: None,
                drag_task_id: None,
                drag_target_idx: 0,
                audio: AudioPlayer::new(),
                editing_task_id: None,
                edit_text: String::new(),
                last_task_press: None,
                show_settings: false,
                custom_work_input: cfg.work_mins.to_string(),
                custom_short_input: cfg.short_mins.to_string(),
                custom_long_input: cfg.long_mins.to_string(),
                sound_option: s.sound_option,
                show_shortcuts: false,
                autostart: platform::get_autostart(),
                show_changelog: s.last_seen_version != CURRENT_VERSION,
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
        let events = iced::event::listen_with(|event, status| match event {
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                Some(Message::MouseMoved(position))
            }
            iced::Event::Mouse(iced::mouse::Event::CursorLeft) => Some(Message::MouseLeft),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )) => Some(Message::MouseReleased),
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. })
                if status == iced::event::Status::Ignored =>
            {
                use iced::keyboard::key::Named;
                match &key {
                    iced::keyboard::Key::Named(Named::Space) => Some(Message::KeySpace),
                    iced::keyboard::Key::Character(c) if c.as_str() == "r" => Some(Message::KeyR),
                    iced::keyboard::Key::Character(c) if c.as_str() == "s" => Some(Message::KeyS),
                    iced::keyboard::Key::Character(c)
                        if c.as_str() == "?" || c.as_str() == "/" =>
                    {
                        Some(Message::ToggleShortcuts)
                    }
                    iced::keyboard::Key::Named(Named::ArrowLeft) => Some(Message::KeyLeft),
                    iced::keyboard::Key::Named(Named::ArrowRight) => Some(Message::KeyRight),
                    iced::keyboard::Key::Named(Named::Escape) => Some(Message::KeyEscape),
                    _ => None,
                }
            }
            _ => None,
        });

        let tray_events = subscription::channel(0xDEAD_BEEF_u64, 16, |mut tx| async move {
            loop {
                use tray_icon::{TrayIconEvent, MouseButton, MouseButtonState};
                if let Ok(ev) = TrayIconEvent::receiver().try_recv() {
                    if matches!(ev,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up, ..
                        }
                    ) {
                        tx.send(Message::TrayShow).await.ok();
                    }
                }
                if let Ok(ev) = tray_icon::menu::MenuEvent::receiver().try_recv() {
                    match ev.id.0.as_str() {
                        "show"         => { tx.send(Message::TrayShow).await.ok(); }
                        "quit"         => { tx.send(Message::TrayQuit).await.ok(); }
                        "check_update" => { tx.send(Message::TrayCheckUpdate).await.ok(); }
                        _ => {}
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });

        let hotkey_events = subscription::channel(0xCAFE_DEAD_u64, 8, |mut tx| async move {
            loop {
                if let Ok(ev) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
                    use global_hotkey::HotKeyState;
                    if ev.state == HotKeyState::Pressed
                        && platform::show_hotkey_id() == Some(ev.id)
                    {
                        tx.send(Message::GlobalHotkeyFired).await.ok();
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        });

        Subscription::batch(vec![tick, clock, events, tray_events, hotkey_events])
    }

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            Message::Tick => {
                if self.timer.running && self.timer.tick() {
                    self.chime();
                    platform::notify_work_done();
                    self.heatmap.add(self.timer.config.work_mins);
                    self.persist();
                }
                if self.hide_in_ticks > 0 {
                    self.hide_in_ticks -= 1;
                }
            }
            Message::TimerToggle => {
                self.click();
                if self.timer.phase == Phase::OpenBreak {
                    self.timer.advance();
                    self.timer.running = true;
                } else {
                    self.timer.running = !self.timer.running;
                }
            }
            Message::TimerReset => { self.click(); self.timer.reset(); }
            Message::TimerSkip  => { self.click(); self.timer.skip(); }
            Message::TaskInputChanged(s) => self.task_input = s,
            Message::TaskAdd => {
                let t = self.task_input.trim().to_string();
                if !t.is_empty() {
                    self.click();
                    self.tasks.push(Task::new(self.next_id, t));
                    self.next_id += 1;
                    self.task_input.clear();
                    self.persist();
                }
            }
            Message::TaskToggle(id) => {
                self.click();
                if self.editing_task_id == Some(id) {
                    self.editing_task_id = None;
                    self.edit_text.clear();
                }
                if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
                    t.done = !t.done;
                }
                self.persist();
            }
            Message::TaskDelete(id) => {
                self.click();
                if self.editing_task_id == Some(id) {
                    self.editing_task_id = None;
                    self.edit_text.clear();
                }
                self.tasks.retain(|t| t.id != id);
                self.persist();
            }
            Message::TaskClearDone => {
                self.click();
                self.tasks.retain(|t| !t.done);
                self.persist();
            }
            Message::TaskDragStart { id, idx } => {
                self.drag_task_id = Some(id);
                self.drag_target_idx = idx;
            }
            Message::TaskHovered(id) => self.hovered_task_id = id,
            Message::TaskPressed(id) => {
                // Commit any in-progress edit before handling click
                if let Some(edit_id) = self.editing_task_id.take() {
                    let text = self.edit_text.trim().to_string();
                    if !text.is_empty() {
                        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == edit_id) {
                            t.text = text;
                            self.persist();
                        }
                    }
                    self.edit_text.clear();
                    if edit_id == id { return Command::none(); }
                }
                let now = std::time::Instant::now();
                let is_double = self.last_task_press
                    .map(|(prev_id, prev_time)| {
                        prev_id == id && now.duration_since(prev_time).as_millis() < 400
                    })
                    .unwrap_or(false);
                if is_double {
                    if let Some(t) = self.tasks.iter().find(|t| t.id == id) {
                        self.editing_task_id = Some(id);
                        self.edit_text = t.text.clone();
                    }
                    self.last_task_press = None;
                } else {
                    self.last_task_press = Some((id, now));
                }
            }
            Message::TaskEditChanged(s) => self.edit_text = s,
            Message::TaskEditSubmit => {
                if let Some(id) = self.editing_task_id.take() {
                    let text = self.edit_text.trim().to_string();
                    if !text.is_empty() {
                        if let Some(t) = self.tasks.iter_mut().find(|t| t.id == id) {
                            t.text = text;
                        }
                        self.persist();
                    }
                }
                self.edit_text.clear();
            }
            Message::MouseReleased => {
                if let Some(drag_id) = self.drag_task_id.take() {
                    if let Some(src) = self.tasks.iter().position(|t| t.id == drag_id) {
                        let task = self.tasks.remove(src);
                        let dst = self.drag_target_idx.min(self.tasks.len());
                        self.tasks.insert(dst, task);
                        self.persist();
                    }
                }
            }
            Message::RefreshTime => self.tod = TimeOfDay::now(),
            Message::TabSelected(tab) => self.active_tab = tab,
            Message::TitleBarDrag => return window::drag(window::Id::MAIN),
            Message::WindowClose => return window::close(window::Id::MAIN),
            Message::HideToTray => { platform::hide_window(); }
            Message::ToggleAlwaysOnTop => {
                self.click();
                self.always_on_top = !self.always_on_top;
                let level = if self.always_on_top {
                    window::Level::AlwaysOnTop
                } else {
                    window::Level::Normal
                };
                return window::change_level(window::Id::MAIN, level);
            }
            Message::MouseMoved(pos) => {
                if pos.y < 40.0 { self.hide_in_ticks = 6; }
                if self.drag_task_id.is_some() && !self.tasks.is_empty() {
                    let raw = ((pos.y - 75.0) / 31.0).floor() as isize;
                    self.drag_target_idx =
                        raw.clamp(0, self.tasks.len() as isize - 1) as usize;
                }
            }
            Message::MouseLeft => {
                self.hide_in_ticks = 0;
                self.hover_left = false;
                self.hover_right = false;
                self.drag_task_id = None;
                self.hovered_task_id = None;
            }
            Message::HoverLeft(v) => self.hover_left = v,
            Message::HoverRight(v) => self.hover_right = v,
            Message::HeatCellEntered(date) => self.hovered_heat_date = Some(date),
            Message::HeatCellLeft => self.hovered_heat_date = None,
            Message::ToggleSettings => {
                self.click();
                self.show_settings = !self.show_settings;
            }
            Message::SetPreset(preset) => {
                self.click();
                let cfg = SessionConfig::from_preset(preset);
                self.custom_work_input  = cfg.work_mins.to_string();
                self.custom_short_input = cfg.short_mins.to_string();
                self.custom_long_input  = cfg.long_mins.to_string();
                self.timer.set_config(cfg);
                self.persist();
            }
            Message::CustomWorkChanged(s)  => self.custom_work_input  = s,
            Message::CustomShortChanged(s) => self.custom_short_input = s,
            Message::CustomLongChanged(s)  => self.custom_long_input  = s,
            Message::ApplyCustomPreset => {
                self.click();
                let work  = self.custom_work_input.trim().parse::<u32>().unwrap_or(25).max(1).min(180);
                let short = self.custom_short_input.trim().parse::<u32>().unwrap_or(5).max(1).min(60);
                let long  = self.custom_long_input.trim().parse::<u32>().unwrap_or(15).max(1).min(60);
                let cfg = SessionConfig { preset: TimerPreset::Custom, work_mins: work, short_mins: short, long_mins: long };
                self.custom_work_input  = work.to_string();
                self.custom_short_input = short.to_string();
                self.custom_long_input  = long.to_string();
                self.timer.set_config(cfg);
                self.persist();
            }
            Message::SetSound(opt) => {
                self.sound_option = opt;
                self.chime();
                self.persist();
            }
            // tray / hotkey
            Message::TrayShow | Message::GlobalHotkeyFired => { platform::show_window(); }
            Message::TrayQuit => return window::close(window::Id::MAIN),
            // keyboard shortcuts
            Message::KeySpace => {
                let mut m = Message::TimerToggle;
                // if settings or shortcuts are open, close them instead
                if self.show_settings || self.show_shortcuts {
                    m = Message::KeyEscape;
                }
                return self.update(m);
            }
            Message::KeyR => {
                if !self.show_settings && !self.show_shortcuts {
                    return self.update(Message::TimerReset);
                }
            }
            Message::KeyS => {
                if !self.show_settings && !self.show_shortcuts {
                    return self.update(Message::TimerSkip);
                }
            }
            Message::KeyLeft => {
                if !self.show_settings && !self.show_shortcuts {
                    return self.update(Message::TabSelected(self.active_tab.prev()));
                }
            }
            Message::KeyRight => {
                if !self.show_settings && !self.show_shortcuts {
                    return self.update(Message::TabSelected(self.active_tab.next()));
                }
            }
            Message::KeyEscape => {
                if self.show_shortcuts { self.show_shortcuts = false; }
                else if self.show_settings { self.show_settings = false; }
            }
            Message::ToggleShortcuts => {
                self.show_settings = false;
                self.show_shortcuts = !self.show_shortcuts;
            }
            Message::ToggleAutostart => {
                self.autostart = !self.autostart;
                platform::set_autostart(self.autostart);
            }
            Message::DismissChangelog => {
                self.show_changelog = false;
                self.persist();
            }
            Message::TrayCheckUpdate => {
                platform::check_for_update(CURRENT_VERSION);
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let p = self.tod.palette();
        let show_controls = self.hide_in_ticks > 0;

        let session_color: Option<Color> = if self.timer.running {
            Some(if self.timer.phase == Phase::Work { p.accent } else { p.success })
        } else {
            None
        };

        let content: Element<Message> = if self.show_changelog {
            changelog_view(p)
        } else if self.show_shortcuts {
            shortcuts_view(p)
        } else if self.show_settings {
            settings_view(
                p,
                &self.timer.config,
                &self.custom_work_input,
                &self.custom_short_input,
                &self.custom_long_input,
                self.sound_option,
                self.autostart,
            )
        } else {
            let tab_content: Element<Message> = match self.active_tab {
                Tab::Timer   => timer_view(p, &self.timer),
                Tab::Tasks   => tasks_view(
                    p,
                    &self.tasks,
                    &self.task_input,
                    self.hovered_task_id,
                    self.drag_task_id,
                    self.drag_target_idx,
                    self.editing_task_id,
                    &self.edit_text,
                ),
                Tab::Heatmap => heatmap_view(p, &self.heatmap, self.hovered_heat_date),
            };

            row(vec![
                mouse_area(nav_arrow(p, "‹", self.hover_left))
                    .on_enter(Message::HoverLeft(true))
                    .on_exit(Message::HoverLeft(false))
                    .on_press(Message::TabSelected(self.active_tab.prev()))
                    .into(),
                tab_content,
                mouse_area(nav_arrow(p, "›", self.hover_right))
                    .on_enter(Message::HoverRight(true))
                    .on_exit(Message::HoverRight(false))
                    .on_press(Message::TabSelected(self.active_tab.next()))
                    .into(),
            ])
            .height(Length::Fill)
            .into()
        };

        let body = column(vec![
            top_bar(p, show_controls, self.always_on_top, session_color, self.show_settings, self.show_shortcuts),
            content,
            page_dots(p, self.active_tab, self.show_settings || self.show_shortcuts || self.show_changelog),
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
    fn click(&self) {
        if let Some(ref a) = self.audio { a.play_click(); }
    }
    fn chime(&self) {
        if let Some(ref a) = self.audio { a.play_alert(self.sound_option); }
    }

    fn persist(&self) {
        storage::save(&SaveData {
            tasks: self.tasks.clone(),
            heatmap: self.heatmap.clone(),
            next_id: self.next_id,
            pomodoros_done: self.timer.done,
            session_config: self.timer.config,
            sound_option: self.sound_option,
            last_seen_version: CURRENT_VERSION.to_string(),
        });
    }
}

// ── Top Bar ───────────────────────────────────────────────────────────────

fn top_bar(
    p: Palette,
    show_controls: bool,
    always_on_top: bool,
    session_color: Option<Color>,
    show_settings: bool,
    show_shortcuts: bool,
) -> Element<'static, Message> {
    let now = chrono::Local::now();
    let time_str = format!("{:02}:{:02}", now.hour(), now.minute());

    let make_badge = move |time: String| -> Element<'static, Message> {
        let mut items: Vec<Element<Message>> = vec![];
        if let Some(color) = session_color {
            items.push(
                container(Space::new(Length::Fixed(6.0), Length::Fixed(6.0)))
                    .style(iced_theme::Container::Custom(Box::new(DotCell(color))))
                    .into(),
            );
            items.push(Space::with_width(5).into());
        }
        items.push(
            text(APP_NAME)
                .size(10)
                .style(iced_theme::Text::Color(p.accent))
                .into(),
        );
        items.push(Space::with_width(6).into());
        items.push(
            text(time)
                .size(10)
                .style(iced_theme::Text::Color(p.subtext))
                .into(),
        );
        row(items).align_items(Alignment::Center).into()
    };

    let help = button(
        text("?").size(11).style(iced_theme::Text::Color(
            if show_shortcuts { p.accent } else { p.subtext },
        )),
    )
    .padding([0, 8])
    .height(Length::Fixed(30.0))
    .style(iced_theme::Button::Custom(Box::new(SettingsBtn { p, active: show_shortcuts })))
    .on_press(Message::ToggleShortcuts);

    let gear = button(
        text("⚙").size(11).style(iced_theme::Text::Color(
            if show_settings { p.accent } else { p.subtext },
        )),
    )
    .padding([0, 8])
    .height(Length::Fixed(30.0))
    .style(iced_theme::Button::Custom(Box::new(SettingsBtn { p, active: show_settings })))
    .on_press(Message::ToggleSettings);

    if !show_controls {
        return mouse_area(
            container(
                row(vec![
                    Space::with_width(Length::Fill).into(),
                    make_badge(time_str),
                    help.into(),
                    gear.into(),
                    Space::with_width(4).into(),
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
        text("✕").size(9).style(iced_theme::Text::Color(p.subtext)),
    )
    .padding([0, 12])
    .height(Length::Fixed(30.0))
    .style(iced_theme::Button::Custom(Box::new(CloseBtn(p))))
    .on_press(Message::HideToTray);

    container(
        row(vec![
            pin.into(),
            drag_zone.into(),
            make_badge(time_str),
            help.into(),
            gear.into(),
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

fn page_dots(p: Palette, active: Tab, show_settings: bool) -> Element<'static, Message> {
    if show_settings {
        return container(Space::new(Length::Fill, Length::Fixed(20.0)))
            .width(Length::Fill)
            .padding([0, 0, 10, 0])
            .into();
    }

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

    let toggle_label = if timer.phase == Phase::OpenBreak {
        "▶  Next Session"
    } else if timer.running {
        "⏸  Pause"
    } else {
        "▶  Start"
    };

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

fn tasks_view<'a>(
    p: Palette,
    tasks: &'a [Task],
    input: &'a str,
    hovered_task_id: Option<u64>,
    drag_task_id: Option<u64>,
    drag_target_idx: usize,
    editing_task_id: Option<u64>,
    edit_text: &'a str,
) -> Element<'a, Message> {
    let pending = tasks.iter().filter(|t| !t.done).count();
    let done_count = tasks.len() - pending;

    let mut clear_btn = button(
        text("clear done").size(10).style(iced_theme::Text::Color(p.subtext)),
    )
    .padding([2, 6])
    .style(iced_theme::Button::Custom(Box::new(GhostBtn(p))));
    if done_count > 0 {
        clear_btn = clear_btn.on_press(Message::TaskClearDone);
    }

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
        Space::with_width(Length::Fill).into(),
        clear_btn.into(),
    ])
    .align_items(Alignment::Center);

    let display: Vec<&Task> = if let Some(drag_id) = drag_task_id {
        if let Some(src) = tasks.iter().position(|t| t.id == drag_id) {
            let mut order: Vec<&Task> = tasks.iter().collect();
            let item = order.remove(src);
            let dst = drag_target_idx.min(order.len());
            order.insert(dst, item);
            order
        } else {
            tasks.iter().collect()
        }
    } else {
        tasks.iter().collect()
    };

    let items: Vec<Element<Message>> = display
        .iter()
        .enumerate()
        .map(|(i, &task)| {
            let is_dragging = drag_task_id == Some(task.id);
            let is_hovered  = hovered_task_id == Some(task.id);
            let is_editing  = editing_task_id == Some(task.id);

            let grip_alpha: f32 =
                if is_dragging { 0.7 } else if is_hovered { 0.45 } else { 0.0 };
            let grip = mouse_area(
                container(
                    text("≡")
                        .size(13)
                        .style(iced_theme::Text::Color(Color { a: grip_alpha, ..p.subtext })),
                )
                .width(Length::Fixed(16.0))
                .center_x()
                .center_y(),
            )
            .on_press(Message::TaskDragStart { id: task.id, idx: i });

            let check = button(
                text(if task.done { "✓" } else { " " })
                    .size(10)
                    .style(iced_theme::Text::Color(
                        if task.done { p.bg } else { p.subtext },
                    )),
            )
            .padding([3, 6])
            .style(iced_theme::Button::Custom(Box::new(TaskCheckBtn { p, done: task.done })))
            .on_press(Message::TaskToggle(task.id));

            let label_area: Element<Message> = if is_editing {
                text_input("", edit_text)
                    .on_input(Message::TaskEditChanged)
                    .on_submit(Message::TaskEditSubmit)
                    .padding([3, 6])
                    .size(12)
                    .style(iced_theme::TextInput::Custom(Box::new(TaskInput(p))))
                    .into()
            } else {
                let label = text(&task.text).size(12).style(iced_theme::Text::Color(
                    if task.done { p.subtext } else { p.text },
                ));
                mouse_area(
                    container(label).padding([0, 8]).width(Length::Fill),
                )
                .on_press(Message::TaskPressed(task.id))
                .into()
            };

            let del = button(text("✕").size(9))
                .padding([3, 5])
                .style(iced_theme::Button::Custom(Box::new(DeleteBtn(p))))
                .on_press(Message::TaskDelete(task.id));

            let row_style = if is_dragging {
                iced_theme::Container::Custom(Box::new(DragRow(p)))
            } else {
                iced_theme::Container::Custom(Box::new(Flat))
            };

            mouse_area(
                container(
                    row(vec![
                        grip.into(),
                        check.into(),
                        label_area,
                        del.into(),
                        Space::with_width(8).into(),
                    ])
                    .align_items(Alignment::Center),
                )
                .padding([4, 2])
                .style(row_style),
            )
            .on_enter(Message::TaskHovered(Some(task.id)))
            .on_exit(Message::TaskHovered(None))
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

// ── Settings View ─────────────────────────────────────────────────────────

fn settings_view<'a>(
    p: Palette,
    config: &SessionConfig,
    work_input: &'a str,
    short_input: &'a str,
    long_input: &'a str,
    sound_option: SoundOption,
    autostart: bool,
) -> Element<'a, Message> {
    let preset_btn = |preset: TimerPreset, label: &'static str| -> Element<'a, Message> {
        let active = config.preset == preset;
        button(text(label).size(11))
            .width(Length::Fill)
            .padding([7, 0])
            .style(iced_theme::Button::Custom(if active {
                Box::new(AccentBtn(p)) as Box<dyn iced::widget::button::StyleSheet<Style = iced::Theme>>
            } else {
                Box::new(GhostBtn(p))
            }))
            .on_press(Message::SetPreset(preset))
            .into()
    };

    let preset_row1 = row(vec![
        preset_btn(TimerPreset::Classic,  "Classic"),
        Space::with_width(8).into(),
        preset_btn(TimerPreset::DeepWork, "Deep Work"),
    ])
    .align_items(Alignment::Center);

    let preset_row2 = row(vec![
        preset_btn(TimerPreset::Balanced, "Balanced"),
        Space::with_width(8).into(),
        preset_btn(TimerPreset::Custom,   "Custom"),
    ])
    .align_items(Alignment::Center);

    let desc = text(config.preset_desc())
        .size(9)
        .style(iced_theme::Text::Color(p.subtext));

    let sound_btn = |opt: SoundOption| -> Element<'a, Message> {
        let active = sound_option == opt;
        button(text(opt.label()).size(11))
            .width(Length::Fill)
            .padding([7, 0])
            .style(iced_theme::Button::Custom(if active {
                Box::new(AccentBtn(p)) as Box<dyn iced::widget::button::StyleSheet<Style = iced::Theme>>
            } else {
                Box::new(GhostBtn(p))
            }))
            .on_press(Message::SetSound(opt))
            .into()
    };

    let sound_row1 = row(vec![
        sound_btn(SoundOption::Chime),
        Space::with_width(8).into(),
        sound_btn(SoundOption::KitchenTimer),
    ])
    .align_items(Alignment::Center);

    let sound_row2 = row(vec![
        sound_btn(SoundOption::Angelic),
        Space::with_width(8).into(),
        Space::with_width(Length::Fill).into(),
    ])
    .align_items(Alignment::Center);

    let autostart_btn = button(
        text(if autostart { "✓  Launch at login" } else { "   Launch at login" }).size(11),
    )
    .width(Length::Fill)
    .padding([7, 0])
    .style(iced_theme::Button::Custom(if autostart {
        Box::new(AccentBtn(p)) as Box<dyn iced::widget::button::StyleSheet<Style = iced::Theme>>
    } else {
        Box::new(GhostBtn(p))
    }))
    .on_press(Message::ToggleAutostart);

    let mut inner_items: Vec<Element<Message>> = vec![
        row(vec![
            text("settings")
                .size(13)
                .style(iced_theme::Text::Color(p.text))
                .into(),
            Space::with_width(Length::Fill).into(),
            button(text("done").size(10))
                .padding([2, 8])
                .style(iced_theme::Button::Custom(Box::new(GhostBtn(p))))
                .on_press(Message::ToggleSettings)
                .into(),
        ])
        .align_items(Alignment::Center)
        .into(),
        Space::with_height(10).into(),
        text("Session Length")
            .size(10)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
        Space::with_height(6).into(),
        preset_row1.into(),
        Space::with_height(6).into(),
        preset_row2.into(),
        Space::with_height(8).into(),
        desc.into(),
        Space::with_height(14).into(),
        text("Completion Sound")
            .size(10)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
        Space::with_height(6).into(),
        sound_row1.into(),
        Space::with_height(6).into(),
        sound_row2.into(),
    ];

    if config.preset == TimerPreset::Custom {
        let input_row = |label: &'static str, val: &'a str, msg: fn(String) -> Message| -> Element<'a, Message> {
            row(vec![
                text(label)
                    .size(11)
                    .style(iced_theme::Text::Color(p.subtext))
                    .width(Length::Fixed(72.0))
                    .into(),
                text_input("", val)
                    .on_input(msg)
                    .width(Length::Fixed(42.0))
                    .padding([4, 6])
                    .size(11)
                    .style(iced_theme::TextInput::Custom(Box::new(TaskInput(p))))
                    .into(),
                Space::with_width(6).into(),
                text("min")
                    .size(10)
                    .style(iced_theme::Text::Color(p.subtext))
                    .into(),
            ])
            .align_items(Alignment::Center)
            .into()
        };

        inner_items.push(Space::with_height(12).into());
        inner_items.push(input_row("Work", work_input, Message::CustomWorkChanged));
        inner_items.push(Space::with_height(5).into());
        inner_items.push(input_row("Short break", short_input, Message::CustomShortChanged));
        inner_items.push(Space::with_height(5).into());
        inner_items.push(input_row("Long break", long_input, Message::CustomLongChanged));
        inner_items.push(Space::with_height(10).into());
        inner_items.push(
            button(text("Apply").size(11))
                .padding([6, 16])
                .style(iced_theme::Button::Custom(Box::new(AccentBtn(p))))
                .on_press(Message::ApplyCustomPreset)
                .into(),
        );
    }

    inner_items.push(Space::with_height(14).into());
    inner_items.push(
        text("System")
            .size(10)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
    );
    inner_items.push(Space::with_height(6).into());
    inner_items.push(autostart_btn.into());

    container(
        scrollable(column(inner_items).padding([14, 20])).height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(iced_theme::Container::Custom(Box::new(Flat)))
    .into()
}

// ── Shortcuts View ────────────────────────────────────────────────────────

fn shortcuts_view(p: Palette) -> Element<'static, Message> {
    let row_item = move |key: &'static str, desc: &'static str| -> Element<'static, Message> {
        row(vec![
            text(key)
                .size(11)
                .font(iced::Font::MONOSPACE)
                .style(iced_theme::Text::Color(p.accent))
                .width(Length::Fixed(110.0))
                .into(),
            text(desc)
                .size(11)
                .style(iced_theme::Text::Color(p.subtext))
                .into(),
        ])
        .align_items(Alignment::Center)
        .into()
    };

    let section = move |label: &'static str| -> Element<'static, Message> {
        text(label)
            .size(10)
            .style(iced_theme::Text::Color(p.subtext))
            .into()
    };

    let items: Vec<Element<Message>> = vec![
        row(vec![
            text("shortcuts")
                .size(13)
                .style(iced_theme::Text::Color(p.text))
                .into(),
            Space::with_width(Length::Fill).into(),
            button(text("done").size(10))
                .padding([2, 8])
                .style(iced_theme::Button::Custom(Box::new(GhostBtn(p))))
                .on_press(Message::ToggleShortcuts)
                .into(),
        ])
        .align_items(Alignment::Center)
        .into(),
        Space::with_height(12).into(),
        section("Timer").into(),
        Space::with_height(6).into(),
        row_item("Space",   "start / pause"),
        Space::with_height(4).into(),
        row_item("R",       "reset timer"),
        Space::with_height(4).into(),
        row_item("S",       "skip phase"),
        Space::with_height(12).into(),
        section("Navigate").into(),
        Space::with_height(6).into(),
        row_item("← →",    "switch tabs"),
        Space::with_height(4).into(),
        row_item("Esc",     "close panel"),
        Space::with_height(4).into(),
        row_item("?",       "this panel"),
        Space::with_height(12).into(),
        section("Global").into(),
        Space::with_height(6).into(),
        row_item("Ctrl+Shift+F", "show / hide window"),
    ];

    container(
        scrollable(column(items).padding([14, 20])).height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(iced_theme::Container::Custom(Box::new(Flat)))
    .into()
}

// ── Changelog View ────────────────────────────────────────────────────────

fn changelog_view(p: Palette) -> Element<'static, Message> {
    let heading = |s: &'static str| -> Element<'static, Message> {
        text(s)
            .size(10)
            .style(iced_theme::Text::Color(p.subtext))
            .into()
    };

    let bullet = move |s: &'static str| -> Element<'static, Message> {
        row(vec![
            text("·")
                .size(11)
                .style(iced_theme::Text::Color(p.accent))
                .width(Length::Fixed(14.0))
                .into(),
            text(s)
                .size(11)
                .style(iced_theme::Text::Color(p.text))
                .into(),
        ])
        .into()
    };

    let items: Vec<Element<Message>> = vec![
        row(vec![
            text(format!("what's new  ·  {}", CURRENT_VERSION))
                .size(13)
                .style(iced_theme::Text::Color(p.text))
                .into(),
        ])
        .into(),
        Space::with_height(14).into(),
        heading("System"),
        Space::with_height(5).into(),
        bullet("Close to tray — app keeps running in the background"),
        Space::with_height(3).into(),
        bullet("Ctrl+Shift+F — show or hide from anywhere"),
        Space::with_height(3).into(),
        bullet("Launch at login — toggle in Settings"),
        Space::with_height(12).into(),
        heading("Notifications"),
        Space::with_height(5).into(),
        bullet("Desktop alert when a work session ends"),
        Space::with_height(12).into(),
        heading("Shortcuts"),
        Space::with_height(5).into(),
        bullet("Space / R / S / ← → timer controls"),
        Space::with_height(3).into(),
        bullet("? button opens the shortcut reference panel"),
        Space::with_height(12).into(),
        heading("Updates"),
        Space::with_height(5).into(),
        bullet("Check for update from the system tray menu"),
        Space::with_height(3).into(),
        bullet("Changelog shown after each update"),
        Space::with_height(18).into(),
        button(text("Got it").size(11))
            .padding([7, 24])
            .style(iced_theme::Button::Custom(Box::new(AccentBtn(p))))
            .on_press(Message::DismissChangelog)
            .into(),
    ];

    container(
        scrollable(column(items).padding([18, 20])).height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(iced_theme::Container::Custom(Box::new(Flat)))
    .into()
}

// ── Heatmap View ──────────────────────────────────────────────────────────

fn heatmap_view<'a>(p: Palette, heatmap: &'a Heatmap, hovered: Option<NaiveDate>) -> Element<'a, Message> {
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
                let is_future = date > today;
                let color = if is_future {
                    iced::Color { a: 0.07, ..p.surface2 }
                } else {
                    theme::heat_color(heatmap.get(date), p)
                };
                let cell = container(Space::new(Length::Fixed(10.0), Length::Fixed(10.0)))
                    .style(iced_theme::Container::Custom(Box::new(HeatCell(color))));
                if is_future {
                    cells.push(cell.into());
                } else {
                    cells.push(
                        mouse_area(cell)
                            .on_enter(Message::HeatCellEntered(date))
                            .on_exit(Message::HeatCellLeft)
                            .into(),
                    );
                }
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

    let hover_label: String = match hovered {
        Some(date) => {
            let mins = heatmap.get(date);
            let sessions = mins / 25;
            let s = if sessions == 1 { "" } else { "s" };
            format!("{} — {} session{}", date.format("%b %-d"), sessions, s)
        }
        None => String::new(),
    };

    let inner = column(vec![
        header.into(),
        Space::with_height(12).into(),
        column(grid_rows).spacing(3).into(),
        Space::with_height(6).into(),
        text(hover_label)
            .size(9)
            .style(iced_theme::Text::Color(p.subtext))
            .into(),
    ])
    .padding([14, 10]);

    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(iced_theme::Container::Custom(Box::new(Flat)))
        .into()
}

// ── App Icon ──────────────────────────────────────────────────────────────

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

            if r > 14.5 { continue; }

            if r >= 11.0 {
                rgba[i]   = 96;
                rgba[i+1] = 165;
                rgba[i+2] = 250;
                rgba[i+3] = 255;
            } else {
                rgba[i]   = 15;
                rgba[i+1] = 17;
                rgba[i+2] = 23;
                rgba[i+3] = 255;

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
    platform::setup_tray();
    platform::setup_hotkey();

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
