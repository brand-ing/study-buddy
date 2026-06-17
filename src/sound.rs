use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink};

pub struct AudioPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    chime: Vec<f32>,
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
            click: gen_click(),
        })
    }

    pub fn play_chime(&self) {
        if let Ok(sink) = Sink::try_new(&self.handle) {
            sink.append(SamplesBuffer::new(1, SR, self.chime.clone()));
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
