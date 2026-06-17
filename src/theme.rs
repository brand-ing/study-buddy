use chrono::Timelike;
use iced::widget::{button, container, progress_bar, text_input};
use iced::{application, Background, Border, Color};

fn hex(v: u32) -> Color {
    Color::from_rgb8(
        ((v >> 16) & 0xFF) as u8,
        ((v >> 8) & 0xFF) as u8,
        (v & 0xFF) as u8,
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimeOfDay {
    Sunrise,
    Morning,
    Afternoon,
    Evening,
    Night,
    LateNight,
}

impl TimeOfDay {
    pub fn now() -> Self {
        match chrono::Local::now().hour() {
            5 | 6   => Self::Sunrise,
            7..=11  => Self::Morning,
            12..=16 => Self::Afternoon,
            17..=19 => Self::Evening,
            20..=22 => Self::Night,
            _       => Self::LateNight,
        }
    }

    pub fn palette(&self) -> Palette {
        match self {
            Self::Sunrise => Palette {
                bg:       hex(0x1A0A2E),
                surface:  hex(0x2D1B45),
                surface2: hex(0x3D2B55),
                accent:   hex(0xFF9A7B),
                accent2:  hex(0xCC7055),
                text:     hex(0xFDE8D8),
                subtext:  hex(0xB89A8A),
                success:  hex(0xFFB38E),
                name: "Sunrise",
            },
            Self::Morning => Palette {
                bg:       hex(0xF5F0E8),
                surface:  hex(0xFFFFFF),
                surface2: hex(0xEDE8DC),
                accent:   hex(0xF59E0B),
                accent2:  hex(0xD97706),
                text:     hex(0x1E293B),
                subtext:  hex(0x64748B),
                success:  hex(0x16A34A),
                name: "Morning",
            },
            Self::Afternoon => Palette {
                bg:       hex(0xFEF8EE),
                surface:  hex(0xFFFFFF),
                surface2: hex(0xFBF0DC),
                accent:   hex(0xF97316),
                accent2:  hex(0xEA6600),
                text:     hex(0x292524),
                subtext:  hex(0x78716C),
                success:  hex(0x16A34A),
                name: "Afternoon",
            },
            Self::Evening => Palette {
                bg:       hex(0x1E1035),
                surface:  hex(0x2D1F45),
                surface2: hex(0x3D2F55),
                accent:   hex(0xFF7B6B),
                accent2:  hex(0xCC4545),
                text:     hex(0xFDE8D8),
                subtext:  hex(0xB89A8A),
                success:  hex(0x4ADE80),
                name: "Evening",
            },
            Self::Night => Palette {
                bg:       hex(0x0F1117),
                surface:  hex(0x1A1D27),
                surface2: hex(0x252835),
                accent:   hex(0x60A5FA),
                accent2:  hex(0x387ACB),
                text:     hex(0xE2E8F0),
                subtext:  hex(0x64748B),
                success:  hex(0x4ADE80),
                name: "Night",
            },
            Self::LateNight => Palette {
                bg:       hex(0x080812),
                surface:  hex(0x10101E),
                surface2: hex(0x181828),
                accent:   hex(0x7C3AED),
                accent2:  hex(0x5B21B6),
                text:     hex(0x94A3B8),
                subtext:  hex(0x475569),
                success:  hex(0x4ADE80),
                name: "Late Night",
            },
        }
    }

    pub fn iced_theme(&self) -> iced::Theme {
        match self {
            Self::Morning | Self::Afternoon => iced::Theme::Light,
            _ => iced::Theme::Dark,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Color,
    pub surface: Color,
    pub surface2: Color,
    pub accent: Color,
    pub accent2: Color,
    pub text: Color,
    pub subtext: Color,
    pub success: Color,
    pub name: &'static str,
}

pub fn heat_color(minutes: u32, p: Palette) -> Color {
    let t: f32 = match minutes {
        0       => 0.0,
        1..=20  => 0.25,
        21..=50 => 0.5,
        51..=100 => 0.75,
        _       => 1.0,
    };
    Color {
        r: p.surface2.r + (p.accent.r - p.surface2.r) * t,
        g: p.surface2.g + (p.accent.g - p.surface2.g) * t,
        b: p.surface2.b + (p.accent.b - p.surface2.b) * t,
        a: 1.0,
    }
}

// ── StyleSheet Implementations ────────────────────────────────────────────

pub struct AppBg(pub Palette);
impl application::StyleSheet for AppBg {
    type Style = iced::Theme;
    fn appearance(&self, _: &iced::Theme) -> application::Appearance {
        application::Appearance {
            background_color: Color::TRANSPARENT, // outer container draws the bg with rounded corners
            text_color: self.0.text,
        }
    }
}

pub struct Surface(pub Palette, pub bool); // true = surface2
impl container::StyleSheet for Surface {
    type Style = iced::Theme;
    fn appearance(&self, _: &iced::Theme) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(if self.1 { self.0.surface2 } else { self.0.surface })),
            border: Border { radius: 12.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            text_color: Some(self.0.text),
            ..Default::default()
        }
    }
}

pub struct Flat;
impl container::StyleSheet for Flat {
    type Style = iced::Theme;
    fn appearance(&self, _: &iced::Theme) -> container::Appearance {
        container::Appearance::default()
    }
}

pub struct HeatCell(pub Color);
impl container::StyleSheet for HeatCell {
    type Style = iced::Theme;
    fn appearance(&self, _: &iced::Theme) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.0)),
            border: Border { radius: 3.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
}

pub struct DotCell(pub Color);
impl container::StyleSheet for DotCell {
    type Style = iced::Theme;
    fn appearance(&self, _: &iced::Theme) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.0)),
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
}

