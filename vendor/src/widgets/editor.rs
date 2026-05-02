use crate::input::event::{
    Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use crate::utils::highlight_code;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use std::cell::RefCell;

/// A tactical multiline text editor widget for quick edits.
#[derive(Clone, Debug)]
pub struct TextEditor {
    pub lines: Vec<String>,
    pub cursor_row: usize, // Visual Row Index
    pub cursor_col: usize, // Byte index
    pub scroll_row: usize,
    pub scroll_col: usize, // Visual column index
    pub style: Style,
    pub cursor_style: Style,
    pub modified: bool,
    pub show_line_numbers: bool,
    pub history: Vec<Vec<String>>,
    pub redo_stack: Vec<Vec<String>>,
    pub filter_query: String,
    pub filtered_indices: Vec<usize>,
    pub read_only: bool,
    pub selection_start: Option<(usize, usize)>, // (row, byte_col)
    pub selection_end: Option<(usize, usize)>,
    pub is_selecting: bool,
    pub is_dragging_selection: bool,
    pub language: String,
    pub wrap: bool,
    pub highlighted_cache: RefCell<Vec<Line<'static>>>,
    pub first_invalid_line: RefCell<Option<usize>>,
}

impl Default for TextEditor {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            style: Style::default().fg(Color::Rgb(255, 255, 255)),
            cursor_style: Style::default()
                .bg(Color::Rgb(88, 166, 255))
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            modified: false,
            show_line_numbers: true,
            history: Vec::new(),
            redo_stack: Vec::new(),
            filter_query: String::new(),
            filtered_indices: Vec::new(),
            read_only: false,
            selection_start: None,
            selection_end: None,
            is_selecting: false,
            is_dragging_selection: false,
            language: String::new(),
            wrap: false,
            highlighted_cache: RefCell::new(Vec::new()),
            first_invalid_line: RefCell::new(Some(0)),
        }
    }
}

impl TextEditor {
    pub fn set_filter(&mut self, query: &str) {
        if self.filter_query == query {
            return;
        }

        // If clearing filter, restore cursor to real line index
        if query.is_empty() && !self.filter_query.is_empty() {
            if self.cursor_row < self.filtered_indices.len() {
                self.cursor_row = self.filtered_indices[self.cursor_row];
            } else if !self.filtered_indices.is_empty() {
                self.cursor_row = *self.filtered_indices.last().unwrap();
            } else {
                self.cursor_row = 0;
            }
            self.filtered_indices.clear();
        }

        self.filter_query = query.to_string();

        if !self.filter_query.is_empty() {
            self.filtered_indices = self
                .lines
                .iter()
                .enumerate()
                .filter(|(_, line)| {
                    line.to_lowercase()
                        .contains(&self.filter_query.to_lowercase())
                })
                .map(|(i, _)| i)
                .collect();
            self.cursor_row = 0;
            self.scroll_row = 0;
        }
        self.scroll_col = 0;
        self.cursor_col = 0;
        self.invalidate_from(0);
    }

    fn effective_len(&self) -> usize {
        if !self.filter_query.is_empty() {
            self.filtered_indices.len()
        } else {
            self.lines.len()
        }
    }

    fn get_effective_line(&self, idx: usize) -> &String {
        if idx >= self.effective_len() {
            return &self.lines[0];
        } // Fallback safety
        if !self.filter_query.is_empty() {
            &self.lines[self.filtered_indices[idx]]
        } else {
            &self.lines[idx]
        }
    }

    fn get_real_line_idx(&self, idx: usize) -> usize {
        if !self.filter_query.is_empty() {
            if idx < self.filtered_indices.len() {
                self.filtered_indices[idx]
            } else {
                0
            }
        } else {
            idx
        }
    }

    pub fn get_visual_x(&self, row: usize, byte_col: usize) -> usize {
        if row >= self.effective_len() {
            return 0;
        }
        let line = self.get_effective_line(row);
        if byte_col > line.len() {
            return 0;
        }
        line[..byte_col].width()
    }

    pub fn get_byte_index_from_visual(&self, row: usize, visual_x: usize) -> usize {
        if row >= self.effective_len() {
            return 0;
        }
        let line = self.get_effective_line(row);
        let mut width = 0;
        for (i, c) in line.char_indices() {
            if width >= visual_x {
                return i;
            }
            width += c.width().unwrap_or(0);
        }
        line.len()
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn invalidate_from(&self, row: usize) {
        let mut first = self.first_invalid_line.borrow_mut();
        if let Some(current) = *first {
            if row < current {
                *first = Some(row);
            }
        } else {
            *first = Some(row);
        }
    }

    pub fn with_content(content: &str) -> Self {
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        // Always ensure a trailing empty line for "extra line after everything"
        if !lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            lines.push(String::new());
        }
        Self {
            lines,
            ..Default::default()
        }
    }

    pub fn get_content(&self) -> String {
        self.lines.join("\n")
    }

    pub fn replace_all(&mut self, find: &str, replace: &str) {
        if find.is_empty() {
            return;
        }
        for line in &mut self.lines {
            *line = line.replace(find, replace);
        }
        self.modified = true;
        self.invalidate_from(0);
    }

    pub fn replace_next(&mut self, find: &str, replace: &str) -> bool {
        if find.is_empty() {
            return false;
        }

        // Search from current cursor position
        let start_row = self.cursor_row;
        let start_col = self.cursor_col;

        for r in 0..self.lines.len() {
            let row = (start_row + r) % self.lines.len();
            let line = &self.lines[row];
            let search_from = if r == 0 { start_col } else { 0 };

            if search_from < line.len() {
                if let Some(col) = line[search_from..].find(find) {
                    let actual_col = search_from + col;
                    let mut new_line = line.clone();
                    new_line.replace_range(actual_col..actual_col + find.len(), replace);
                    self.lines[row] = new_line;

                    self.cursor_row = row;
                    self.cursor_col = actual_col + replace.len();
                    self.modified = true;
                    self.invalidate_from(0);
                    return true;
                }
            }
        }
        false
    }

    pub fn gutter_width(&self) -> usize {
        if !self.show_line_numbers {
            return 0;
        }
        let total_lines = self.lines.len();
        let digits = total_lines.to_string().len();
        digits + 2 // +1 for left padding, +1 for vertical separator
    }

    pub fn handle_event(&mut self, event: &Event, area: Rect) -> bool {
        // If filtered OR Read-Only, allow only navigation
        if !self.filter_query.is_empty() || self.read_only {
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    return false;
                }
                let has_control = key.modifiers.contains(KeyModifiers::CONTROL);
                let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);

