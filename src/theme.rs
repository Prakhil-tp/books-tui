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
    /// Reads `NO_COLOR`: set and non-empty selects the monochrome palette.
    pub fn new() -> Self {
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
        Self::with_color(!no_color)
    }

    pub fn with_color(color: bool) -> Self {
        let paint = |style: Style, index: u8| {
            if color {
                style.fg(Color::Indexed(index))
            } else {
                style
            }
        };
        Self {
            amber: paint(Style::default(), 179).add_modifier(Modifier::BOLD),
            amber_soft: paint(Style::default(), 137).add_modifier(Modifier::BOLD),
            dim: paint(Style::default(), 244),
            faint: paint(Style::default(), 244).add_modifier(Modifier::DIM),
            text: Style::default(),
        }
        .monochrome_dim(color)
    }

    fn monochrome_dim(mut self, color: bool) -> Self {
        if !color {
            self.dim = self.dim.add_modifier(Modifier::DIM);
        }
        self
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}
