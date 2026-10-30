//! Terminal palette.

use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub amber: Style,
    pub amber_soft: Style,
    pub dim: Style,
    pub faint: Style,
    pub text: Style,
}

impl Palette {
    pub fn new() -> Self {
        let amber = Color::Indexed(179);
        let amber_soft = Color::Indexed(137);
        let dim = Color::Indexed(244);
        Self {
            amber: Style::default().fg(amber).add_modifier(Modifier::BOLD),
            amber_soft: Style::default().fg(amber_soft).add_modifier(Modifier::BOLD),
            dim: Style::default().fg(dim),
            faint: Style::default().fg(dim).add_modifier(Modifier::DIM),
            text: Style::default(),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}
