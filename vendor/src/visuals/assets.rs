use crate::compositor::plane::{Cell, Color, Styles};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Folder,
    File,
    Rust,
    Json,
    Settings,
    Dracon,
    ButtonLeft,
    ButtonMid,
    ButtonRight,
}

pub struct IconAsset {
    pub width: u16,
    pub height: u16,
    pub cells: Vec<Cell>,
}

impl Icon {
    pub fn get_asset(&self) -> IconAsset {
        match self {
            Icon::Folder => generate_folder_icon(),
            Icon::File => generate_file_icon(),
            Icon::Rust => generate_rust_icon(),
            Icon::Json => generate_json_icon(),
            Icon::Settings => generate_settings_icon(),
            Icon::Dracon => generate_dracon_icon(),
            Icon::ButtonLeft => generate_button_slice(Slice::Left),
            Icon::ButtonMid => generate_button_slice(Slice::Mid),
            Icon::ButtonRight => generate_button_slice(Slice::Right),
        }
    }

    pub fn get_sprite_data(&self) -> Vec<u8> {
        let w = 16;
        let h = 16;
        let mut data = vec![0u8; (w * h * 4) as usize];

        let color = match self {
            Icon::Folder => (250, 200, 50),
            Icon::File => (200, 200, 220),
            Icon::Rust => (222, 165, 132),
            Icon::Json => (100, 150, 255),
            Icon::Settings => (150, 150, 160),
            Icon::Dracon => (255, 50, 50),
            _ => (128, 128, 128),
        };

        for y in 0..h {
            for x in 0..w {
                let idx = ((y * w + x) * 4) as usize;
                // Simple box with border
                if x == 0 || x == w - 1 || y == 0 || y == h - 1 {
                    // Border (Darker)
                    data[idx] = (color.0 as f32 * 0.5) as u8;
                    data[idx + 1] = (color.1 as f32 * 0.5) as u8;
                    data[idx + 2] = (color.2 as f32 * 0.5) as u8;
                    data[idx + 3] = 255;
                } else {
                    // Fill
                    data[idx] = color.0;
                    data[idx + 1] = color.1;
                    data[idx + 2] = color.2;
                    data[idx + 3] = 255;
                }
            }
        }

        // Custom shapes for specific icons
        if *self == Icon::Folder {
            // Add tab at top
            for y in 0..4 {
                for x in 8..16 {
                    let idx = ((y * w + x) * 4) as usize;
                    data[idx + 3] = 0; // Transparent (cut out top right)
                }
            }
        }

        data
    }
}

enum Slice {
    Left,
    Mid,
    Right,
}

fn generate_folder_icon() -> IconAsset {
    let cells = vec![
        Cell {
            char: '󰉋',
            fg: Color::Rgb(250, 200, 50),
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: false,
            skip: false,
        },
        Cell {
            char: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: true,
            skip: false,
        },
    ];
    IconAsset {
        width: 2,
        height: 1,
        cells,
    }
}

fn generate_file_icon() -> IconAsset {
    let cells = vec![
        Cell {
            char: '󰈔',
            fg: Color::Rgb(200, 200, 220),
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: false,
            skip: false,
        },
        Cell {
            char: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: true,
            skip: false,
        },
    ];
    IconAsset {
        width: 2,
        height: 1,
        cells,
    }
}

fn generate_rust_icon() -> IconAsset {
    let cells = vec![
        Cell {
            char: '',
            fg: Color::Rgb(222, 165, 132),
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: false,
            skip: false,
        },
        Cell {
            char: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: true,
            skip: false,
        },
    ];
    IconAsset {
        width: 2,
        height: 1,
        cells,
    }
}

fn generate_json_icon() -> IconAsset {
    let cells = vec![
        Cell {
            char: '',
            fg: Color::Rgb(100, 150, 255),
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: false,
            skip: false,
        },
        Cell {
            char: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: true,
            skip: false,
        },
    ];
    IconAsset {
        width: 2,
        height: 1,
        cells,
    }
}

fn generate_settings_icon() -> IconAsset {
    let cells = vec![
        Cell {
            char: '󰒓',
            fg: Color::Rgb(150, 150, 160),
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: false,
            skip: false,
        },
        Cell {
            char: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: true,
            skip: false,
        },
    ];
    IconAsset {
        width: 2,
        height: 1,
        cells,
    }
}

fn generate_dracon_icon() -> IconAsset {
    let cells = vec![
        Cell {
            char: '󰊠',
            fg: Color::Rgb(255, 50, 50),
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: false,
            skip: false,
        },
        Cell {
            char: ' ',
            fg: Color::Reset,
            bg: Color::Reset,
            style: Styles::empty(),
            transparent: true,
            skip: false,
        },
    ];
    IconAsset {
        width: 2,
        height: 1,
        cells,
    }
}

fn generate_button_slice(slice: Slice) -> IconAsset {
    // 3-slice button using Block characters
    let (char, fg, bg) = match slice {
        Slice::Left => ('▌', Color::Rgb(255, 180, 0), Color::Rgb(40, 40, 45)),
        Slice::Mid => ('█', Color::Rgb(40, 40, 45), Color::Reset),
        Slice::Right => ('▐', Color::Rgb(255, 180, 0), Color::Rgb(40, 40, 45)),
    };
    let cells = vec![Cell {
        char,
        fg,
        bg,
        style: Styles::empty(),
        transparent: false,
        skip: false,
    }];
    IconAsset {
        width: 1,
        height: 1,
        cells,
    }
}