pub struct AccentBtn(pub Palette);
impl button::StyleSheet for AccentBtn {
    type Style = iced::Theme;
    fn active(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(self.0.accent)),
            text_color: self.0.bg,
            border: Border { radius: 8.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn hovered(&self, s: &iced::Theme) -> button::Appearance {
        let mut a = self.active(s);
        a.background = Some(Background::Color(self.0.accent2));
        a
    }
    fn pressed(&self, s: &iced::Theme) -> button::Appearance { self.hovered(s) }
    fn disabled(&self, s: &iced::Theme) -> button::Appearance {
        let mut a = self.active(s);
        if let Some(Background::Color(ref mut c)) = a.background { c.a = 0.35; }
        a
    }
}

pub struct GhostBtn(pub Palette);
impl button::StyleSheet for GhostBtn {
    type Style = iced::Theme;
    fn active(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: self.0.subtext,
            border: Border { radius: 8.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn hovered(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color { a: 0.1, ..self.0.accent })),
            text_color: self.0.accent,
            border: Border { radius: 8.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn pressed(&self, s: &iced::Theme) -> button::Appearance { self.hovered(s) }
    fn disabled(&self, s: &iced::Theme) -> button::Appearance { self.active(s) }
}

pub struct TaskCheckBtn { pub p: Palette, pub done: bool }
impl button::StyleSheet for TaskCheckBtn {
    type Style = iced::Theme;
    fn active(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(
                if self.done { self.p.success } else { self.p.surface2 }
            )),
            text_color: if self.done { self.p.bg } else { self.p.subtext },
            border: Border { radius: 5.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn hovered(&self, s: &iced::Theme) -> button::Appearance { self.active(s) }
    fn pressed(&self, s: &iced::Theme) -> button::Appearance { self.active(s) }
    fn disabled(&self, s: &iced::Theme) -> button::Appearance { self.active(s) }
}

pub struct DeleteBtn(pub Palette);
impl button::StyleSheet for DeleteBtn {
    type Style = iced::Theme;
    fn active(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: Color { a: 0.0, ..Color::BLACK },
            border: Border { radius: 5.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn hovered(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color { r: 0.9, g: 0.2, b: 0.2, a: 0.15 })),
            text_color: Color { r: 0.85, g: 0.25, b: 0.25, a: 1.0 },
            border: Border { radius: 5.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn pressed(&self, s: &iced::Theme) -> button::Appearance { self.hovered(s) }
    fn disabled(&self, s: &iced::Theme) -> button::Appearance { self.active(s) }
}

pub struct TaskInput(pub Palette);
impl text_input::StyleSheet for TaskInput {
    type Style = iced::Theme;
    fn active(&self, _: &iced::Theme) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(self.0.surface2),
            border: Border { radius: 8.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            icon_color: self.0.subtext,
        }
    }
    fn focused(&self, _: &iced::Theme) -> text_input::Appearance {
        text_input::Appearance {
            background: Background::Color(self.0.surface),
            border: Border { radius: 8.0.into(), color: self.0.accent, width: 1.5 },
            icon_color: self.0.accent,
        }
    }
    fn placeholder_color(&self, _: &iced::Theme) -> Color { self.0.subtext }
    fn value_color(&self, _: &iced::Theme) -> Color { self.0.text }
    fn disabled_color(&self, _: &iced::Theme) -> Color { self.0.subtext }
    fn selection_color(&self, _: &iced::Theme) -> Color {
        Color { a: 0.3, ..self.0.accent }
    }
    fn hovered(&self, s: &iced::Theme) -> text_input::Appearance { self.focused(s) }
    fn disabled(&self, s: &iced::Theme) -> text_input::Appearance { self.active(s) }
}

pub struct ProgressStyle(pub Palette);
impl progress_bar::StyleSheet for ProgressStyle {
    type Style = iced::Theme;
    fn appearance(&self, _: &iced::Theme) -> progress_bar::Appearance {
        progress_bar::Appearance {
            background: Background::Color(self.0.surface2),
            bar: Background::Color(self.0.accent),
            border_radius: 2.0.into(),
        }
    }
}

// ── Titlebar / Navigation ─────────────────────────────────────────────────

/// Invisible drag area — no visual change, just captures clicks for window drag.
pub struct DragBtn;
impl button::StyleSheet for DragBtn {
    type Style = iced::Theme;
    fn active(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: Color::TRANSPARENT,
            border: Border { radius: 0.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn hovered(&self, s: &iced::Theme) -> button::Appearance { self.active(s) }
    fn pressed(&self, s: &iced::Theme) -> button::Appearance { self.active(s) }
    fn disabled(&self, s: &iced::Theme) -> button::Appearance { self.active(s) }
}

pub struct TabBtn { pub p: Palette, pub active: bool }
impl button::StyleSheet for TabBtn {
    type Style = iced::Theme;
    fn active(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: if self.active {
                Some(Background::Color(Color { a: 0.13, ..self.p.accent }))
            } else {
                None
            },
            text_color: if self.active { self.p.accent } else { self.p.subtext },
            border: Border { radius: 6.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn hovered(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color { a: 0.08, ..self.p.accent })),
            text_color: self.p.accent,
            border: Border { radius: 6.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn pressed(&self, s: &iced::Theme) -> button::Appearance { self.hovered(s) }
    fn disabled(&self, s: &iced::Theme) -> button::Appearance { self.active(s) }
}

pub struct CloseBtn(pub Palette);
impl button::StyleSheet for CloseBtn {
    type Style = iced::Theme;
    fn active(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: None,
            text_color: self.0.subtext,
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn hovered(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color { r: 0.85, g: 0.18, b: 0.18, a: 0.88 })),
            text_color: Color::WHITE,
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn pressed(&self, s: &iced::Theme) -> button::Appearance { self.hovered(s) }
    fn disabled(&self, s: &iced::Theme) -> button::Appearance { self.active(s) }
}

pub struct OuterBorder(pub Palette);
impl container::StyleSheet for OuterBorder {
    type Style = iced::Theme;
    fn appearance(&self, _: &iced::Theme) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.0.bg)),
            border: Border { radius: 10.0.into(), color: self.0.surface2, width: 1.0 },
            ..Default::default()
        }
    }
}

pub struct SettingsCard(pub Palette);
impl container::StyleSheet for SettingsCard {
    type Style = iced::Theme;
    fn appearance(&self, _: &iced::Theme) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.0.surface)),
            border: Border { radius: 8.0.into(), color: self.0.surface2, width: 1.0 },
            ..Default::default()
        }
    }
}

pub struct HoverBar(pub Palette);
impl container::StyleSheet for HoverBar {
    type Style = iced::Theme;
    fn appearance(&self, _: &iced::Theme) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.0.surface)),
            border: Border {
                radius: [10.0, 10.0, 0.0, 0.0].into(),
                color: Color::TRANSPARENT,
                width: 0.0,
            },
            ..Default::default()
        }
    }
}

