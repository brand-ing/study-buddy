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

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Work,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn duration(&self) -> u32 {
        match self {
            Phase::Work => 25 * 60,
            Phase::ShortBreak => 5 * 60,
            Phase::LongBreak => 15 * 60,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Phase::Work       => "🎯  Work",
            Phase::ShortBreak => "☕  Short Break",
            Phase::LongBreak  => "🌿  Long Break",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pomodoro {
    pub phase: Phase,
    pub remaining: u32,
    pub running: bool,
    pub done: u32,
}

impl Default for Pomodoro {
    fn default() -> Self {
        Self {
            phase: Phase::Work,
            remaining: Phase::Work.duration(),
            running: false,
            done: 0,
        }
    }
}

impl Pomodoro {
    pub fn new(done: u32) -> Self {
        Self { done, ..Default::default() }
    }

    /// Advances the timer by one second. Returns true when a Work session completes.
    pub fn tick(&mut self) -> bool {
        if !self.running {
            return false;
        }
        if self.remaining > 0 {
            self.remaining -= 1;
        }
        if self.remaining == 0 {
            let completed_work = self.phase == Phase::Work;
            if completed_work {
                self.done += 1;
            }
            self.advance();
            return completed_work;
        }
        false
    }

    fn advance(&mut self) {
        self.phase = match self.phase {
            Phase::Work => {
                if self.done % 4 == 0 { Phase::LongBreak } else { Phase::ShortBreak }
            }
            _ => Phase::Work,
        };
        self.remaining = self.phase.duration();
        self.running = false;
    }

    pub fn skip(&mut self) {
        self.advance();
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.done);
    }

    pub fn format(&self) -> String {
        format!("{:02}:{:02}", self.remaining / 60, self.remaining % 60)
    }

    pub fn progress(&self) -> f32 {
        let total = self.phase.duration() as f32;
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