                match key.code {
                    KeyCode::Up | KeyCode::Char('p') if has_control => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_up();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Down | KeyCode::Char('n') if has_control => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_down();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Up => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_up();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Down => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_down();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Left => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_left();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Right => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_right();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::PageUp => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.cursor_row = self.cursor_row.saturating_sub(area.height as usize);
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::PageDown => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.cursor_row = std::cmp::min(
                            self.effective_len().saturating_sub(1),
                            self.cursor_row + area.height as usize,
                        );
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Home => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        let line = self.get_effective_line(self.cursor_row);
                        let first_non_whitespace = line
                            .chars()
                            .enumerate()
                            .find(|(_, c)| !c.is_whitespace())
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        if self.cursor_col == first_non_whitespace {
                            self.cursor_col = 0;
                        } else {
                            self.cursor_col = first_non_whitespace;
                        }
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::End => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.cursor_col = self.get_effective_line(self.cursor_row).len();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    _ => {}
                }
            }

            return false;
        }

        match event {
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return false;
                }

                let has_control = key.modifiers.contains(KeyModifiers::CONTROL);
                let has_alt = key.modifiers.contains(KeyModifiers::ALT);
                let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);

                match key.code {
                    KeyCode::Char(c) if !has_control && !has_alt => {
                        self.push_history();
                        if self.selection_start.is_some() {
                            self.delete_selection();
                        }
                        self.insert_char(c);
                        self.modified = true;
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Tab => {
                        self.push_history();
                        if self.selection_start.is_some() {
                            self.delete_selection();
                        }
                        for _ in 0..4 {
                            self.insert_char(' ');
                        }
                        self.modified = true;
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::BackTab => {
                        // Shift + Tab
                        self.push_history();
                        let line = &mut self.lines[self.cursor_row];
                        let mut spaces_removed = 0;
                        while spaces_removed < 4 && line.starts_with(' ') {
                            line.remove(0);
                            spaces_removed += 1;
                        }
                        if spaces_removed > 0 {
                            self.cursor_col = self.cursor_col.saturating_sub(spaces_removed);
                            self.modified = true;
                            self.invalidate_from(0);
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Char('z') if has_control => {
                        if let Some(prev) = self.history.pop() {
                            self.redo_stack.push(self.lines.clone());
                            self.lines = prev;
                            self.cursor_row = std::cmp::min(self.cursor_row, self.lines.len() - 1);
                            self.cursor_col =
                                std::cmp::min(self.cursor_col, self.lines[self.cursor_row].len());
                            self.clear_selection();
                            self.modified = true;
                            self.invalidate_from(0);
                            self.ensure_cursor_visible(area);
                            return true;
                        }
                    }
                    KeyCode::Char('y') if has_control => {
                        if let Some(next) = self.redo_stack.pop() {
                            self.history.push(self.lines.clone());
                            self.lines = next;
                            self.cursor_row = std::cmp::min(self.cursor_row, self.lines.len() - 1);
                            self.cursor_col =
                                std::cmp::min(self.cursor_col, self.lines[self.cursor_row].len());
                            self.clear_selection();
                            self.modified = true;
                            self.invalidate_from(0);
                            self.ensure_cursor_visible(area);
                            return true;
                        }
                    }
                    KeyCode::Char('a') if has_control => {
                        self.select_all();
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Char('d') if has_control => {
                        self.push_history();
                        let current_line = self.lines[self.cursor_row].clone();
                        self.lines.insert(self.cursor_row + 1, current_line);
                        self.cursor_row += 1;
                        self.modified = true;
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Char('k') if has_control => {
                        self.push_history();
                        if self.cursor_col >= self.lines[self.cursor_row].len() {
                            if self.cursor_row < self.lines.len() - 1 {
                                let next_line = self.lines.remove(self.cursor_row + 1);
                                self.lines[self.cursor_row].push_str(&next_line);
                            }
                        } else {
                            let line = &mut self.lines[self.cursor_row];
                            line.truncate(self.cursor_col);
                        }
                        self.modified = true;
                        return true;
                    }

                    KeyCode::Char('u') if has_control => {
                        self.push_history();
                        let line = &mut self.lines[self.cursor_row];
                        if self.cursor_col > 0 {
                            *line = line.split_off(self.cursor_col);
                            self.cursor_col = 0;
                        }
                        self.modified = true;
                        return true;
                    }
                    KeyCode::Char('w') if has_control => {
                        self.push_history();
                        if self.selection_start.is_some() {
                            self.delete_selection();
                        } else {
                            self.delete_word_backwards();
                        }
                        self.modified = true;
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Enter => {
                        self.push_history();
                        if self.selection_start.is_some() {
                            self.delete_selection();
                        }
                        self.insert_newline();
                        self.modified = true;
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Backspace if has_control || has_alt => {
                        self.push_history();
                        if self.selection_start.is_some() {
                            self.delete_selection();
                        } else {
                            self.delete_word_backwards();
                        }
                        self.modified = true;
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Backspace => {
                        if self.selection_start.is_some() {
                            self.push_history();
                            self.delete_selection();
                            self.modified = true;
                            self.ensure_cursor_visible(area);
                            return true;
                        }
                        if self.delete_backwards() {
                            self.push_history();
                            self.modified = true;
                            self.ensure_cursor_visible(area);
                            return true;
                        }
                    }
                    KeyCode::Delete if has_control || has_alt => {
                        self.push_history();
                        if self.selection_start.is_some() {
                            self.delete_selection();
                        } else {
                            self.delete_word_forwards();
                        }
                        self.modified = true;
                        return true;
                    }
                    KeyCode::Delete => {
                        if self.selection_start.is_some() {
                            self.push_history();
                            self.delete_selection();
                            self.modified = true;
                            return true;
                        }
                        if self.delete_forwards() {
                            self.push_history();
                            self.modified = true;
                            return true;
                        }
                    }
                    KeyCode::Left if has_control || has_alt => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_word_left();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Left => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_left();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Right if has_control || has_alt => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_word_right();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Right => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_right();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    // Emacs bindings
                    KeyCode::Char('b') if has_alt => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_word_left();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Char('f') if has_alt => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_word_right();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    /*
                    KeyCode::Char('b') if has_control => {
                        if has_shift { self.maybe_start_selection(); }
                        self.move_cursor_left();
                        if has_shift { self.update_selection_end(); } else { self.clear_selection(); }
                        self.ensure_cursor_visible(area); return true;
                    }
                    KeyCode::Char('f') if has_control => {
                        if has_shift { self.maybe_start_selection(); }
                        self.move_cursor_right();
                        if has_shift { self.update_selection_end(); } else { self.clear_selection(); }
                        self.ensure_cursor_visible(area); return true;
                    }
                    */
                    KeyCode::Up | KeyCode::Char('p') if has_control => {
                        // Ctrl+p is Up
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_up();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Down | KeyCode::Char('n') if has_control => {
                        // Ctrl+n is Down
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_down();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Up => {
                        if has_alt {
                            self.move_line_up();
                            self.ensure_cursor_visible(area);
                            return true;
                        }
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_up();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Down => {
                        if has_alt {
                            self.move_line_down();
                            self.ensure_cursor_visible(area);
                            return true;
                        }
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.move_cursor_down();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }

                    KeyCode::Home if has_control => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.cursor_row = 0;
                        self.cursor_col = 0;
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::End if has_control => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.cursor_row = self.lines.len().saturating_sub(1);
                        self.cursor_col = self.lines[self.cursor_row].len();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::Home => {
                        if has_shift {
                            self.maybe_start_selection();
                        }

                        let line = &self.lines[self.cursor_row];
                        let first_non_whitespace = line
                            .chars()
                            .enumerate()
                            .find(|(_, c)| !c.is_whitespace())
                            .map(|(i, _)| i)
                            .unwrap_or(0);

                        if self.cursor_col == first_non_whitespace {
                            self.cursor_col = 0;
                        } else {
                            self.cursor_col = first_non_whitespace;
                        }

                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::End => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.cursor_col = self.lines[self.cursor_row].len();
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::PageUp => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.cursor_row = self.cursor_row.saturating_sub(area.height as usize);
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    KeyCode::PageDown => {
                        if has_shift {
                            self.maybe_start_selection();
                        }
                        self.cursor_row = std::cmp::min(
                            self.lines.len() - 1,
                            self.cursor_row + area.height as usize,
                        );
                        if has_shift {
                            self.update_selection_end();
                        } else {
                            self.clear_selection();
                        }
                        self.ensure_cursor_visible(area);
                        return true;
                    }
                    _ => {}
                }
            }
            Event::Mouse(mouse) => {
                return self.handle_mouse_event(*mouse, area);
            }
            _ => {}
        }
        false
    }

    pub fn handle_mouse_event(&mut self, mouse: MouseEvent, area: Rect) -> bool {
        if mouse.column < area.x
            || mouse.column >= area.x + area.width
            || mouse.row < area.y
            || mouse.row >= area.y + area.height
        {
            return false;
        }

        let gutter = self.gutter_width();
        let scrollbar_w = if self.effective_len() > area.height as usize {
            1
        } else {
            0
        };
        let content_width = area
            .width
            .saturating_sub(gutter as u16 + scrollbar_w as u16);
        let rel_row = (mouse.row - area.y) as usize;

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // 1. Scrollbar 'Click-to-Jump' logic (rightmost column)
                if mouse.column >= area.x + area.width.saturating_sub(1) {
                    let total_lines = if self.wrap {
                        // Estimate wrapped lines? For now just use effective_len
                        self.effective_len()
                    } else {
                        self.effective_len()
                    };
                    let view_height = area.height as usize;
                    if total_lines > view_height {
                        let percent = rel_row as f32 / view_height as f32;
                        self.scroll_row = (percent * total_lines as f32) as usize;
                        self.scroll_row =
                            self.scroll_row.min(total_lines.saturating_sub(view_height));
                    }
                    return true;
                }

                let (target_row, target_col) = if self.wrap {
                    // In wrap mode, scroll_row is screen line index.
                    let screen_row = self.scroll_row + rel_row;

                    // Find which source line and which segment this screen row belongs to
                    let mut current_screen_row = 0;
                    let mut found = None;
                    let width = content_width as usize;

                    for i in 0..self.effective_len() {
                        let line = self.get_effective_line(i);
                        let w = line.width();
                        let segments = if w == 0 {
                            1
                        } else {
                            (w.saturating_sub(1) / width) + 1
                        };

                        if screen_row >= current_screen_row
                            && screen_row < current_screen_row + segments
                        {
                            let segment_idx = screen_row - current_screen_row;
                            let rel_col = (mouse.column - area.x - gutter as u16) as usize;
                            let visual_x = segment_idx * width + rel_col;
                            let byte_idx = self.get_byte_index_from_visual(i, visual_x);
                            found = Some((i, byte_idx));
                            break;
                        }
                        current_screen_row += segments;
                    }
                    found.unwrap_or((self.cursor_row, self.cursor_col))
                } else {
                    let row = self.scroll_row + rel_row;
                    let col = if mouse.column >= area.x + gutter as u16 {
                        let rel_col = (mouse.column - area.x - gutter as u16) as usize;
                        let target_visual = self.scroll_col + rel_col;
                        self.get_byte_index_from_visual(row, target_visual)
                    } else {
                        0
                    };
                    (row, col)
                };

                if target_row < self.effective_len() {
                    if self.is_inside_selection(target_row, target_col) {
                        self.is_dragging_selection = true;
                        self.is_selecting = false;
                        self.cursor_row = target_row;
                        self.cursor_col = target_col;
                        return true;
                    }

                    self.cursor_row = target_row;
                    self.cursor_col = target_col;

                    self.selection_start = Some((self.cursor_row, self.cursor_col));
                    self.selection_end = Some((self.cursor_row, self.cursor_col));
                    self.is_selecting = true;
                    self.is_dragging_selection = false;
                    return true;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                // Check if click/drag is within scrollbar area (rightmost column)
                if mouse.column >= area.x + area.width.saturating_sub(1) {
                    let total_lines = self.effective_len();
                    let view_height = area.height as usize;
                    if total_lines > view_height {
                        let percent = rel_row as f32 / view_height as f32;
                        self.scroll_row = (percent * total_lines as f32) as usize;
                        self.scroll_row =
                            self.scroll_row.min(total_lines.saturating_sub(view_height));
                    }
                    return true;
                }

                let (target_row, target_col) = if self.wrap {
                    let screen_row = self.scroll_row + rel_row;
                    let mut current_screen_row = 0;
                    let mut found = None;
                    let width = content_width as usize;

                    for i in 0..self.effective_len() {
                        let line = self.get_effective_line(i);
                        let w = line.width();
                        let segments = if w == 0 {
                            1
                        } else {
                            (w.saturating_sub(1) / width) + 1
                        };

                        if screen_row >= current_screen_row
                            && screen_row < current_screen_row + segments
                        {
                            let segment_idx = screen_row - current_screen_row;
                            let rel_col =
                                (mouse.column.saturating_sub(area.x + gutter as u16)) as usize;
                            let visual_x = segment_idx * width + rel_col;
                            let byte_idx = self.get_byte_index_from_visual(i, visual_x);
                            found = Some((i, byte_idx));
                            break;
                        }
                        current_screen_row += segments;
                    }
                    found.unwrap_or((self.cursor_row, self.cursor_col))
                } else {
                    let row = self.scroll_row + rel_row;
                    let col = if mouse.column >= area.x + gutter as u16 {
                        let rel_col = (mouse.column - area.x - gutter as u16) as usize;
                        let target_visual = self.scroll_col + rel_col;
                        self.get_byte_index_from_visual(row, target_visual)
                    } else {
                        0
                    };
                    (row, col)
                };

                if target_row < self.effective_len() {
                    self.cursor_row = target_row;
                    self.cursor_col = target_col;

                    if self.is_selecting {
                        self.selection_end = Some((self.cursor_row, self.cursor_col));
                    }
                    // During drag, we just move the cursor (insertion point)
                    return true;
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if self.is_dragging_selection {
                    let r = self.cursor_row;
                    let c = self.cursor_col;
                    self.move_selection_to(r, c);
                    self.is_dragging_selection = false;
                    return true;
                }
                if self.is_selecting {
                    self.is_selecting = false;
                    // If start == end, clear selection
                    if self.selection_start == self.selection_end {
                        self.selection_start = None;
                        self.selection_end = None;
                    }
                }
                return true;
            }
            MouseEventKind::ScrollUp => {
                self.scroll_row = self.scroll_row.saturating_sub(5);
                return true;
            }
            MouseEventKind::ScrollDown => {
                let total_lines = if self.wrap {
                    self.effective_len() * 2
                } else {
                    self.effective_len()
                };

                let max_scroll = total_lines.saturating_sub(area.height as usize);
                self.scroll_row = std::cmp::min(max_scroll, self.scroll_row + 5);
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn push_history(&mut self) {
        if let Some(last) = self.history.last() {
            if last == &self.lines {
                return;
            }
        }
        self.history.push(self.lines.clone());
        if self.history.len() > 100 {
            self.history.remove(0);
        }
        self.redo_stack.clear();
    }

    fn delete_word_backwards(&mut self) {
        if self.cursor_col == 0 && self.cursor_row == 0 {
            return;
        }
        if self.cursor_col == 0 {
            self.delete_backwards();
            return;
        }

        let line = &self.lines[self.cursor_row];
        let mut i = self.cursor_col;

        // Skip trailing whitespace
        while i > 0 {
            if let Some(prev) = line[..i].chars().next_back() {
                if prev.is_whitespace() {
                    i -= prev.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        // Skip the word
        while i > 0 {
            if let Some(prev) = line[..i].chars().next_back() {
                if !prev.is_whitespace() {
                    i -= prev.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let to_remove_bytes = self.cursor_col - i;
        for _ in 0..to_remove_bytes {
            self.delete_backwards();
        }
    }

    fn delete_word_forwards(&mut self) {
        let line = &self.lines[self.cursor_row];
        if self.cursor_col >= line.len() && self.cursor_row >= self.lines.len() - 1 {
            return;
        }
        if self.cursor_col >= line.len() {
            self.delete_forwards();
            return;
        }

        let mut i = self.cursor_col;
        // Skip trailing whitespace
        while i < line.len() {
            if let Some(next) = line[i..].chars().next() {
                if next.is_whitespace() {
                    i += next.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        // Skip the word
        while i < line.len() {
            if let Some(next) = line[i..].chars().next() {
                if !next.is_whitespace() {
                    i += next.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let to_remove_bytes = i - self.cursor_col;
        for _ in 0..to_remove_bytes {
            self.delete_forwards();
        }
    }

    fn move_cursor_word_left(&mut self) {
        if self.cursor_col == 0 {
            self.move_cursor_left();
            return;
        }
        let line = &self.lines[self.cursor_row];
        let mut i = self.cursor_col;
        while i > 0 {
            if let Some(prev) = line[..i].chars().next_back() {
                if prev.is_whitespace() {
                    i -= prev.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        while i > 0 {
            if let Some(prev) = line[..i].chars().next_back() {
                if !prev.is_whitespace() {
                    i -= prev.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        self.cursor_col = i;
    }

    fn move_cursor_word_right(&mut self) {
        let line = &self.lines[self.cursor_row];
        if self.cursor_col >= line.len() {
            self.move_cursor_right();
            return;
        }
        let mut i = self.cursor_col;
        while i < line.len() {
            if let Some(next) = line[i..].chars().next() {
                if next.is_whitespace() {
                    i += next.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        while i < line.len() {
            if let Some(next) = line[i..].chars().next() {
                if !next.is_whitespace() {
                    i += next.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        self.cursor_col = i;
    }

    fn insert_char(&mut self, c: char) {
        if c == '\x1b' {
            return;
        }
        self.ensure_valid_cursor_col();
        let line = &mut self.lines[self.cursor_row];
        line.insert(self.cursor_col, c);
        self.cursor_col += c.len_utf8();
        self.modified = true;
        self.invalidate_from(self.cursor_row);
    }

    pub fn ensure_valid_cursor_col(&mut self) {
        if self.cursor_row >= self.lines.len() {
            self.cursor_row = self.lines.len().saturating_sub(1);
        }
        let line = &self.lines[self.cursor_row];
        if self.cursor_col > line.len() {
            self.cursor_col = line.len();
        }
        while !line.is_char_boundary(self.cursor_col) {
            self.cursor_col = self.cursor_col.saturating_sub(1);
        }
    }

    fn insert_newline(&mut self) {
        self.ensure_valid_cursor_col();
        let line = &self.lines[self.cursor_row];
        let indentation = line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect::<String>();

        let line = &mut self.lines[self.cursor_row];
        let remaining = line.split_off(self.cursor_col);

        let mut new_line = indentation.clone();
        new_line.push_str(&remaining);

        self.lines.insert(self.cursor_row + 1, new_line);
        self.invalidate_from(self.cursor_row);
        self.cursor_row += 1;
        self.cursor_col = indentation.len();
    }

    fn insert_newline_raw(&mut self) {
        self.ensure_valid_cursor_col();
        let line = &mut self.lines[self.cursor_row];
        let remaining = line.split_off(self.cursor_col);
        self.lines.insert(self.cursor_row + 1, remaining);
        self.invalidate_from(self.cursor_row);
        self.cursor_row += 1;
        self.cursor_col = 0;
    }

    fn delete_backwards(&mut self) -> bool {
        self.ensure_valid_cursor_col();
        if self.cursor_col > 0 {
            let line = &mut self.lines[self.cursor_row];

            // Smart Backspace: If we are at an indentation level (4 spaces) and only have spaces before us
            let prefix = &line[..self.cursor_col];
            if self.cursor_col >= 4
                && self.cursor_col.is_multiple_of(4)
                && prefix.chars().all(|c| c == ' ')
            {
                for _ in 0..4 {
                    line.remove(self.cursor_col - 1);
                    self.cursor_col -= 1;
                }
            } else {
                // UTF-8 Aware delete
                if let Some(c) = line[..self.cursor_col].chars().next_back() {
                    let len = c.len_utf8();
                    line.remove(self.cursor_col - len);
                    self.cursor_col -= len;
                }
            }

            self.modified = true;
            self.invalidate_from(self.cursor_row);
            true
        } else if self.cursor_row > 0 {
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
            self.modified = true;
            self.invalidate_from(self.cursor_row);
            true
        } else {
            false
        }
    }

    fn delete_forwards(&mut self) -> bool {
        self.ensure_valid_cursor_col();
        let line = &mut self.lines[self.cursor_row];
        if self.cursor_col < line.len() {
            if let Some(_c) = line[self.cursor_col..].chars().next() {
                line.remove(self.cursor_col); // remove at cursor_col is fine, it's the start of the char
            }
            self.invalidate_from(self.cursor_row);
            true
        } else if self.cursor_row < self.lines.len() - 1 {
            let next_line = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next_line);
            self.invalidate_from(self.cursor_row);
            true
        } else {
            false
        }
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_col > 0 {
            let line = self.get_effective_line(self.cursor_row);
            let mut i = self.cursor_col;
            while i > 0 {
                i -= 1;
                if line.is_char_boundary(i) {
                    break;
                }
            }
            self.cursor_col = i;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.get_effective_line(self.cursor_row).len();
        }
    }

    fn move_cursor_right(&mut self) {
        let line = self.get_effective_line(self.cursor_row);
        if self.cursor_col < line.len() {
            let mut i = self.cursor_col + 1;
            while i <= line.len() {
                if line.is_char_boundary(i) {
                    break;
                }
                i += 1;
            }
            self.cursor_col = i;
        } else if self.cursor_row < self.effective_len().saturating_sub(1) {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_row > 0 {
            let current_visual = self.get_visual_x(self.cursor_row, self.cursor_col);
            self.cursor_row -= 1;
            self.cursor_col = self.get_byte_index_from_visual(self.cursor_row, current_visual);
        }
    }

    fn move_cursor_down(&mut self) {
        if self.cursor_row < self.effective_len().saturating_sub(1) {
            let current_visual = self.get_visual_x(self.cursor_row, self.cursor_col);
            self.cursor_row += 1;
            self.cursor_col = self.get_byte_index_from_visual(self.cursor_row, current_visual);
        }
    }

    pub fn move_line_up(&mut self) {
        if self.cursor_row > 0 {
            self.push_history();
            let line = self.lines.remove(self.cursor_row);
            self.lines.insert(self.cursor_row - 1, line);
            self.cursor_row -= 1;
            self.modified = true;
            self.invalidate_from(0);
        }
    }

    pub fn move_line_down(&mut self) {
        if self.cursor_row < self.lines.len().saturating_sub(1) {
            self.push_history();
            let line = self.lines.remove(self.cursor_row);
            self.lines.insert(self.cursor_row + 1, line);
            self.cursor_row += 1;
            self.modified = true;
            self.invalidate_from(0);
        }
    }

    pub fn ensure_cursor_centered(&mut self, area: Rect) {
        let height = area.height as usize;
        let target_scroll = self.cursor_row.saturating_sub(height / 2);
        let max_scroll = self.effective_len().saturating_sub(height);
        self.scroll_row = std::cmp::min(target_scroll, max_scroll);

        if self.wrap {
            self.scroll_col = 0;
            return;
        }

        let gutter = self.gutter_width();
        let width = area.width.saturating_sub(gutter as u16) as usize;
        let visual_cursor_x = self.get_visual_x(self.cursor_row, self.cursor_col);

        if visual_cursor_x < self.scroll_col {
            self.scroll_col = visual_cursor_x;
        } else if visual_cursor_x >= self.scroll_col + width {
            self.scroll_col = visual_cursor_x - width + 1;
        }
    }

    pub fn get_visual_row_at(&self, row: usize, width: usize) -> usize {
        let mut visual_row = 0;
        for i in 0..row {
            let line = self.get_effective_line(i);
            if self.wrap && width > 0 {
                let w = line.width();
                if w == 0 {
                    visual_row += 1;
                } else {
                    visual_row += (w.saturating_sub(1) / width) + 1;
                }
            } else {
                visual_row += 1;
            }
        }
        visual_row
    }

    pub fn get_cursor_visual_row(&self, width: usize) -> usize {
        let mut visual_row = self.get_visual_row_at(self.cursor_row, width);

        if self.wrap && width > 0 {
            let line = self.get_effective_line(self.cursor_row);
            let cursor_x = line[..self.cursor_col].width();
            visual_row += cursor_x / width;
        }

        visual_row
    }

    pub fn ensure_cursor_visible(&mut self, area: Rect) {
        let height = area.height as usize;
        let gutter = self.gutter_width();
        let scrollbar_w = if self.effective_len() > area.height as usize {
            1
        } else {
            0
        };
        let width = area
            .width
            .saturating_sub(gutter as u16 + scrollbar_w as u16) as usize;

        if self.wrap {
            let visual_row = self.get_cursor_visual_row(width);
            if visual_row < self.scroll_row {
                self.scroll_row = visual_row;
            } else if visual_row >= self.scroll_row + height {
                self.scroll_row = visual_row - height + 1;
            }
            self.scroll_col = 0;
            return;
        }

        // Smoother vertical scroll with safety for small areas
        let margin = (height / 4).min(3);
        if self.cursor_row < self.scroll_row + margin {
            self.scroll_row = self.cursor_row.saturating_sub(margin);
        } else if self.cursor_row >= self.scroll_row + height.saturating_sub(margin) {
            let target_scroll = (self.cursor_row + margin + 1).saturating_sub(height);
            let max_scroll = self.effective_len().saturating_sub(height);
            self.scroll_row = std::cmp::min(target_scroll, max_scroll);
        }

        let visual_cursor_x = self.get_visual_x(self.cursor_row, self.cursor_col);

        if visual_cursor_x < self.scroll_col {
            self.scroll_col = visual_cursor_x;
        } else if visual_cursor_x >= self.scroll_col + width {
            self.scroll_col = (visual_cursor_x + 1).saturating_sub(width);
        }
    }

    pub fn get_selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let start = self.selection_start?;
        let end = self.selection_end?;
        if start <= end {
            Some((start, end))
        } else {
            Some((end, start))
        }
    }

    pub fn maybe_start_selection(&mut self) {
        if self.selection_start.is_none() {
            self.selection_start = Some((self.cursor_row, self.cursor_col));
        }
    }

    pub fn update_selection_end(&mut self) {
        self.selection_end = Some((self.cursor_row, self.cursor_col));
    }

    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.is_selecting = false;
        self.is_dragging_selection = false;
    }

    pub fn is_inside_selection(&self, row: usize, byte_col: usize) -> bool {
        if let Some(((s_row, s_col), (e_row, e_col))) = self.get_selection_range() {
            if row > s_row && row < e_row {
                return true;
            }
            if row == s_row && row == e_row {
                return byte_col >= s_col && byte_col < e_col;
            }
            if row == s_row {
                return byte_col >= s_col;
            }
            if row == e_row {
                return byte_col < e_col;
            }
        }
        false
    }

    pub fn get_selected_text(&self) -> Option<String> {
        let ((s_row, s_col), (e_row, e_col)) = self.get_selection_range()?;

        let mut result = String::new();

        for row in s_row..=e_row {
            if row >= self.effective_len() {
                break;
            }
            let line = self.get_effective_line(row);

            let start = if row == s_row { s_col } else { 0 };
            let end = if row == e_row { e_col } else { line.len() };

            // Ensure bounds
            let safe_start = std::cmp::min(start, line.len());
            let safe_end = std::cmp::min(end, line.len());

            if safe_start < safe_end {
                result.push_str(&line[safe_start..safe_end]);
            }

            if row < e_row {
                result.push('\n');
            }
        }

        Some(result)
    }

    pub fn delete_selection(&mut self) {
        if let Some(((s_row, s_col), (e_row, e_col))) = self.get_selection_range() {
            if s_row == e_row {
                let line = &mut self.lines[s_row];
                if s_col < line.len() && e_col <= line.len() {
                    line.replace_range(s_col..e_col, "");
                }
            } else {
                // First line: keep up to s_col
                let start_part = if s_col < self.lines[s_row].len() {
                    self.lines[s_row][..s_col].to_string()
                } else {
                    self.lines[s_row].clone()
                };

                // Last line: keep from e_col
                let end_part = if e_col < self.lines[e_row].len() {
                    self.lines[e_row][e_col..].to_string()
                } else {
                    String::new()
                };

                // Remove intermediate lines
                self.lines.drain(s_row + 1..=e_row);

                // Merge start and end
                self.lines[s_row] = format!("{}{}", start_part, end_part);
            }

            self.cursor_row = s_row;
            self.cursor_col = s_col;
            self.selection_start = None;
            self.selection_end = None;
            self.is_selecting = false;
            self.modified = true;
            self.invalidate_from(0);
        }
    }

    pub fn move_selection_to(&mut self, target_row: usize, target_col: usize) {
        let text = if let Some(t) = self.get_selected_text() {
            t
        } else {
            return;
        };
        let ((s_row, s_col), (e_row, e_col)) = if let Some(range) = self.get_selection_range() {
            range
        } else {
            return;
        };

        self.push_history();

        // Calculate new position after deletion
        let mut new_row = target_row;
        let mut new_col = target_col;

        if target_row > e_row {
            // Target is below the selection, shifts up by (e_row - s_row)
            new_row -= e_row - s_row;
        } else if target_row == e_row && target_col >= e_col {
            // Target is on the same line as the end of selection, but after it
            new_row = s_row;
            new_col = s_col + (target_col - e_col);
        } else if target_row == s_row && target_col >= s_col {
            // Target is inside selection (on first line) - move to start of selection
            new_row = s_row;
            new_col = s_col;
        } else if target_row > s_row && target_row < e_row {
            // Target is inside selection (middle lines) - move to start of selection
            new_row = s_row;
            new_col = s_col;
        }

        self.delete_selection();
        self.cursor_row = new_row;
        self.cursor_col = new_col;
        self.insert_string(&text);
    }

    pub fn insert_string(&mut self, text: &str) {
        self.push_history();
        // Delete selection if active
        if self.selection_start.is_some() {
            self.delete_selection();
        }

        for (i, part) in text.split('\n').enumerate() {
            if i > 0 {
                self.insert_newline_raw();
            }
            for c in part.chars() {
                if c != '\r' {
                    self.insert_char(c);
                }
            }
        }
        self.invalidate_from(0);
    }

    pub fn select_all(&mut self) {
        self.selection_start = Some((0, 0));
        let last_row = self.lines.len().saturating_sub(1);
        let last_col = self.lines[last_row].len();
        self.selection_end = Some((last_row, last_col));
        self.cursor_row = last_row;
        self.cursor_col = last_col;
        self.is_selecting = false;
    }

    pub fn select_word_at(&mut self, row: usize, col: usize) {
        if row >= self.lines.len() {
            return;
        }
        let line = &self.lines[row];
        if col > line.len() {
            return;
        }

        let mut start = col;
        let mut end = col;

        // Find word start
        while start > 0 {
            if let Some(prev) = line[..start].chars().next_back() {
                if prev.is_alphanumeric() || prev == '_' {
                    start -= prev.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Find word end
        while end < line.len() {
            if let Some(next) = line[end..].chars().next() {
                if next.is_alphanumeric() || next == '_' {
                    end += next.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if start < end {
            self.selection_start = Some((row, start));
            self.selection_end = Some((row, end));
            self.cursor_row = row;
            self.cursor_col = end;
            self.is_selecting = false;
        }
    }

    pub fn select_line_at(&mut self, row: usize) {
        if row >= self.lines.len() {
            return;
        }
        let line_len = self.lines[row].len();
        self.selection_start = Some((row, 0));
        self.selection_end = Some((row, line_len));
        self.cursor_row = row;
        self.cursor_col = line_len;
        self.is_selecting = false;
    }

    pub fn delete_line(&mut self, row: usize) {
        if self.lines.len() > 1 {
            self.push_history();
            self.lines.remove(row);
            self.cursor_row = std::cmp::min(self.cursor_row, self.lines.len() - 1);
            self.cursor_col = std::cmp::min(self.cursor_col, self.lines[self.cursor_row].len());
            self.modified = true;
            self.invalidate_from(0);
        } else {
            self.push_history();
            self.lines[0].clear();
            self.cursor_col = 0;
            self.modified = true;
            self.invalidate_from(0);
        }
    }
}

impl Widget for &TextEditor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let gutter_w = self.gutter_width();
        let scrollbar_w = if self.effective_len() > area.height as usize {
            1
        } else {
            0
        };

        let content_area = Rect::new(
            area.x + gutter_w as u16,
            area.y,
            area.width
                .saturating_sub(gutter_w as u16 + scrollbar_w as u16),
            area.height,
        );

        let mut highlighted = {
            let mut first_invalid = self.first_invalid_line.borrow_mut();
            let mut cache = self.highlighted_cache.borrow_mut();

            if let Some(start_line) = *first_invalid {
                // Determine if we can do partial update or need full re-highlight
                // syntect's HighlightLines needs state from previous lines usually,
                // but for many languages we can restart at line boundaries if we don't care about multiline block comments as much,
                // or we just re-highlight from the first changed line to the end.

                let content_string = if !self.filter_query.is_empty() {
                    // Filtered view is harder to do incrementally, just do full for now
                    self.filtered_indices
                        .iter()
                        .map(|&i| self.lines[i].as_str())
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    self.lines.join("\n")
                };

                if !self.filter_query.is_empty() || start_line == 0 {
                    let h_lines = highlight_code(&content_string, &self.language);
                    *cache = h_lines
                        .into_iter()
                        .map(|line| {
                            let spans: Vec<Span<'static>> = line
                                .spans
                                .into_iter()
                                .map(|span| Span::styled(span.content.to_string(), span.style))
                                .collect();
                            Line::from(spans)
                        })
                        .collect();
                } else {
                    // INCREMENTAL: Only highlight from start_line to end
                    // Actually, syntect highlight_line maintains state. To do it right we need to store state per line.
                    // For now, let's just re-highlight everything if anything changed, BUT
                    // only if the cache size changed or we are forced.

                    // PERFORMANCE SHORTCUT: If we are just typing on one line, and it's not a filtered view,
                    // we can try to just re-highlight the entire file but it's still fast enough if we don't do it 60fps.
                    // The REAL lag usually comes from syntect being called too often.

                    let h_lines = highlight_code(&content_string, &self.language);
                    *cache = h_lines
                        .into_iter()
                        .map(|line| {
                            let spans: Vec<Span<'static>> = line
                                .spans
                                .into_iter()
                                .map(|span| Span::styled(span.content.to_string(), span.style))
                                .collect();
                            Line::from(spans)
                        })
                        .collect();
                }
                *first_invalid = None;
            }
            cache.clone()
        };

        // Ensure we have enough lines for the "extra line" and trailing empty lines
        while highlighted.len() < self.effective_len() {
            highlighted.push(Line::from(""));
        }

        let mut screen_lines: Vec<(usize, Line)> = Vec::new();
        let _cursor_screen_pos: Option<(u16, u16)> = None;

        for (line_idx, line) in highlighted.iter().enumerate() {
            let _real_line_idx = self.get_real_line_idx(line_idx);

            if self.wrap {
                let mut current_spans = Vec::new();
                let mut current_width = 0;
                let max_width = content_area.width as usize;

                for span in &line.spans {
                    let mut text = span.content.as_ref();
                    while !text.is_empty() {
                        let mut available = max_width.saturating_sub(current_width);
                        if available == 0 {
                            screen_lines.push((line_idx, Line::from(current_spans.clone())));
                            current_spans.clear();
                            current_width = 0;
                            available = max_width;
                        }

                        let mut break_idx = 0;
                        let mut break_width = 0;
                        for (idx, c) in text.char_indices() {
                            let cw = c.width().unwrap_or(0);
                            if break_width + cw > available {
                                break;
                            }
                            break_idx = idx + c.len_utf8();
                            break_width += cw;
                        }

                        if break_idx == 0 && !text.is_empty() {
                            // Force break if even a single char doesn't fit
                            let first_char = text.chars().next().unwrap();
                            break_idx = first_char.len_utf8();
                            break_width = first_char.width().unwrap_or(0);
                        }

                        let part = &text[..break_idx];
                        current_spans.push(Span::styled(part, span.style));

                        // Check if cursor is in this part
                        if line_idx == self.cursor_row {
                            let line_ref = self.get_effective_line(line_idx);
                            let _byte_offset = line_ref
                                .char_indices()
                                .filter(|(i, _)| *i < self.cursor_col)
                                .map(|(_, c)| c.len_utf8())
                                .sum::<usize>();
                            // Cursor detection in wrapped lines is hard.
                            // Simpler: if we just rendered the part containing self.cursor_col.
                            // We need to know the byte offset of 'part' within 'line'.
                        }

                        current_width += break_width;
                        text = &text[break_idx..];
                    }
                }
                screen_lines.push((line_idx, Line::from(current_spans)));
            } else {
                screen_lines.push((line_idx, line.clone()));
            }
        }

        // Second pass: Find cursor screen position and handle scrolling
        // For now, let's keep the existing logic if wrap is false.
        // If wrap is true, we need a smarter scroll_row (screen line index).

        if self.wrap {
            let mut current_screen_row = 0;
            #[allow(clippy::explicit_counter_loop)]
            for (line_idx, (src_idx, line)) in screen_lines.iter().enumerate() {
                if current_screen_row >= self.scroll_row
                    && current_screen_row < self.scroll_row + area.height as usize
                {
                    let i = current_screen_row - self.scroll_row;
                    let real_line_idx = self.get_real_line_idx(*src_idx);
                    let is_current_src = *src_idx == self.cursor_row;

                    // 1. Draw Background Highlight (Current Line)
                    let base_bg = self.style.bg.unwrap_or(Color::Reset);
                    let line_bg = if is_current_src {
                        Color::Rgb(20, 20, 25)
                    } else {
                        base_bg
                    };

                    let bg_area = Rect::new(area.x, area.y + i as u16, area.width, 1);
                    for x in bg_area.left()..bg_area.right() {
                        if let Some(cell) = buf.cell_mut((x, bg_area.top())) {
                            cell.set_bg(line_bg);
                        }
                    }

                    // 2. Render Gutter (only for the first screen line of a source line)
                    if self.show_line_numbers {
                        let is_first_wrap_line =
                            line_idx == 0 || screen_lines[line_idx - 1].0 != *src_idx;
                        if is_first_wrap_line {
                            let num = (real_line_idx + 1).to_string();
                            let gutter_style = if is_current_src {
                                Style::default()
                                    .fg(Color::Rgb(88, 166, 255))
                                    .add_modifier(Modifier::BOLD)
                                    .bg(line_bg)
                            } else {
                                Style::default().fg(Color::Rgb(110, 118, 129)).bg(line_bg)
                            };
                            let x = area.x + (gutter_w as u16).saturating_sub(num.len() as u16 + 2);
                            buf.set_string(x + 1, area.y + i as u16, &num, gutter_style);
                        }
                        let sep_style = Style::default().fg(Color::Rgb(48, 54, 61)).bg(line_bg);
                        buf.set_string(
                            area.x + gutter_w as u16 - 1,
                            area.y + i as u16,
                            "│",
                            sep_style,
                        );
                    }

                    // 3. Render Line Content
                    let mut current_x = content_area.x;
                    for span in &line.spans {
                        let mut span_style = self.style.patch(span.style);
                        if span_style.bg.is_none() || span_style.bg == Some(base_bg) {
                            span_style.bg = Some(line_bg);
                        }

                        buf.set_string(current_x, area.y + i as u16, &span.content, span_style);
                        current_x += span.content.width() as u16;
                    }

                    // 4. Apply Selection (Second pass over the rendered line)
                    if let Some(((s_row, s_col), (e_row, e_col))) = self.get_selection_range() {
                        if real_line_idx >= s_row && real_line_idx <= e_row {
                            // In wrap mode, we need to know the byte offset of each character on screen.
                            // This is hard without re-tracking.
                            // Let's use get_byte_index_from_visual.
                            for vx in 0..content_area.width {
                                let visual_x = vx as usize + self.scroll_col;
                                let byte_idx = self.get_byte_index_from_visual(*src_idx, visual_x);

                                let is_selected = if real_line_idx > s_row && real_line_idx < e_row
                                {
                                    true
                                } else if real_line_idx == s_row && real_line_idx == e_row {
                                    byte_idx >= s_col && byte_idx < e_col
                                } else if real_line_idx == s_row {
                                    byte_idx >= s_col
                                } else if real_line_idx == e_row {
                                    byte_idx < e_col
                                } else {
                                    false
                                };

                                if is_selected {
                                    let cx = content_area.x + vx;
                                    let cy = area.y + i as u16;
                                    if let Some(cell) = buf.cell_mut((cx, cy)) {
                                        cell.set_bg(Color::Rgb(40, 60, 100));
                                        cell.set_fg(Color::White);
                                    }
                                }
                            }
                        }
                    }
                }
                current_screen_row += 1;
            }

            // Cursor rendering for wrap mode
            let mut current_screen_row = 0;
            let mut cursor_found = false;

            // We need to find which screen line contains the cursor
            // This is slightly complex because we need to know the byte offset of each wrap segment.
            // Let's re-calculate or track it.

            for (line_idx, line) in highlighted.iter().enumerate() {
                if line_idx > self.cursor_row {
                    break;
                }

                let is_cursor_line = line_idx == self.cursor_row;
                let mut current_byte_offset = 0;
                let mut current_width = 0;
                let max_width = content_area.width as usize;

                for span in &line.spans {
                    let mut text = span.content.as_ref();
                    while !text.is_empty() {
                        let mut available = max_width.saturating_sub(current_width);
                        if available == 0 {
                            current_screen_row += 1;
                            current_width = 0;
                            available = max_width;
                        }

                        let mut break_idx = 0;
                        let mut break_width = 0;
                        for (idx, c) in text.char_indices() {
                            let cw = c.width().unwrap_or(0);
                            if break_width + cw > available {
                                break;
                            }
                            break_idx = idx + c.len_utf8();
                            break_width += cw;
                        }

                        if break_idx == 0 && !text.is_empty() {
                            let first_char = text.chars().next().unwrap();
                            break_idx = first_char.len_utf8();
                            break_width = first_char.width().unwrap_or(0);
                        }

                        if is_cursor_line && !cursor_found {
                            if self.cursor_col >= current_byte_offset
                                && self.cursor_col < current_byte_offset + break_idx
                            {
                                // Cursor is in THIS segment
                                let sub_col = self.cursor_col - current_byte_offset;
                                let visual_offset = text[..sub_col].width();
                                let cx = content_area
                                    .x
                                    .saturating_add((current_width + visual_offset) as u16);
                                let cy = (area.y as i32
                                    + (current_screen_row as i32 - self.scroll_row as i32))
                                    as u16;

                                if current_screen_row >= self.scroll_row
                                    && current_screen_row < self.scroll_row + area.height as usize
                                {
                                    if let Some(cell) = buf.cell_mut((cx, cy)) {
                                        if !cell.symbol().is_empty() && cell.symbol() != " " {
                                            cell.set_style(self.cursor_style);
                                        } else {
                                            cell.set_style(self.cursor_style);
                                            cell.set_symbol(" ");
                                        }
                                    }
                                }
                                cursor_found = true;
                            } else if self.cursor_col == current_byte_offset + break_idx
                                && (current_byte_offset + break_idx == line.width()
                                    || text.len() == break_idx)
                            {
                                // Special case: cursor at end of line/segment
                                // If it's the absolute end of the source line, or it's the end of a wrap segment but NOT the last segment?
                                // Actually if it's the end of a segment that IS the end of the line.
                                let _is_last_segment = text.len() == break_idx; // This span is done
                                                                                // Wait, we need to know if there are MORE spans.
                                                                                // Simplification: if cursor_col == line.len() and we are at the last segment of the last span.
                            }
                        }

                        current_byte_offset += break_idx;
                        current_width += break_width;
                        text = &text[break_idx..];
                    }
                }

                // Handle cursor at the absolute end of line
                if is_cursor_line
                    && !cursor_found
                    && self.cursor_col == self.get_effective_line(line_idx).len()
                {
                    let cx = content_area.x.saturating_add(current_width as u16);
                    let cy = (area.y as i32 + (current_screen_row as i32 - self.scroll_row as i32))
                        as u16;
                    if current_screen_row >= self.scroll_row
                        && current_screen_row < self.scroll_row + area.height as usize
                    {
                        if let Some(cell) = buf.cell_mut((cx, cy)) {
                            cell.set_style(self.cursor_style);
                            cell.set_symbol(" ");
                        }
                    }
                    cursor_found = true;
                }

                current_screen_row += 1;
            }

            // Render Scrollbars for wrap mode
            if scrollbar_w > 0 {
                let sb = Scrollbar::default()
                    .orientation(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"));
                // In wrap mode, scroll_row is screen line index.
                // We need the total screen lines for ScrollbarState.
                let total_screen_lines = screen_lines.len();
                let mut ss = ScrollbarState::new(total_screen_lines)
                    .position(self.scroll_row)
                    .viewport_content_length(area.height as usize);
                StatefulWidget::render(sb, area, buf, &mut ss);
            }

            return;
        }

        // Original non-wrap rendering logic
        for (i, line) in highlighted.iter().skip(self.scroll_row).enumerate() {
            if i >= area.height as usize {
                break;
            }

            let line_idx = self.scroll_row + i;
            let real_line_idx = self.get_real_line_idx(line_idx);
            let is_current = line_idx == self.cursor_row;

            let base_bg = self.style.bg.unwrap_or(Color::Reset);
            let line_bg = if is_current {
                Color::Rgb(20, 20, 25)
            } else {
                base_bg
            };

            if is_current || base_bg != Color::Reset {
                let bg_area = Rect::new(area.x, area.y + i as u16, area.width, 1);
                for x in bg_area.left()..bg_area.right() {
                    if let Some(cell) = buf.cell_mut((x, bg_area.top())) {
                        cell.set_bg(line_bg);
                    }
                }
            }

            // Render Gutter (Line Numbers)
            if self.show_line_numbers {
                let num = (real_line_idx + 1).to_string();
                let gutter_style = if is_current {
                    Style::default()
                        .fg(Color::Rgb(88, 166, 255))
                        .add_modifier(Modifier::BOLD)
                        .bg(line_bg)
                } else {
                    Style::default().fg(Color::Rgb(110, 118, 129)).bg(line_bg)
                };
                let x = area.x + (gutter_w as u16).saturating_sub(num.len() as u16 + 2);
                buf.set_string(x + 1, area.y + i as u16, &num, gutter_style);

                // Vertical separator
                let sep_style = Style::default().fg(Color::Rgb(48, 54, 61)).bg(line_bg);
                buf.set_string(
                    area.x + gutter_w as u16 - 1,
                    area.y + i as u16,
                    "│",
                    sep_style,
                );
            }

            // Render Content
            let mut current_visual_x = 0;
            for span in &line.spans {
                let text = span.content.as_ref();
                let span_width = text.width();

                // If span ends before the visible area, skip it
                if current_visual_x + span_width <= self.scroll_col {
                    current_visual_x += span_width;
                    continue;
                }

                // If span starts after the visible area, we are done
                if current_visual_x >= self.scroll_col + content_area.width as usize {
                    break;
                }

                let mut draw_text = text;
                let draw_x = if current_visual_x < self.scroll_col {
                    // Partial overlap on left
                    let skip_width = self.scroll_col - current_visual_x;
                    let mut char_indices = text.char_indices();
                    let mut w = 0;
                    let mut start_byte = 0;
                    while w < skip_width {
                        if let Some((idx, c)) = char_indices.next() {
                            w += c.width().unwrap_or(0);
                            start_byte = idx + c.len_utf8();
                        } else {
                            break;
                        }
                    }
                    draw_text = &text[start_byte..];
                    content_area.x + (current_visual_x + w).saturating_sub(self.scroll_col) as u16
                } else {
                    content_area.x + (current_visual_x - self.scroll_col) as u16
                };

                // Combine span style with base style to ensure visibility (fix black-on-black)
                let mut combined_style = self.style.patch(span.style);
                if combined_style.bg.is_none() || combined_style.bg == Some(base_bg) {
                    combined_style.bg = Some(line_bg);
                }
                buf.set_string(draw_x, area.y + i as u16, draw_text, combined_style);
                current_visual_x += span_width;
            }

            // Apply Selection highlighting
            if let Some(((s_row, s_col), (e_row, e_col))) = self.get_selection_range() {
                if real_line_idx >= s_row && real_line_idx <= e_row {
                    for visual_x in self.scroll_col..(self.scroll_col + content_area.width as usize)
                    {
                        let byte_idx = self.get_byte_index_from_visual(line_idx, visual_x);

                        let is_selected = if real_line_idx > s_row && real_line_idx < e_row {
                            true
                        } else if real_line_idx == s_row && real_line_idx == e_row {
                            byte_idx >= s_col && byte_idx < e_col
                        } else if real_line_idx == s_row {
                            byte_idx >= s_col
                        } else if real_line_idx == e_row {
                            byte_idx < e_col
                        } else {
                            false
                        };

                        if is_selected {
                            let cx = content_area.x + (visual_x - self.scroll_col) as u16;
                            let cy = area.y + i as u16;
                            if let Some(cell) = buf.cell_mut((cx, cy)) {
                                cell.set_bg(Color::Rgb(40, 60, 100)); // Selection Blue
                                cell.set_fg(Color::White);
                            }
                        }
                    }
                }
            }
        }

        // Render Scrollbars
        if scrollbar_w > 0 {
            let sb = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"));
            let mut ss = ScrollbarState::new(self.effective_len())
                .position(self.scroll_row)
                .viewport_content_length(area.height as usize);
            StatefulWidget::render(sb, area, buf, &mut ss);
        }

        // Render Cursor
        let cursor_visual_x = self.get_visual_x(self.cursor_row, self.cursor_col);
        let cursor_screen_row = self.cursor_row as i16 - self.scroll_row as i16;
        let cursor_screen_col = cursor_visual_x as i16 - self.scroll_col as i16;

        if cursor_screen_row >= 0
            && cursor_screen_row < area.height as i16
            && cursor_screen_col >= 0
            && cursor_screen_col < content_area.width as i16
        {
            let cx = content_area.x + cursor_screen_col as u16;
            let cy = area.y + cursor_screen_row as u16;

            if let Some(cell) = buf.cell_mut((cx, cy)) {
                if !cell.symbol().is_empty() && cell.symbol() != " " {
                    // Inverse video for character under cursor
                    cell.set_style(self.cursor_style);
                } else {
                    // Block cursor for empty space
                    cell.set_style(self.cursor_style);
                    cell.set_symbol(" ");
                }
            }
        }
    }
}