pub struct Sidebar(pub Palette);
impl container::StyleSheet for Sidebar {
    type Style = iced::Theme;
    fn appearance(&self, _: &iced::Theme) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(self.0.surface)),
            border: Border {
                radius: [10.0, 0.0, 0.0, 10.0].into(),
                color: Color::TRANSPARENT,
                width: 0.0,
            },
            ..Default::default()
        }
    }
}

pub struct DragRow(pub Palette);
impl container::StyleSheet for DragRow {
    type Style = iced::Theme;
    fn appearance(&self, _: &iced::Theme) -> container::Appearance {
        container::Appearance {
            background: Some(Background::Color(Color { a: 0.12, ..self.0.accent })),
            border: Border { radius: 6.0.into(), color: Color { a: 0.30, ..self.0.accent }, width: 1.0 },
            ..Default::default()
        }
    }
}

pub struct SettingsBtn { pub p: Palette, pub active: bool }
impl button::StyleSheet for SettingsBtn {
    type Style = iced::Theme;
    fn active(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: if self.active {
                Some(Background::Color(Color { a: 0.13, ..self.p.accent }))
            } else {
                None
            },
            text_color: if self.active { self.p.accent } else { self.p.subtext },
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn hovered(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color { a: 0.08, ..self.p.accent })),
            text_color: self.p.accent,
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn pressed(&self, s: &iced::Theme) -> button::Appearance { self.hovered(s) }
    fn disabled(&self, s: &iced::Theme) -> button::Appearance { self.active(s) }
}

pub struct PinBtn { pub p: Palette, pub active: bool }
impl button::StyleSheet for PinBtn {
    type Style = iced::Theme;
    fn active(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: if self.active {
                Some(Background::Color(Color { a: 0.13, ..self.p.accent }))
            } else {
                None
            },
            text_color: if self.active { self.p.accent } else { self.p.subtext },
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn hovered(&self, _: &iced::Theme) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(Color { a: 0.08, ..self.p.accent })),
            text_color: self.p.accent,
            border: Border { radius: 4.0.into(), color: Color::TRANSPARENT, width: 0.0 },
            ..Default::default()
        }
    }
    fn pressed(&self, s: &iced::Theme) -> button::Appearance { self.hovered(s) }
    fn disabled(&self, s: &iced::Theme) -> button::Appearance { self.active(s) }
}
