use crate::input::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

#[derive(PartialEq, Eq, Debug)]
enum ParserState {
    Normal,
    PasteData, // Buffering untrusted text until \x1b[201~
}

/// A simple state machine for parsing ANSI sequences.
/// Focused on performance and SGR 1006 correctness.
pub struct Parser {
    buffer: Vec<u8>,
    state: ParserState,
    paste_buffer: Vec<u8>,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(32),
            state: ParserState::Normal,
            paste_buffer: Vec::with_capacity(1024),
        }
    }

    /// Feeds a byte into the parser. Returns Option<Event> if a complete event is formed.
    pub fn advance(&mut self, byte: u8) -> Option<Event> {
        // Safety: Prevent buffer bloat
        if self.buffer.len() > 2048 {
            self.buffer.clear();
        }

        match self.state {
            ParserState::PasteData => {
                self.paste_buffer.push(byte);
                // Check for end marker \x1b[201~ (6 bytes)
                if self.paste_buffer.len() >= 6 {
                    if let Some(pos) = self
                        .paste_buffer
                        .windows(6)
                        .rposition(|w| w == b"\x1b[201~")
                    {
                        // Found end marker. Extract content up to pos.
                        let content =
                            String::from_utf8_lossy(&self.paste_buffer[..pos]).to_string();
                        self.paste_buffer.clear();
                        self.state = ParserState::Normal;
                        return Some(Event::Paste(content));
                    }
                }
                // Check limit
                if self.paste_buffer.len() > 1024 * 1024 {
                    self.paste_buffer.clear();
                    self.state = ParserState::Normal;
                }
                return None;
            }
            ParserState::Normal => {
                // Quick path for ASCII only if buffer empty
                if self.buffer.is_empty() && byte >= 0x20 && byte != 0x7F {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::Char(byte as char),
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Press,
                    }));
                }
                self.buffer.push(byte);

                // Try parse
                if let Some(event) = self.try_parse() {
                    self.buffer.clear();
                    return Some(event);
                }
            }
        }
        None
    }

    pub fn check_timeout(&mut self) -> Option<Event> {
        if self.state == ParserState::Normal && !self.buffer.is_empty() && self.buffer[0] == 0x1B {
            self.buffer.clear();
            return Some(Event::Key(KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
            }));
        }
        None
    }

    fn try_parse(&mut self) -> Option<Event> {
        if self.buffer[0] != 0x1B {
            // Not an escape sequence
            match self.buffer[0] {
                b'\r' | b'\n' => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::Enter,
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Press,
                    }))
                }
                b'\t' => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::Tab,
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Press,
                    }))
                }
                0x08 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::Backspace,
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                    }))
                }
                0x7F => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::Backspace,
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Press,
                    }))
                }
                // Handle standard Ctrl+A (1) through Ctrl+Z (26)
                c if (1..=26).contains(&c) && c != 8 => {
                    let char_code = (c + 96) as char; // 1->'a', 26->'z'
                    let modifiers = KeyModifiers::CONTROL;
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::Char(char_code),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }));
                }
                // Handle Ctrl+Space (NUL / 0)
                0 => {
                    let modifiers = KeyModifiers::CONTROL;
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::Char(' '),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }));
                }
                // Handle Ctrl+. (Unit Separator / 31)
                31 => {
                    let modifiers = KeyModifiers::CONTROL;
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::Char('.'),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }));
                }
                // Handle remaining control codes or normal chars
                c => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::Char(c as char),
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Press,
                    }))
                }
            }
        }

        // Focus In: \x1b[I
        if self.buffer == b"\x1b[I" {
            return Some(Event::FocusGained);
        }
        // Focus Out: \x1b[O
        if self.buffer == b"\x1b[O" {
            return Some(Event::FocusLost);
        }

        // SS3 (F1-F4): \x1bO P, Q, R, S
        if self.buffer.len() == 3 && self.buffer[0] == 0x1B && self.buffer[1] == b'O' {
            let key = match self.buffer[2] {
                b'P' => KeyCode::F(1),
                b'Q' => KeyCode::F(2),
                b'R' => KeyCode::F(3),
                b'S' => KeyCode::F(4),
                _ => return None,
            };
            return Some(Event::Key(KeyEvent {
                code: key,
                modifiers: KeyModifiers::empty(),
                kind: KeyEventKind::Press,
            }));
        }

        // Start Paste: \x1b[200~
        if self.buffer == b"\x1b[200~" {
            self.state = ParserState::PasteData;
            self.paste_buffer.clear();
            return None; // Consumed, switch mode
        }

        // SGR Mouse: \x1b[<b;x;yM or m
        if self.buffer.len() > 3 && self.buffer[1] == b'[' && self.buffer[2] == b'<' {
            if let Some(&last) = self.buffer.last() {
                if last == b'M' || last == b'm' {
                    if let Some(evt) = self.parse_sgr() {
                        return Some(evt);
                    } else {
                        return Some(Event::Unsupported(self.buffer.clone()));
                    }
                }
            }
        }

        // Normal Mouse: \x1b[Mbxy
        if self.buffer.len() == 6 && self.buffer[1] == b'[' && self.buffer[2] == b'M' {
            let b = self.buffer[3].saturating_sub(32);
            let x = self.buffer[4].saturating_sub(32).saturating_sub(1) as u16;
            let y = self.buffer[5].saturating_sub(32).saturating_sub(1) as u16;

            let mut modifiers = KeyModifiers::empty();
            if (b & 4) != 0 {
                modifiers.insert(KeyModifiers::SHIFT);
            }
            if (b & 8) != 0 {
                modifiers.insert(KeyModifiers::ALT);
            }
            if (b & 16) != 0 {
                modifiers.insert(KeyModifiers::CONTROL);
            }

            let is_motion = (b & 32) != 0;
            let is_extra = (b & 64) != 0;

            if is_extra {
                let kind = match b & 0b11 {
                    0 => MouseEventKind::ScrollUp,
                    1 => MouseEventKind::ScrollDown,
                    _ => return Some(Event::Unsupported(self.buffer.clone())),
                };
                return Some(Event::Mouse(MouseEvent {
                    kind,
                    column: x,
                    row: y,
                    modifiers,
                }));
            }

            let button = match b & 0b11 {
                0 => MouseButton::Left,
                1 => MouseButton::Middle,
                2 => MouseButton::Right,
                3 => MouseButton::Left, // Release fallback
                _ => MouseButton::Other(b),
            };

            let kind = if is_motion {
                MouseEventKind::Drag(button)
            } else {
                MouseEventKind::Down(button)
            };

            return Some(Event::Mouse(MouseEvent {
                kind,
                column: x,
                row: y,
                modifiers,
            }));
        }

        // Kitty Keyboard: \x1b[code;modifiersu
        if self.buffer.len() > 2 && self.buffer[1] == b'[' {
            if let Some(&last) = self.buffer.last() {
                if last == b'u' {
                    let s_res = std::str::from_utf8(&self.buffer[2..self.buffer.len() - 1]);
                    if let Ok(s) = s_res {
                        let parts: Vec<&str> = s.split(';').collect();
                        if let Some(evt) = crate::input::kitty_key::parse_kitty_keyboard(&parts) {
                            return Some(evt);
                        }
                    }
                    return Some(Event::Unsupported(self.buffer.clone()));
                }
            }
        }

        // Generic CSI / ANSI Keys
        if self.buffer.len() > 2 && self.buffer[1] == b'[' {
            if let Some(&last) = self.buffer.last() {
                if (0x40..=0x7E).contains(&last) {
                    if let Some(evt) = self.parse_csi_normal() {
                        return Some(evt);
                    }
                    return Some(Event::Unsupported(self.buffer.clone()));
                }
            }
        }

        // Alt+Key Fallback (Esc + Char)
        if self.buffer.len() == 2 {
            let second = self.buffer[1];

            // Double Esc -> Single Esc event
            if second == 0x1B {
                return Some(Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: KeyModifiers::empty(),
                    kind: KeyEventKind::Press,
                }));
            }

            if second == 0x7F {
                return Some(Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    modifiers: KeyModifiers::ALT,
                    kind: KeyEventKind::Press,
                }));
            }
            if second != b'[' && second != b'O' && second != b'2' {
                let mut modifiers = KeyModifiers::ALT;
                let key = if (1..=26).contains(&second) {
                    modifiers.insert(KeyModifiers::CONTROL);
                    KeyCode::Char((second + 96) as char)
                } else {
                    KeyCode::Char(second as char)
                };

                // Safety: never emit Esc as a Char
                if let KeyCode::Char(c) = key {
                    if c == '\x1b' {
                        return None;
                    }
                }

                return Some(Event::Key(KeyEvent {
                    code: key,
                    modifiers,
                    kind: KeyEventKind::Press,
                }));
            }
        }

        None
    }

    fn parse_csi_normal(&self) -> Option<Event> {
        let last = *self.buffer.last()?;
        let content = std::str::from_utf8(&self.buffer[2..self.buffer.len() - 1]).ok()?;

        let parts: Vec<&str> = content.split(';').collect();
        let mut modifiers = KeyModifiers::empty();

        if last == b'~' {
            let code = parts.first()?.parse::<u16>().ok()?;
            match code {
                2 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::Insert,
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                3 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::Delete,
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                5 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::PageUp,
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                6 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::PageDown,
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                1 | 7 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::Home,
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                4 | 8 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::End,
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                11 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::F(1),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                12 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::F(2),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                13 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::F(3),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                14 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::F(4),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                15 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::F(5),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                17 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::F(6),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                18 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::F(7),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                19 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::F(8),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                20 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::F(9),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                21 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::F(10),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                23 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::F(11),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                24 => {
                    return Some(Event::Key(KeyEvent {
                        code: KeyCode::F(12),
                        modifiers,
                        kind: KeyEventKind::Press,
                    }))
                }
                27 => {
                    // modifyOtherKeys format: 27;modifier;char~
                    if parts.len() > 2 {
                        let mod_val = parts[1].parse::<u8>().ok()?.saturating_sub(1);
                        let char_code = parts[2].parse::<u32>().ok()?;
                        let mut m = KeyModifiers::empty();
                        if (mod_val & 1) != 0 {
                            m.insert(KeyModifiers::SHIFT);
                        }
                        if (mod_val & 2) != 0 {
                            m.insert(KeyModifiers::ALT);
                        }
                        if (mod_val & 4) != 0 {
                            m.insert(KeyModifiers::CONTROL);
                        }

                        if let Some(c) = std::char::from_u32(char_code) {
                            return Some(Event::Key(KeyEvent {
                                code: KeyCode::Char(c),
                                modifiers: m,
                                kind: KeyEventKind::Press,
                            }));
                        }
                    }
                }
                _ => {}
            };
        }

        let key = match last {
            b'A' => KeyCode::Up,
            b'B' => KeyCode::Down,
            b'C' => KeyCode::Right,
            b'D' => KeyCode::Left,
            b'H' => KeyCode::Home,
            b'F' => KeyCode::End,
            _ => return None,
        };

        // Parse modifiers (e.g. 1;5A or 5A)
        let mod_code_str = if parts.len() > 1 {
            Some(parts[1])
        } else if !parts[0].is_empty() && parts[0] != "1" {
            // Handle sequences like \x1b[5D
            Some(parts[0])
        } else {
            None
        };

        if let Some(s) = mod_code_str {
            if let Ok(mod_code) = s.parse::<u8>() {
                let m = mod_code.saturating_sub(1);
                if (m & 1) != 0 {
                    modifiers.insert(KeyModifiers::SHIFT);
                }
                if (m & 2) != 0 {
                    modifiers.insert(KeyModifiers::ALT);
                }
                if (m & 4) != 0 {
                    modifiers.insert(KeyModifiers::CONTROL);
                }
            }
        }

        Some(Event::Key(KeyEvent {
            code: key,
            modifiers,
            kind: KeyEventKind::Press,
        }))
    }

    fn parse_sgr(&self) -> Option<Event> {
        let s = std::str::from_utf8(&self.buffer[3..self.buffer.len() - 1]).ok()?;
        let parts: Vec<&str> = s.split(';').collect();
        if parts.len() != 3 {
            return None;
        }

        let b: u32 = parts[0].parse().ok()?;
        let x: u16 = parts[1].parse::<u16>().ok()?.saturating_sub(1);
        let y: u16 = parts[2].parse::<u16>().ok()?.saturating_sub(1);

        let last_char = *self.buffer.last()?;
        let is_release = last_char == b'm';

        let mut modifiers = KeyModifiers::empty();
        if (b & 4) != 0 {
            modifiers.insert(KeyModifiers::SHIFT);
        }
        if (b & 8) != 0 {
            modifiers.insert(KeyModifiers::ALT);
        }
        if (b & 16) != 0 {
            modifiers.insert(KeyModifiers::CONTROL);
        }

        let is_motion = (b & 32) != 0;
        let is_extra = (b & 64) != 0;

        if is_extra {
            let kind = match b & 0b11 {
                0 => MouseEventKind::ScrollUp,
                1 => MouseEventKind::ScrollDown,
                2 => MouseEventKind::ScrollLeft,
                3 => MouseEventKind::ScrollRight,
                _ => return None,
            };
            return Some(Event::Mouse(MouseEvent {
                kind,
                column: x,
                row: y,
                modifiers,
            }));
        }

        let button = match b & 0b1100_1011 {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            3 => MouseButton::Left, // Fallback for release
            8 | 128 => MouseButton::Back,
            9 | 129 => MouseButton::Forward,
            _ => MouseButton::Other(b as u8),
        };

        let kind = if is_release {
            MouseEventKind::Up(button)
        } else if is_motion {
            if (b & 3) == 3 {
                MouseEventKind::Moved
            } else {
                MouseEventKind::Drag(button)
            }
        } else {
            MouseEventKind::Down(button)
        };

        Some(Event::Mouse(MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sgr_back_button() {
        let mut parser = Parser::new();
        // SGR 1006 Sequence: \x1b[<b;x;yM
        // Back Button is often '8' or '136'
        let seq = b"\x1b[<8;10;20M";

        for byte in seq {
            if let Some(event) = parser.advance(*byte) {
                if let Event::Mouse(me) = event {
                    assert_eq!(me.kind, MouseEventKind::Down(MouseButton::Back));
                    assert_eq!(me.row, 19);
                    assert_eq!(me.column, 9);
                    return;
                }
            }
        }
        panic!("Did not parse SGR Back Button event");
    }

    #[test]
    fn test_sgr_shift_left_click() {
        let mut parser = Parser::new();
        // MB1 (0) + Shift (4) = 4.
        let seq = b"\x1b[<4;10;20M";
        let mut found = false;
        for &byte in seq {
            if let Some(Event::Mouse(me)) = parser.advance(byte) {
                assert_eq!(me.kind, MouseEventKind::Down(MouseButton::Left));
                assert!(me.modifiers.contains(KeyModifiers::SHIFT));
                assert_eq!(me.column, 9); // 10 - 1
                assert_eq!(me.row, 19); // 20 - 1
                found = true;
            }
        }
        assert!(found, "Did not parse SGR Shift+Left Click");
    }

    #[test]
    fn test_sgr_left_drag() {
        let mut parser = Parser::new();
        // MB1 (0) + Drag (32) = 32.
        let seq = b"\x1b[<32;15;25M";
        let mut found = false;
        for &byte in seq {
            if let Some(Event::Mouse(me)) = parser.advance(byte) {
                assert_eq!(me.kind, MouseEventKind::Drag(MouseButton::Left));
                assert_eq!(me.column, 14);
                assert_eq!(me.row, 24);
                found = true;
            }
        }
        assert!(found, "Did not parse SGR Left Drag");
    }

    #[test]
    fn test_sgr_mouse_moved() {
        let mut parser = Parser::new();
        // None (3) + Motion (32) = 35.
        let seq = b"\x1b[<35;15;25M";
        let mut found = false;
        for &byte in seq {
            if let Some(Event::Mouse(me)) = parser.advance(byte) {
                assert_eq!(me.kind, MouseEventKind::Moved);
                found = true;
            }
        }
        assert!(found, "Did not parse SGR Mouse Moved");
    }

    #[test]
    fn test_sgr_forward_button() {
        let mut parser = Parser::new();
        // Forward Button is often '9'
        let seq = b"\x1b[<9;5;5M";

        for byte in seq {
            if let Some(event) = parser.advance(*byte) {
                if let Event::Mouse(me) = event {
                    assert_eq!(me.kind, MouseEventKind::Down(MouseButton::Forward));
                    return;
                }
            }
        }
        panic!("Did not parse SGR Forward Button event");
    }

    #[test]
    fn test_kitty_keyboard() {
        let mut parser = Parser::new();

        // 1. Shift+A (\x1b[97;2u)
        let seq = b"\x1b[97;2u";
        let mut found = false;
        for &byte in seq {
            if let Some(Event::Key(k)) = parser.advance(byte) {
                if let KeyCode::Char('a') = k.code {
                    assert!(k.modifiers.contains(KeyModifiers::SHIFT));
                    found = true;
                }
            }
        }
        assert!(found, "Failed to parse Kitty Shift+A");
    }
}
