use crate::input::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

/// A reusable Text Input widget.
#[derive(Clone, Debug)]
pub struct TextInput {
    pub value: String,
    pub cursor_position: usize,
    pub style: Style,
    pub cursor_style: Style,
    pub placeholder: String,
    pub placeholder_style: Style,
}

impl Default for TextInput {
    fn default() -> Self {
        Self {
            value: String::new(),
            cursor_position: 0,
            style: Style::default().fg(Color::White),
            cursor_style: Style::default().bg(Color::White).fg(Color::Black),
            placeholder: String::new(),
            placeholder_style: Style::default().fg(Color::DarkGray),
        }
    }
}

impl TextInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self.cursor_position = self.value.len();
        self
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn set_value(&mut self, value: String) {
        self.value = value;
        self.cursor_position = self.value.len();
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_position = 0;
    }

    /// Handles an input event. Returns true if the input was modified.
    pub fn handle_event(&mut self, event: &Event) -> bool {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return false;
            }

            let has_control = key.modifiers.contains(KeyModifiers::CONTROL);
            let has_alt = key.modifiers.contains(KeyModifiers::ALT);

            match key.code {
                KeyCode::Char(c) if !has_control && !has_alt => {
                    if c == '\x1b' {
                        return false;
                    }
                    if self.cursor_position >= self.value.len() {
                        self.value.push(c);
                    } else {
                        let mut new_val = String::with_capacity(self.value.len() + 1);
                        for (i, ch) in self.value.chars().enumerate() {
                            if i == self.cursor_position {
                                new_val.push(c);
                            }
                            new_val.push(ch);
                        }
                        self.value = new_val;
                    }
                    self.cursor_position += 1;
                    return true;
                }
                // Ctrl+u: Clear to start
                KeyCode::Char('u') if has_control => {
                    if self.cursor_position > 0 {
                        self.value = self.value.chars().skip(self.cursor_position).collect();
                        self.cursor_position = 0;
                        return true;
                    }
                }
                // Ctrl+k: Clear to end
                KeyCode::Char('k') if has_control => {
                    if self.cursor_position < self.value.len() {
                        self.value = self.value.chars().take(self.cursor_position).collect();
                        return true;
                    }
                }
                // Ctrl+w / Ctrl+Backspace / Alt+Backspace: Delete word backwards
                KeyCode::Char('w') if has_control => {
                    return self.delete_word_backwards();
                }
                KeyCode::Backspace if has_control || has_alt => {
                    return self.delete_word_backwards();
                }
                // Ctrl+Delete: Delete word forwards
                KeyCode::Delete if has_control || has_alt => {
                    return self.delete_word_forwards();
                }
                // Ctrl+a: Start of line
                KeyCode::Char('a') if has_control => {
                    self.cursor_position = 0;
                    return true;
                }
                // Ctrl+e: End of line (Note: might be shadowed by global shortcuts)
                KeyCode::Char('e') if has_control => {
                    self.cursor_position = self.value.len();
                    return true;
                }
                // Ctrl+f: Move right
                KeyCode::Char('f') if has_control => {
                    if self.cursor_position < self.value.len() {
                        self.cursor_position += 1;
                        return true;
                    }
                }
                // Ctrl+b: Move left
                KeyCode::Char('b') if has_control => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                        return true;
                    }
                }
                KeyCode::Backspace => {
                    if self.cursor_position > 0 {
                        let mut new_val = String::with_capacity(self.value.len());
                        for (i, ch) in self.value.chars().enumerate() {
                            if i != self.cursor_position - 1 {
                                new_val.push(ch);
                            }
                        }
                        self.value = new_val;
                        self.cursor_position -= 1;
                        return true;
                    }
                }
                KeyCode::Delete => {
                    if self.cursor_position < self.value.len() {
                        let mut new_val = String::with_capacity(self.value.len());
                        for (i, ch) in self.value.chars().enumerate() {
                            if i != self.cursor_position {
                                new_val.push(ch);
                            }
                        }
                        self.value = new_val;
                        return true;
                    }
                }
                KeyCode::Left => {
                    if self.cursor_position > 0 {
                        self.cursor_position -= 1;
                        return true;
                    }
                }
                KeyCode::Right => {
                    if self.cursor_position < self.value.len() {
                        self.cursor_position += 1;
                        return true;
                    }
                }
                KeyCode::Home => {
                    self.cursor_position = 0;
                    return true;
                }
                KeyCode::End => {
                    self.cursor_position = self.value.len();
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    fn delete_word_backwards(&mut self) -> bool {
        if self.cursor_position == 0 {
            return false;
        }
        let mut i = self.cursor_position;

        // Skip trailing whitespace
        while i > 0 {
            let prev = self.value[..i].chars().next_back().unwrap();
            if prev.is_whitespace() {
                i -= prev.len_utf8();
            } else {
                break;
            }
        }
        // Skip the word
        while i > 0 {
            let prev = self.value[..i].chars().next_back().unwrap();
            if !prev.is_whitespace() {
                i -= prev.len_utf8();
            } else {
                break;
            }
        }

        let tail = self.value.split_off(self.cursor_position);
        self.value.truncate(i);
        self.value.push_str(&tail);
        self.cursor_position = i;
        true
    }

    fn delete_word_forwards(&mut self) -> bool {
        if self.cursor_position >= self.value.len() {
            return false;
        }
        let mut i = self.cursor_position;

        // Skip trailing whitespace
        while i < self.value.len() {
            let next = self.value[i..].chars().next().unwrap();
            if next.is_whitespace() {
                i += next.len_utf8();
            } else {
                break;
            }
        }
        // Skip the word
        while i < self.value.len() {
            let next = self.value[i..].chars().next().unwrap();
            if !next.is_whitespace() {
                i += next.len_utf8();
            } else {
                break;
            }
        }

        let tail = self.value.split_off(i);
        self.value.truncate(self.cursor_position);
        self.value.push_str(&tail);
        true
    }
}

impl Widget for &TextInput {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let display_text = if self.value.is_empty() {
            &self.placeholder
        } else {
            &self.value
        };

        let style = if self.value.is_empty() {
            self.placeholder_style
        } else {
            self.style
        };

        buf.set_string(area.x, area.y, display_text, style);

        // Draw Cursor
        // Only if focused/active? Assuming this widget is only rendered when active or we rely on caller.
        // We'll draw cursor if it's within bounds.
        // If text exceeds width, we should scroll. Implementing basic scrolling:

        let cursor_x = area.x + self.cursor_position as u16;

        // Simple horizontal scrolling logic (viewport follows cursor)
        // This is complex for a stateless render if we don't store "scroll_offset".
        // For simplicity, let's assume we render from the start, or we need to add `scroll_offset` to state.

        // To keep it simple for now: No scrolling, just clamp cursor.
        if cursor_x < area.x + area.width {
            if let Some(cell) = buf.cell_mut((cursor_x, area.y)) {
                cell.set_style(self.cursor_style);
                if self.cursor_position < self.value.len() {
                    // If cursor is over a character, ensure char is visible
                    let c = self.value.chars().nth(self.cursor_position).unwrap_or(' ');
                    cell.set_symbol(&c.to_string());
                } else {
                    cell.set_symbol(" ");
                }
            }
        }
    }
}
