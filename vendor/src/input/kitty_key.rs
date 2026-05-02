use super::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub fn parse_kitty_keyboard(parts: &[&str]) -> Option<Event> {
    if parts.is_empty() {
        return None;
    }

    let code_val: u32 = parts[0].parse().ok()?;
    let mut modifiers = KeyModifiers::empty();
    let mut kind = KeyEventKind::Press; // Default

    // Second param is modifiers (1-based bitmask)
    if parts.len() > 1 {
        if let Ok(mod_val) = parts[1].parse::<u8>() {
            let m = mod_val.saturating_sub(1);
            if (m & 1) != 0 {
                modifiers.insert(KeyModifiers::SHIFT);
            }
            if (m & 2) != 0 {
                modifiers.insert(KeyModifiers::ALT);
            }
            if (m & 4) != 0 {
                modifiers.insert(KeyModifiers::CONTROL);
            }
            if (m & 8) != 0 {
                modifiers.insert(KeyModifiers::SUPER);
            } // Super
              // Bit 4 (16) -> Hyper? Bit 5 (32) -> Meta?
        }
    }

    // Third param is event type (1=Press, 2=Repeat, 3=Release)
    if parts.len() > 2 {
        if let Ok(type_val) = parts[2].parse::<u8>() {
            match type_val {
                1 => kind = KeyEventKind::Press,
                2 => kind = KeyEventKind::Repeat,
                3 => kind = KeyEventKind::Release,
                _ => {}
            }
        }
    }

    // Map Kitty Functional Keys (57344+)
    // 57344 = Escape? No, Escape is 27.
    // Kitty maps F1-F35 to PUA.

    let key_code = if (57344..=63743).contains(&code_val) {
        // PUA mapping
        map_kitty_pua(code_val)
    } else {
        // Standard ASCII / Unicode
        match code_val {
            27 => KeyCode::Esc,
            13 => KeyCode::Enter,
            9 => KeyCode::Tab,
            127 => KeyCode::Backspace,
            _ => {
                if let Some(c) = std::char::from_u32(code_val) {
                    KeyCode::Char(c)
                } else {
                    return None;
                }
            }
        }
    };

    Some(Event::Key(KeyEvent {
        code: key_code,
        modifiers,
        kind,
    }))
}

fn map_kitty_pua(code: u32) -> KeyCode {
    // Basic mapping, incomplete
    match code {
        57364 => KeyCode::F(1),
        57365 => KeyCode::F(2),
        57366 => KeyCode::F(3),
        57367 => KeyCode::F(4),
        57368 => KeyCode::F(5),
        57369 => KeyCode::F(6),
        57370 => KeyCode::F(7),
        57371 => KeyCode::F(8),
        57372 => KeyCode::F(9),
        57373 => KeyCode::F(10),
        57374 => KeyCode::F(11),
        57375 => KeyCode::F(12),
        // Cursor Keys
        57358 => KeyCode::Up,
        57359 => KeyCode::Down,
        57360 => KeyCode::Left,
        57361 => KeyCode::Right,
        57362 => KeyCode::PageUp,
        57363 => KeyCode::PageDown,
        57344 => KeyCode::Esc, // Is this right?
        57345 => KeyCode::Insert,
        57346 => KeyCode::Delete,
        57347 => KeyCode::Home,
        57348 => KeyCode::End,
        // ...
        _ => KeyCode::Null,
    }
}
