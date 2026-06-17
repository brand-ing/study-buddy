use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub text: String,
    pub done: bool,
}

impl Task {
    pub fn new(id: u64, text: String) -> Self {
        Self { id, text, done: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum TimerPreset {
    #[default]
    Classic,
    DeepWork,
    Balanced,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SessionConfig {
    pub preset: TimerPreset,
    pub work_mins: u32,
    pub short_mins: u32,
    pub long_mins: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self { preset: TimerPreset::Classic, work_mins: 25, short_mins: 5, long_mins: 15 }
    }
}

impl SessionConfig {
    pub fn from_preset(preset: TimerPreset) -> Self {
        match preset {
            TimerPreset::Classic  => Self { preset, work_mins: 25, short_mins: 5,  long_mins: 15 },
            TimerPreset::DeepWork => Self { preset, work_mins: 50, short_mins: 10, long_mins: 0 },
            TimerPreset::Balanced => Self { preset, work_mins: 45, short_mins: 15, long_mins: 0 },
            TimerPreset::Custom   => Self::default(),
        }
    }

    pub fn has_open_break(&self) -> bool {
        matches!(self.preset, TimerPreset::DeepWork | TimerPreset::Balanced)
    }

    pub fn preset_desc(&self) -> &'static str {
        match self.preset {
            TimerPreset::Classic  => "25 min work · 5 min break · 15 min long break",
            TimerPreset::DeepWork => "50 min work · 10 min break · open long break",
            TimerPreset::Balanced => "45 min work · 15 min break · open long break",
            TimerPreset::Custom   => "customize session lengths below",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Phase {
    Work,
    ShortBreak,
    LongBreak,
    OpenBreak,
}

impl Phase {
    pub fn label(&self) -> &'static str {
        match self {
            Phase::Work       => "▸  Work",
            Phase::ShortBreak => "◌  Short Break",
            Phase::LongBreak  => "◎  Long Break",
            Phase::OpenBreak  => "◎  Break — take your time",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pomodoro {
    pub phase: Phase,
    pub remaining: u32,
    pub running: bool,
    pub done: u32,
    pub config: SessionConfig,
}

impl Pomodoro {
    pub fn new(done: u32, config: SessionConfig) -> Self {
        let mut p = Self { phase: Phase::Work, remaining: 0, running: false, done, config };
        p.remaining = p.phase_duration(Phase::Work);
        p
    }

    pub fn phase_duration(&self, phase: Phase) -> u32 {
        match phase {
            Phase::Work       => self.config.work_mins * 60,
            Phase::ShortBreak => self.config.short_mins * 60,
            Phase::LongBreak  => self.config.long_mins * 60,
            Phase::OpenBreak  => 0,
        }
    }

    pub fn tick(&mut self) -> bool {
        if !self.running || self.phase == Phase::OpenBreak { return false; }
        if self.remaining > 0 { self.remaining -= 1; }
        if self.remaining == 0 {
            let completed = self.phase == Phase::Work;
            if completed { self.done += 1; }
            self.advance();
            return completed;
        }
        false
    }

    pub fn advance(&mut self) {
        self.phase = match self.phase {
            Phase::Work => {
                if self.done % 4 == 0 {
                    if self.config.has_open_break() { Phase::OpenBreak } else { Phase::LongBreak }
                } else {
                    Phase::ShortBreak
                }
            }
            _ => Phase::Work,
        };
        self.remaining = self.phase_duration(self.phase);
        self.running = false;
    }

    pub fn skip(&mut self) { self.advance(); }

    pub fn reset(&mut self) { *self = Self::new(self.done, self.config); }

    pub fn set_config(&mut self, cfg: SessionConfig) {
        self.config = cfg;
        self.phase = Phase::Work;
        self.remaining = self.phase_duration(Phase::Work);
        self.running = false;
    }

    pub fn format(&self) -> String {
        if self.phase == Phase::OpenBreak { return "--:--".to_string(); }
        format!("{:02}:{:02}", self.remaining / 60, self.remaining % 60)
    }

    pub fn progress(&self) -> f32 {
        let total = self.phase_duration(self.phase) as f32;
        if total == 0.0 { return 0.0; }
        (total - self.remaining as f32) / total
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Heatmap {
    pub data: HashMap<String, u32>,
}

impl Heatmap {
    pub fn add(&mut self, minutes: u32) {
        let today = chrono::Local::now().date_naive().to_string();
        *self.data.entry(today).or_insert(0) += minutes;
    }

    pub fn get(&self, date: chrono::NaiveDate) -> u32 {
        self.data.get(&date.to_string()).copied().unwrap_or(0)
    }
}
