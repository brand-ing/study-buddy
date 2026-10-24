use crate::data::SoundOption;
use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink};

pub struct AudioPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    chime: Vec<f32>,
    kitchen_timer: Vec<f32>,
    angelic: Vec<f32>,
    click: Vec<f32>,
}

const SR: u32 = 44100;

impl AudioPlayer {
    pub fn new() -> Option<Self> {
        let (stream, handle) = OutputStream::try_default().ok()?;
        Some(Self {
            _stream: stream,
            handle,
            chime: gen_chime(),
            kitchen_timer: gen_kitchen_timer(),
            angelic: gen_angelic(),
            click: gen_click(),
        })
    }

    pub fn play_alert(&self, opt: SoundOption) {
        let samples = match opt {
            SoundOption::Chime       => self.chime.clone(),
            SoundOption::KitchenTimer => self.kitchen_timer.clone(),
            SoundOption::Angelic     => self.angelic.clone(),
        };
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.append(SamplesBuffer::new(1, SR, samples));
            sink.detach();
        }
    }

    pub fn play_click(&self) {
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.append(SamplesBuffer::new(1, SR, self.click.clone()));
            sink.detach();
        }
    }
}

// C5 → E5 → G5 staggered, each note with exponential decay + 2nd harmonic
fn gen_chime() -> Vec<f32> {
    let notes: &[(f32, f32)] = &[(523.25, 0.0), (659.25, 0.35), (783.99, 0.70)];
    let total = (SR as f32 * 1.8) as usize;
    let mut out = vec![0f32; total];
    for &(freq, t0) in notes {
        let start = (t0 * SR as f32) as usize;
        let len = (SR as f32 * 0.9) as usize;
        for i in 0..len {
            if start + i >= total {
                break;
            }
            let t = i as f32 / SR as f32;
            let env = (-t * 3.5_f32).exp();
            out[start + i] += (std::f32::consts::TAU * freq * t).sin() * env * 0.35
                + (std::f32::consts::TAU * freq * 2.0 * t).sin() * env * 0.08;
        }
    }
    out
}

// 3 rapid metallic bell strikes like a wind-up kitchen timer reaching zero
fn gen_kitchen_timer() -> Vec<f32> {
    let total = (SR as f32 * 1.6) as usize;
    let mut out = vec![0f32; total];
    // Three strikes spaced 0.38s apart
    let strike_times: &[f32] = &[0.0, 0.38, 0.76];
    // Inharmonic partials for a metallic bell character
    let partials: &[(f32, f32)] = &[
        (1050.0, 0.40),
        (2897.0, 0.20),
        (5765.0, 0.12),
        (8200.0, 0.06),
    ];
    let strike_len = (SR as f32 * 0.55) as usize;
    for &t0 in strike_times {
        let start = (t0 * SR as f32) as usize;
        for i in 0..strike_len {
            if start + i >= total {
                break;
            }
            let t = i as f32 / SR as f32;
            let env = (-t * 9.0_f32).exp();
            let mut sample = 0.0f32;
            for &(freq, amp) in partials {
                sample += (std::f32::consts::TAU * freq * t).sin() * amp * env;
            }
            out[start + i] += sample * 0.55;
        }
    }
    out
}

// Soft angelic chord: E4-G4-B4-E5 with slow attack and ethereal shimmer
fn gen_angelic() -> Vec<f32> {
    let total = (SR as f32 * 3.2) as usize;
    let mut out = vec![0f32; total];
    // Chord notes with slight detuning pairs for width
    let voices: &[(f32, f32)] = &[
        (329.63, 0.18),  // E4
        (331.0,  0.10),  // E4 detuned
        (392.0,  0.18),  // G4
        (393.5,  0.10),  // G4 detuned
        (493.88, 0.20),  // B4
        (495.5,  0.10),  // B4 detuned
        (659.25, 0.22),  // E5
        (660.8,  0.12),  // E5 detuned
    ];
    let attack_secs = 0.55f32;
    let sustain_end = 1.8f32;
    let total_secs = total as f32 / SR as f32;
    for i in 0..total {
        let t = i as f32 / SR as f32;
        let env = if t < attack_secs {
            // smooth sinusoidal attack
            (std::f32::consts::PI * 0.5 * t / attack_secs).sin()
        } else if t < sustain_end {
            1.0
        } else {
            // cosine decay
            let decay_t = (t - sustain_end) / (total_secs - sustain_end);
            ((1.0 - decay_t) * std::f32::consts::PI * 0.5).sin().max(0.0)
        };
        let mut sample = 0.0f32;
        for &(freq, amp) in voices {
            sample += (std::f32::consts::TAU * freq * t).sin() * amp;
        }
        out[i] = sample * env * 0.38;
    }
    out
}

// Inharmonic partials with very fast decay → mechanical "clack"
fn gen_click() -> Vec<f32> {
    let partials: &[f32] = &[300.0, 720.0, 1350.0, 2100.0, 3800.0];
    let len = (SR as f32 * 0.055) as usize;
    let mut out = vec![0f32; len];
    for &f in partials {
        for i in 0..len {
            let t = i as f32 / SR as f32;
            out[i] += (std::f32::consts::TAU * f * t).sin() * (-t * 120.0_f32).exp() * 0.10;
        }
    }
    out
}
