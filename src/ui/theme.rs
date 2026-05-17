#![allow(unused_imports, dead_code)]

use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use parking_lot::RwLock;

#[allow(dead_code)]
pub struct DraconTheme {
    pub bg: Color,
    pub fg: Color,
    pub accent_primary: Color,
    pub accent_secondary: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
    pub border_active: Color,
    pub border_inactive: Color,
    pub header_fg: Color,
    // Semantic roles — wired through ThemeStyle
    pub danger: Color,       // ESC, Quit, Delete, Kill
    pub warning: Color,      // Modified, Caution, Unsaved
    pub success: Color,      // Enabled, Saved, Connected
    pub muted: Color,        // Dim labels, Hints, DarkGray replacements
    pub info: Color,         // Informational, Cyan-type
    // File-type colors
    pub file_code: Color,
    pub file_config: Color,
    pub file_media: Color,
    pub file_archive: Color,
    pub file_exec: Color,
    // Subtle/structural colors
    pub border_subtle: Color,     // Very dark border (darker than border_inactive)
    pub selection_alt_bg: Color,  // Multi-selection row bg
    // Footer stat bar
    pub stat_cpu_blue: Color,
    pub stat_cpu_cyan: Color,
    pub stat_cpu_yellow: Color,
    pub stat_cpu_light: Color,
    pub stat_progress_bg: Color,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn to_color(self) -> Color {
        Color::Rgb(self.r, self.g, self.b)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeStyle {
    pub accent_primary: RgbColor,
    pub accent_secondary: RgbColor,
    pub selection_bg: RgbColor,
    pub selection_fg: RgbColor,
    pub border_active: RgbColor,
    pub border_inactive: RgbColor,
    pub header_fg: RgbColor,
    // Semantic roles
    pub danger: RgbColor,
    pub warning: RgbColor,
    pub success: RgbColor,
    pub muted: RgbColor,
    pub info: RgbColor,
    // File-type colors
    pub file_code: RgbColor,
    pub file_config: RgbColor,
    pub file_media: RgbColor,
    pub file_archive: RgbColor,
    pub file_exec: RgbColor,
    // Subtle/structural colors
    pub border_subtle: RgbColor,
    pub selection_alt_bg: RgbColor,
    // Footer stat bar
    pub stat_cpu_blue: RgbColor,
    pub stat_cpu_cyan: RgbColor,
    pub stat_cpu_yellow: RgbColor,
    pub stat_cpu_light: RgbColor,
    pub stat_progress_bg: RgbColor,
}

impl ThemeStyle {
    pub fn preset_warm() -> Self {
        Self {
            accent_primary: RgbColor::new(224, 164, 90),   // Warm amber
            accent_secondary: RgbColor::new(94, 199, 178),  // Mint-teal
            selection_bg: RgbColor::new(178, 122, 64),      // Muted bronze
            selection_fg: RgbColor::new(0, 0, 0),           // Black
            border_active: RgbColor::new(224, 164, 90),     // Warm amber
            border_inactive: RgbColor::new(86, 88, 98),     // Neutral slate
            header_fg: RgbColor::new(240, 196, 138),         // Sand
            danger: RgbColor::new(255, 80, 80),             // Bright red
            warning: RgbColor::new(255, 190, 50),           // Amber yellow
            success: RgbColor::new(80, 210, 120),           // Vivid green
            muted: RgbColor::new(120, 120, 135),            // Warm gray
            info: RgbColor::new(100, 200, 230),             // Sky cyan
            file_code: RgbColor::new(236, 156, 116),       // Apricot
            file_config: RgbColor::new(132, 190, 255),      // Sky blue
            file_media: RgbColor::new(201, 156, 244),       // Lilac
            file_archive: RgbColor::new(238, 132, 170),     // Rose
            file_exec: RgbColor::new(118, 203, 125),        // Green
            border_subtle: RgbColor::new(40, 45, 55),
            selection_alt_bg: RgbColor::new(78, 58, 112),
            stat_cpu_blue: RgbColor::new(88, 166, 255),
            stat_cpu_cyan: RgbColor::new(80, 200, 255),
            stat_cpu_yellow: RgbColor::new(255, 170, 0),
            stat_cpu_light: RgbColor::new(140, 165, 210),
            stat_progress_bg: RgbColor::new(85, 80, 20),
        }
    }

    pub fn preset_cool() -> Self {
        Self {
            accent_primary: RgbColor::new(160, 118, 255),   // Purple
            accent_secondary: RgbColor::new(116, 184, 255),  // Ice blue
            selection_bg: RgbColor::new(104, 80, 150),      // Deep violet
            selection_fg: RgbColor::new(255, 255, 255),     // White
            border_active: RgbColor::new(160, 118, 255),    // Purple
            border_inactive: RgbColor::new(82, 86, 104),    // Cool slate
            header_fg: RgbColor::new(198, 164, 255),        // Soft violet
            danger: RgbColor::new(255, 80, 120),            // Hot pink
            warning: RgbColor::new(255, 200, 80),           // Warm yellow
            success: RgbColor::new(80, 230, 160),           // Mint green
            muted: RgbColor::new(110, 112, 140),            // Cool gray
            info: RgbColor::new(130, 200, 255),             // Light blue
            file_code: RgbColor::new(200, 160, 255),        // Lavender
            file_config: RgbColor::new(120, 200, 240),      // Cyan
            file_media: RgbColor::new(255, 140, 200),       // Pink
            file_archive: RgbColor::new(255, 170, 120),     // Peach
            file_exec: RgbColor::new(100, 240, 180),       // Aqua
            border_subtle: RgbColor::new(48, 50, 68),
            selection_alt_bg: RgbColor::new(72, 56, 110),
            stat_cpu_blue: RgbColor::new(110, 160, 255),
            stat_cpu_cyan: RgbColor::new(86, 184, 255),
            stat_cpu_yellow: RgbColor::new(230, 180, 80),
            stat_cpu_light: RgbColor::new(150, 170, 220),
            stat_progress_bg: RgbColor::new(70, 60, 110),
        }
    }

    pub fn preset_forest() -> Self {
        Self {
            accent_primary: RgbColor::new(126, 196, 102),   // Moss green
            accent_secondary: RgbColor::new(86, 168, 142),  // Pine teal
            selection_bg: RgbColor::new(66, 116, 84),       // Deep fern
            selection_fg: RgbColor::new(255, 255, 255),     // White
            border_active: RgbColor::new(126, 196, 102),    // Moss green
            border_inactive: RgbColor::new(76, 88, 82),    // Bark slate
            header_fg: RgbColor::new(182, 226, 164),        // Pale leaf
            danger: RgbColor::new(230, 80, 80),            // Autumn red
            warning: RgbColor::new(230, 180, 60),          // Golden
            success: RgbColor::new(100, 220, 120),        // Bright green
            muted: RgbColor::new(108, 118, 108),            // Moss gray
            info: RgbColor::new(130, 200, 180),            // Sage
            file_code: RgbColor::new(180, 210, 120),       // Lime
            file_config: RgbColor::new(120, 180, 200),     // Forest blue
            file_media: RgbColor::new(200, 160, 120),      // Wood
            file_archive: RgbColor::new(180, 140, 100),    // Bark
            file_exec: RgbColor::new(140, 220, 140),       // Leaf green
            border_subtle: RgbColor::new(50, 58, 50),
            selection_alt_bg: RgbColor::new(50, 72, 60),
            stat_cpu_blue: RgbColor::new(100, 170, 230),
            stat_cpu_cyan: RgbColor::new(80, 190, 200),
            stat_cpu_yellow: RgbColor::new(210, 180, 60),
            stat_cpu_light: RgbColor::new(150, 180, 160),
            stat_progress_bg: RgbColor::new(60, 70, 30),
        }
    }

    pub fn preset_sunset() -> Self {
        Self {
            accent_primary: RgbColor::new(236, 146, 98),   // Orange coral
            accent_secondary: RgbColor::new(236, 99, 141),  // Pink coral
            selection_bg: RgbColor::new(142, 74, 92),       // Plum dusk
            selection_fg: RgbColor::new(255, 255, 255),     // White
            border_active: RgbColor::new(236, 146, 98),    // Orange coral
            border_inactive: RgbColor::new(94, 78, 92),    // Dusk slate
            header_fg: RgbColor::new(255, 198, 156),        // Peach light
            danger: RgbColor::new(255, 70, 70),            // Fire red
            warning: RgbColor::new(255, 200, 60),          // Sun yellow
            success: RgbColor::new(100, 210, 140),         // Seafoam
            muted: RgbColor::new(130, 112, 120),           // Dusk gray
            info: RgbColor::new(200, 160, 220),            // Twilight purple
            file_code: RgbColor::new(240, 160, 120),       // Salmon
            file_config: RgbColor::new(180, 160, 240),     // Lavender
            file_media: RgbColor::new(255, 140, 180),      // Rose
            file_archive: RgbColor::new(220, 180, 140),    // Sand
            file_exec: RgbColor::new(160, 220, 160),      // Spring green
            border_subtle: RgbColor::new(56, 46, 56),
            selection_alt_bg: RgbColor::new(82, 52, 72),
            stat_cpu_blue: RgbColor::new(130, 150, 255),
            stat_cpu_cyan: RgbColor::new(200, 140, 220),
            stat_cpu_yellow: RgbColor::new(240, 170, 80),
            stat_cpu_light: RgbColor::new(180, 160, 200),
            stat_progress_bg: RgbColor::new(82, 52, 52),
        }
    }

    pub fn preset_mono() -> Self {
        Self {
            accent_primary: RgbColor::new(210, 214, 224),   // Soft silver
            accent_secondary: RgbColor::new(162, 172, 188),  // Steel blue-gray
            selection_bg: RgbColor::new(82, 90, 108),        // Graphite
            selection_fg: RgbColor::new(255, 255, 255),     // White
            border_active: RgbColor::new(210, 214, 224),    // Soft silver
            border_inactive: RgbColor::new(72, 78, 92),    // Slate dark
            header_fg: RgbColor::new(228, 232, 240),        // Light silver
            danger: RgbColor::new(220, 80, 80),            // Bright red (needs contrast in mono)
            warning: RgbColor::new(220, 200, 80),          // Bright yellow
            success: RgbColor::new(80, 200, 120),          // Bright green
            muted: RgbColor::new(100, 104, 116),           // Mid gray
            info: RgbColor::new(140, 180, 220),            // Muted blue
            file_code: RgbColor::new(190, 194, 206),       // Light gray
            file_config: RgbColor::new(150, 160, 180),     // Steel
            file_media: RgbColor::new(180, 170, 200),      // Mauve gray
            file_archive: RgbColor::new(170, 160, 150),    // Warm gray
            file_exec: RgbColor::new(140, 200, 160),       // Pale green
            border_subtle: RgbColor::new(50, 52, 62),
            selection_alt_bg: RgbColor::new(66, 60, 80),
            stat_cpu_blue: RgbColor::new(150, 180, 220),
            stat_cpu_cyan: RgbColor::new(140, 180, 200),
            stat_cpu_yellow: RgbColor::new(200, 200, 120),
            stat_cpu_light: RgbColor::new(170, 175, 195),
            stat_progress_bg: RgbColor::new(70, 70, 70),
        }
    }

    pub fn preset_legacy_red() -> Self {
        Self {
            accent_primary: RgbColor::new(226, 78, 86),    // Legacy red
            accent_secondary: RgbColor::new(72, 190, 182),  // Legacy teal
            selection_bg: RgbColor::new(226, 78, 86),       // Legacy red highlight
            selection_fg: RgbColor::new(255, 255, 255),    // White
            border_active: RgbColor::new(226, 78, 86),     // Legacy red
            border_inactive: RgbColor::new(70, 88, 104),   // Blue-gray slate
            header_fg: RgbColor::new(156, 214, 206),       // Pale teal
            danger: RgbColor::new(255, 60, 60),            // Pure red
            warning: RgbColor::new(255, 200, 50),          // Bright yellow
            success: RgbColor::new(80, 200, 120),         // Green
            muted: RgbColor::new(100, 110, 130),           // Blue-gray
            info: RgbColor::new(100, 200, 220),            // Teal
            file_code: RgbColor::new(236, 156, 116),       // Apricot
            file_config: RgbColor::new(132, 190, 255),     // Sky blue
            file_media: RgbColor::new(201, 156, 244),       // Lilac
            file_archive: RgbColor::new(238, 132, 170),    // Rose
            file_exec: RgbColor::new(118, 203, 125),       // Green
            border_subtle: RgbColor::new(40, 45, 55),
            selection_alt_bg: RgbColor::new(78, 58, 112),
            stat_cpu_blue: RgbColor::new(88, 166, 255),
            stat_cpu_cyan: RgbColor::new(80, 200, 255),
            stat_cpu_yellow: RgbColor::new(255, 170, 0),
            stat_cpu_light: RgbColor::new(140, 165, 210),
            stat_progress_bg: RgbColor::new(85, 80, 20),
        }
    }

    pub fn default_purple() -> Self {
        // Match the original DraconTheme::cyberpunk() base colors —
        // Warm Amber accent, not Legacy Red.
        Self {
            accent_primary: RgbColor::new(224, 164, 90),   // Warm Amber
            accent_secondary: RgbColor::new(94, 199, 178),  // Mint-Teal
            selection_bg: RgbColor::new(178, 122, 64),      // Muted Bronze
            selection_fg: RgbColor::new(0, 0, 0),           // Black
            border_active: RgbColor::new(224, 164, 90),     // Warm Amber
            border_inactive: RgbColor::new(86, 88, 98),     // Neutral Slate
            header_fg: RgbColor::new(240, 196, 138),        // Sand
            danger: RgbColor::new(255, 80, 80),            // Bright red
            warning: RgbColor::new(255, 190, 50),          // Amber yellow
            success: RgbColor::new(80, 210, 120),           // Vivid green
            muted: RgbColor::new(120, 120, 135),            // Warm gray
            info: RgbColor::new(100, 200, 230),            // Sky cyan
            file_code: RgbColor::new(236, 156, 116),       // Apricot
            file_config: RgbColor::new(132, 190, 255),     // Sky blue
            file_media: RgbColor::new(201, 156, 244),       // Lilac
            file_archive: RgbColor::new(238, 132, 170),    // Rose
            file_exec: RgbColor::new(118, 203, 125),        // Green
            border_subtle: RgbColor::new(40, 45, 55),
            selection_alt_bg: RgbColor::new(78, 58, 112),
            stat_cpu_blue: RgbColor::new(88, 166, 255),
            stat_cpu_cyan: RgbColor::new(80, 200, 255),
            stat_cpu_yellow: RgbColor::new(255, 170, 0),
            stat_cpu_light: RgbColor::new(140, 165, 210),
            stat_progress_bg: RgbColor::new(85, 80, 20),
        }
    }

    pub fn preset_nord() -> Self {
        Self {
            accent_primary: RgbColor::new(136, 192, 208),   // Nord frost blue
            accent_secondary: RgbColor::new(163, 190, 140),  // Nord aurora green
            selection_bg: RgbColor::new(94, 129, 172),       // Nord blue
            selection_fg: RgbColor::new(236, 239, 244),      // Nord snow
            border_active: RgbColor::new(136, 192, 208),     // Frost
            border_inactive: RgbColor::new(76, 86, 106),    // Nord polar night
            header_fg: RgbColor::new(216, 222, 233),         // Nord snow 1
            danger: RgbColor::new(235, 111, 146),            // Nord red aurora
            warning: RgbColor::new(235, 203, 139),          // Nord yellow aurora
            success: RgbColor::new(163, 190, 140),           // Nord green aurora
            muted: RgbColor::new(108, 112, 134),            // Nord polar night 3
            info: RgbColor::new(180, 142, 173),             // Nord purple aurora
            file_code: RgbColor::new(136, 192, 208),        // Frost blue
            file_config: RgbColor::new(180, 142, 173),      // Purple aurora
            file_media: RgbColor::new(208, 135, 112),       // Nord orange aurora
            file_archive: RgbColor::new(235, 111, 146),    // Red aurora
            file_exec: RgbColor::new(163, 190, 140),        // Green aurora
            border_subtle: RgbColor::new(46, 52, 64),
            selection_alt_bg: RgbColor::new(60, 56, 90),
            stat_cpu_blue: RgbColor::new(136, 192, 208),
            stat_cpu_cyan: RgbColor::new(120, 180, 220),
            stat_cpu_yellow: RgbColor::new(235, 203, 139),
            stat_cpu_light: RgbColor::new(160, 180, 200),
            stat_progress_bg: RgbColor::new(60, 66, 80),
        }
    }

    pub fn preset_dracula() -> Self {
        Self {
            accent_primary: RgbColor::new(189, 147, 249),   // Dracula purple
            accent_secondary: RgbColor::new(80, 250, 123),   // Dracula green
            selection_bg: RgbColor::new(98, 114, 164),       // Dracula comment blue
            selection_fg: RgbColor::new(248, 248, 242),      // Dracula foreground
            border_active: RgbColor::new(189, 147, 249),    // Purple
            border_inactive: RgbColor::new(68, 71, 90),     // Dracula current line
            header_fg: RgbColor::new(248, 248, 242),         // Foreground
            danger: RgbColor::new(255, 85, 85),             // Dracula red
            warning: RgbColor::new(241, 250, 140),           // Dracula yellow
            success: RgbColor::new(80, 250, 123),           // Dracula green
            muted: RgbColor::new(98, 114, 164),             // Comment blue
            info: RgbColor::new(139, 233, 253),             // Dracula cyan
            file_code: RgbColor::new(255, 121, 198),        // Dracula pink
            file_config: RgbColor::new(139, 233, 253),      // Dracula cyan
            file_media: RgbColor::new(255, 184, 108),        // Dracula orange
            file_archive: RgbColor::new(255, 85, 85),      // Dracula red
            file_exec: RgbColor::new(80, 250, 123),        // Dracula green
            border_subtle: RgbColor::new(48, 50, 68),
            selection_alt_bg: RgbColor::new(68, 71, 90),
            stat_cpu_blue: RgbColor::new(98, 114, 164),
            stat_cpu_cyan: RgbColor::new(139, 233, 253),
            stat_cpu_yellow: RgbColor::new(241, 250, 140),
            stat_cpu_light: RgbColor::new(200, 180, 255),
            stat_progress_bg: RgbColor::new(68, 71, 90),
        }
    }

    pub fn preset_solarized_dark() -> Self {
        Self {
            accent_primary: RgbColor::new(181, 137, 0),     // Solarized yellow
            accent_secondary: RgbColor::new(42, 161, 152),   // Solarized cyan
            selection_bg: RgbColor::new(7, 54, 66),          // Solarized base02
            selection_fg: RgbColor::new(147, 161, 161),      // Solarized base1
            border_active: RgbColor::new(181, 137, 0),      // Yellow
            border_inactive: RgbColor::new(54, 62, 68),     // Base02 variant
            header_fg: RgbColor::new(147, 161, 161),        // Base1
            danger: RgbColor::new(220, 50, 47),             // Solarized red
            warning: RgbColor::new(203, 75, 22),           // Solarized orange
            success: RgbColor::new(133, 153, 0),           // Solarized green
            muted: RgbColor::new(88, 110, 117),            // Solarized base01
            info: RgbColor::new(38, 139, 210),              // Solarized blue
            file_code: RgbColor::new(181, 137, 0),          // Yellow
            file_config: RgbColor::new(38, 139, 210),       // Blue
            file_media: RgbColor::new(108, 113, 196),       // Solarized violet
            file_archive: RgbColor::new(203, 75, 22),      // Orange
            file_exec: RgbColor::new(133, 153, 0),        // Green
            border_subtle: RgbColor::new(40, 45, 55),
            selection_alt_bg: RgbColor::new(78, 58, 112),
            stat_cpu_blue: RgbColor::new(88, 166, 255),
            stat_cpu_cyan: RgbColor::new(80, 200, 255),
            stat_cpu_yellow: RgbColor::new(255, 170, 0),
            stat_cpu_light: RgbColor::new(140, 165, 210),
            stat_progress_bg: RgbColor::new(85, 80, 20),
        }
    }

    pub fn preset_one_dark() -> Self {
        Self {
            accent_primary: RgbColor::new(198, 120, 221),   // One Dark purple
            accent_secondary: RgbColor::new(86, 182, 194),   // One Dark cyan
            selection_bg: RgbColor::new(61, 68, 83),         // One Dark selection
            selection_fg: RgbColor::new(255, 255, 255),      // White
            border_active: RgbColor::new(198, 120, 221),    // Purple
            border_inactive: RgbColor::new(55, 61, 74),     // One Dark gutter
            header_fg: RgbColor::new(171, 178, 191),         // One Dark silver
            danger: RgbColor::new(224, 108, 117),           // One Dark red
            warning: RgbColor::new(209, 154, 102),          // One Dark yellow
            success: RgbColor::new(152, 195, 121),          // One Dark green
            muted: RgbColor::new(92, 99, 112),             // One Dark comment gray
            info: RgbColor::new(86, 182, 194),             // One Dark cyan
            file_code: RgbColor::new(224, 108, 117),        // Red (function)
            file_config: RgbColor::new(86, 182, 194),       // Cyan (keyword)
            file_media: RgbColor::new(209, 154, 102),       // Yellow (string)
            file_archive: RgbColor::new(198, 120, 221),    // Purple (type)
            file_exec: RgbColor::new(152, 195, 121),      // Green (variable)
            border_subtle: RgbColor::new(50, 56, 72),
            selection_alt_bg: RgbColor::new(55, 61, 78),
            stat_cpu_blue: RgbColor::new(86, 182, 194),
            stat_cpu_cyan: RgbColor::new(120, 160, 220),
            stat_cpu_yellow: RgbColor::new(209, 154, 102),
            stat_cpu_light: RgbColor::new(160, 170, 200),
            stat_progress_bg: RgbColor::new(55, 61, 78),
        }
    }

    pub fn preset_tokyo_night() -> Self {
        Self {
            accent_primary: RgbColor::new(125, 207, 255),   // Tokyo Night blue
            accent_secondary: RgbColor::new(187, 154, 247),  // Tokyo Night purple
            selection_bg: RgbColor::new(53, 59, 83),         // Tokyo Night selection
            selection_fg: RgbColor::new(255, 255, 255),      // White
            border_active: RgbColor::new(125, 207, 255),    // Blue
            border_inactive: RgbColor::new(42, 45, 62),     // Tokyo Night dark
            header_fg: RgbColor::new(192, 202, 245),         // Tokyo Night foreground dim
            danger: RgbColor::new(247, 118, 142),           // Tokyo Night red
            warning: RgbColor::new(255, 158, 100),          // Tokyo Night orange
            success: RgbColor::new(158, 206, 106),          // Tokyo Night green
            muted: RgbColor::new(82, 87, 113),             // Tokyo Night comment
            info: RgbColor::new(125, 207, 255),            // Blue
            file_code: RgbColor::new(247, 118, 142),        // Red
            file_config: RgbColor::new(187, 154, 247),      // Purple
            file_media: RgbColor::new(255, 158, 100),       // Orange
            file_archive: RgbColor::new(224, 175, 224),    // Pink
            file_exec: RgbColor::new(158, 206, 106),      // Green
            border_subtle: RgbColor::new(40, 45, 55),
            selection_alt_bg: RgbColor::new(78, 58, 112),
            stat_cpu_blue: RgbColor::new(88, 166, 255),
            stat_cpu_cyan: RgbColor::new(80, 200, 255),
            stat_cpu_yellow: RgbColor::new(255, 170, 0),
            stat_cpu_light: RgbColor::new(140, 165, 210),
            stat_progress_bg: RgbColor::new(85, 80, 20),
        }
    }

    fn apply_to_theme(&self, theme: &mut DraconTheme) {
        theme.accent_primary = self.accent_primary.to_color();
        theme.accent_secondary = self.accent_secondary.to_color();
        theme.selection_bg = self.selection_bg.to_color();
        theme.selection_fg = self.selection_fg.to_color();
        theme.border_active = self.border_active.to_color();
        theme.border_inactive = self.border_inactive.to_color();
        theme.header_fg = self.header_fg.to_color();
        theme.danger = self.danger.to_color();
        theme.warning = self.warning.to_color();
        theme.success = self.success.to_color();
        theme.muted = self.muted.to_color();
        theme.info = self.info.to_color();
        theme.file_code = self.file_code.to_color();
        theme.file_config = self.file_config.to_color();
        theme.file_media = self.file_media.to_color();
        theme.file_archive = self.file_archive.to_color();
        theme.file_exec = self.file_exec.to_color();
        theme.border_subtle = self.border_subtle.to_color();
        theme.selection_alt_bg = self.selection_alt_bg.to_color();
        theme.stat_cpu_blue = self.stat_cpu_blue.to_color();
        theme.stat_cpu_cyan = self.stat_cpu_cyan.to_color();
        theme.stat_cpu_yellow = self.stat_cpu_yellow.to_color();
        theme.stat_cpu_light = self.stat_cpu_light.to_color();
        theme.stat_progress_bg = self.stat_progress_bg.to_color();
    }
}

impl DraconTheme {
    pub fn cyberpunk() -> Self {
        Self {
            bg: Color::Rgb(0, 0, 0),                    // True Color Pure Black
            fg: Color::Rgb(255, 255, 255),              // Pure White
            accent_primary: Color::Rgb(224, 164, 90),   // Warm Amber
            accent_secondary: Color::Rgb(94, 199, 178), // Mint-Teal
            selection_bg: Color::Rgb(178, 122, 64),     // Muted Bronze
            selection_fg: Color::Rgb(0, 0, 0),          // Black (for contrast)
            border_active: Color::Rgb(224, 164, 90),    // Primary Accent
            border_inactive: Color::Rgb(86, 88, 98),    // Neutral Slate
            header_fg: Color::Rgb(240, 196, 138),       // Sand
            danger: Color::Rgb(255, 80, 80),           // Bright red
            warning: Color::Rgb(255, 190, 50),          // Amber yellow
            success: Color::Rgb(80, 210, 120),          // Vivid green
            muted: Color::Rgb(120, 120, 135),           // Warm gray
            info: Color::Rgb(100, 200, 230),            // Sky cyan
            file_code: Color::Rgb(236, 156, 116),       // Apricot
            file_config: Color::Rgb(132, 190, 255),     // Sky Blue
            file_media: Color::Rgb(201, 156, 244),      // Lilac
            file_archive: Color::Rgb(238, 132, 170),    // Rose
            file_exec: Color::Rgb(118, 203, 125),       // Green
            border_subtle: Color::Rgb(40, 45, 55),       // Very dark border
            selection_alt_bg: Color::Rgb(78, 58, 112),   // Purple-tint multi-select
            stat_cpu_blue: Color::Rgb(88, 166, 255),     // CPU bar blue
            stat_cpu_cyan: Color::Rgb(80, 200, 255),     // CPU bar cyan
            stat_cpu_yellow: Color::Rgb(255, 170, 0),    // CPU bar yellow
            stat_cpu_light: Color::Rgb(140, 165, 210),   // CPU bar light
            stat_progress_bg: Color::Rgb(85, 80, 20),    // Task progress bg
        }
    }
}

pub static THEME: std::sync::LazyLock<DraconTheme> =
    std::sync::LazyLock::new(DraconTheme::cyberpunk);

static ACTIVE_STYLE: LazyLock<RwLock<ThemeStyle>> =
    LazyLock::new(|| RwLock::new(ThemeStyle::default_purple()));
static ACTIVE_THEME: LazyLock<RwLock<DraconTheme>> = LazyLock::new(|| {
    let mut theme = DraconTheme::cyberpunk();
    ThemeStyle::default_purple().apply_to_theme(&mut theme);
    RwLock::new(theme)
});

pub fn style_settings() -> ThemeStyle {
    ACTIVE_STYLE.read().clone()
}

pub fn set_style_settings(style: ThemeStyle) {
    {
        let mut active_style = ACTIVE_STYLE.write();
        *active_style = style.clone();
    }
    {
        let mut active_theme = ACTIVE_THEME.write();
        let mut theme = DraconTheme::cyberpunk();
        style.apply_to_theme(&mut theme);
        *active_theme = theme;
    }
}

// --- Themed color accessors ---

pub fn accent_primary() -> Color { ACTIVE_THEME.read().accent_primary }
pub fn accent_secondary() -> Color { ACTIVE_THEME.read().accent_secondary }
pub fn selection_bg() -> Color { ACTIVE_THEME.read().selection_bg }
pub fn selection_fg() -> Color { ACTIVE_THEME.read().selection_fg }
pub fn border_active() -> Color { ACTIVE_THEME.read().border_active }
pub fn border_inactive() -> Color { ACTIVE_THEME.read().border_inactive }
pub fn header_fg() -> Color { ACTIVE_THEME.read().header_fg }

pub fn danger() -> Color { ACTIVE_THEME.read().danger }
pub fn warning() -> Color { ACTIVE_THEME.read().warning }
pub fn success() -> Color { ACTIVE_THEME.read().success }
pub fn muted() -> Color { ACTIVE_THEME.read().muted }
pub fn info() -> Color { ACTIVE_THEME.read().info }

pub fn file_code() -> Color { ACTIVE_THEME.read().file_code }
pub fn file_config() -> Color { ACTIVE_THEME.read().file_config }
pub fn file_media() -> Color { ACTIVE_THEME.read().file_media }
pub fn file_archive() -> Color { ACTIVE_THEME.read().file_archive }
pub fn file_exec() -> Color { ACTIVE_THEME.read().file_exec }

pub fn border_subtle() -> Color { ACTIVE_THEME.read().border_subtle }
pub fn selection_alt_bg() -> Color { ACTIVE_THEME.read().selection_alt_bg }
pub fn stat_cpu_blue() -> Color { ACTIVE_THEME.read().stat_cpu_blue }
pub fn stat_cpu_cyan() -> Color { ACTIVE_THEME.read().stat_cpu_cyan }
pub fn stat_cpu_yellow() -> Color { ACTIVE_THEME.read().stat_cpu_yellow }
pub fn stat_cpu_light() -> Color { ACTIVE_THEME.read().stat_cpu_light }
pub fn stat_progress_bg() -> Color { ACTIVE_THEME.read().stat_progress_bg }

// --- Base colors (from DraconTheme, not per-preset) ---

pub fn fg() -> Color { ACTIVE_THEME.read().fg }
pub fn bg() -> Color { ACTIVE_THEME.read().bg }

// --- Monitor-specific colors (not per-theme, kept as constants) ---

pub fn gauge_danger() -> Color { Color::Rgb(255, 60, 60) }
pub fn gauge_warning() -> Color { Color::Rgb(255, 180, 0) }
pub fn monitor_label() -> Color { Color::Rgb(60, 65, 75) }
pub fn monitor_dim() -> Color { Color::Rgb(100, 100, 110) }
pub fn monitor_separator() -> Color { Color::Rgb(30, 30, 35) }
pub fn monitor_row_even() -> Color { Color::Rgb(180, 185, 190) }
pub fn monitor_row_odd() -> Color { Color::Rgb(140, 145, 150) }
