#![allow(unused_imports)]

//! File list mouse index calculation and open-with suggestions.
//! Extracted from event_helpers.rs (Phase 4).

use crate::app::App;
use crate::state::FileColumn;
use crate::ui::theme as theme;

#[allow(dead_code)]
const FILE_LIST_START_ROW: u16 = 3;

pub fn fs_mouse_index(row: u16, app: &App) -> Option<usize> {
    let fs = app.current_file_state()?;
    let offset = fs.view.table_state.offset();
    let rel_row = row.saturating_sub(FILE_LIST_START_ROW) as usize;
    let idx = offset.saturating_add(rel_row);
    if idx >= fs.list.files.len() {
        return None;
    }
    Some(idx)
}

pub fn get_open_with_suggestions(_app: &App, ext: &str) -> Vec<String> {
    dracon_terminal_engine::utils::get_open_with_suggestions(ext)
}

