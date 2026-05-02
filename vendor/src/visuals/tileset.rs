use crate::compositor::plane::{Color, Styles};

#[derive(Clone, Copy, Debug)]
pub struct ThemeStyle {
    pub fg: Color,
    pub bg: Color,
    pub attrs: Styles,
}

impl ThemeStyle {
    pub fn new(fg: Color, bg: Color, attrs: Styles) -> Self {
        Self { fg, bg, attrs }
    }
}

pub trait Tileset {
    // Basic Geometry
    fn wall_vertical(&self) -> char;
    fn wall_horizontal(&self) -> char;
    fn corner_top_left(&self) -> char;
    fn corner_top_right(&self) -> char;
    fn corner_bottom_left(&self) -> char;
    fn corner_bottom_right(&self) -> char;

    // Palette
    fn primary_color(&self) -> Color;
    fn secondary_color(&self) -> Color;
    fn background_color(&self) -> Color;
    fn text_color(&self) -> Color;
    fn accent_color(&self) -> Color;

    // Semantic Components
    fn button_style(&self, focused: bool) -> ThemeStyle;
    fn header_style(&self) -> ThemeStyle;
}

pub struct DirectorTheme;
impl Tileset for DirectorTheme {
    fn wall_vertical(&self) -> char {
        '│'
    }
    fn wall_horizontal(&self) -> char {
        '─'
    }
    fn corner_top_left(&self) -> char {
        '╭'
    }
    fn corner_top_right(&self) -> char {
        '╮'
    }
    fn corner_bottom_left(&self) -> char {
        '╰'
    }
    fn corner_bottom_right(&self) -> char {
        '╯'
    }

    fn primary_color(&self) -> Color {
        Color::Rgb(0, 255, 200)
    } // Cyan
    fn secondary_color(&self) -> Color {
        Color::Rgb(40, 40, 60)
    } // Dark slate
    fn background_color(&self) -> Color {
        Color::Rgb(10, 10, 15)
    } // Deep void
    fn text_color(&self) -> Color {
        Color::Rgb(220, 220, 220)
    } // Off-white
    fn accent_color(&self) -> Color {
        Color::Rgb(255, 50, 100)
    } // Neon Pink

    fn button_style(&self, focused: bool) -> ThemeStyle {
        if focused {
            ThemeStyle::new(self.background_color(), self.primary_color(), Styles::BOLD)
        } else {
            ThemeStyle::new(
                self.primary_color(),
                self.secondary_color(),
                Styles::empty(),
            )
        }
    }

    fn header_style(&self) -> ThemeStyle {
        ThemeStyle::new(self.primary_color(), Color::Reset, Styles::BOLD)
    }
}

pub struct PaperTheme;
impl Tileset for PaperTheme {
    fn wall_vertical(&self) -> char {
        '|'
    }
    fn wall_horizontal(&self) -> char {
        '-'
    }
    fn corner_top_left(&self) -> char {
        '+'
    }
    fn corner_top_right(&self) -> char {
        '+'
    }
    fn corner_bottom_left(&self) -> char {
        '+'
    }
    fn corner_bottom_right(&self) -> char {
        '+'
    }

    fn primary_color(&self) -> Color {
        Color::Rgb(0, 0, 0)
    } // Black
    fn secondary_color(&self) -> Color {
        Color::Rgb(240, 240, 240)
    } // Light Grey
    fn background_color(&self) -> Color {
        Color::Rgb(255, 255, 255)
    } // White
    fn text_color(&self) -> Color {
        Color::Rgb(20, 20, 20)
    } // Dark Grey
    fn accent_color(&self) -> Color {
        Color::Rgb(0, 0, 200)
    } // Blue link

    fn button_style(&self, focused: bool) -> ThemeStyle {
        if focused {
            ThemeStyle::new(self.background_color(), self.text_color(), Styles::empty())
        } else {
            ThemeStyle::new(self.text_color(), self.secondary_color(), Styles::empty())
        }
    }

    fn header_style(&self) -> ThemeStyle {
        ThemeStyle::new(self.text_color(), Color::Reset, Styles::UNDERLINE)
    }
}
