use std::borrow::Cow;

use crate::input::event as rt;

pub use crate::contracts::{
    InputEvent as Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode,
    ModifierKeyCode, MouseButton, MouseEvent, MouseEventKind, UiEvent, UiResize,
};

pub fn from_runtime_event(event: rt::Event) -> Event {
    match event {
        rt::Event::Key(key) => Event::Key(from_runtime_key_event(key)),
        rt::Event::Mouse(me) => Event::Mouse(from_runtime_mouse_event(me)),
        rt::Event::Resize(w, h) => Event::Resize(w, h),
        rt::Event::Paste(s) => Event::Paste(s),
        rt::Event::FocusGained => Event::FocusGained,
        rt::Event::FocusLost => Event::FocusLost,
        rt::Event::Unsupported(bytes) => Event::Unsupported(bytes),
    }
}

pub fn to_runtime_event(event: &Event) -> rt::Event {
    match event {
        Event::Key(key) => rt::Event::Key(to_runtime_key_event(*key)),
        Event::Mouse(me) => rt::Event::Mouse(to_runtime_mouse_event(*me)),
        Event::Resize(w, h) => rt::Event::Resize(*w, *h),
        Event::Paste(s) => rt::Event::Paste(s.clone()),
        Event::FocusGained => rt::Event::FocusGained,
        Event::FocusLost => rt::Event::FocusLost,
        Event::Unsupported(bytes) => rt::Event::Unsupported(bytes.clone()),
    }
}

pub fn to_ui_event(event: &Event) -> Option<UiEvent> {
    match event {
        Event::Resize(width, height) => Some(UiEvent::Resize(UiResize {
            width: *width,
            height: *height,
        })),
        Event::Key(key) => Some(UiEvent::Key {
            key: Cow::Owned(format_key(key)),
        }),
        _ => None,
    }
}

fn from_runtime_key_event(key: rt::KeyEvent) -> KeyEvent {
    KeyEvent {
        code: from_runtime_key_code(key.code),
        modifiers: from_runtime_key_modifiers(key.modifiers),
        kind: from_runtime_key_kind(key.kind),
    }
}

fn to_runtime_key_event(key: KeyEvent) -> rt::KeyEvent {
    rt::KeyEvent {
        code: to_runtime_key_code(key.code),
        modifiers: to_runtime_key_modifiers(key.modifiers),
        kind: to_runtime_key_kind(key.kind),
    }
}

fn from_runtime_key_kind(kind: rt::KeyEventKind) -> KeyEventKind {
    match kind {
        rt::KeyEventKind::Press => KeyEventKind::Press,
        rt::KeyEventKind::Repeat => KeyEventKind::Repeat,
        rt::KeyEventKind::Release => KeyEventKind::Release,
    }
}

fn to_runtime_key_kind(kind: KeyEventKind) -> rt::KeyEventKind {
    match kind {
        KeyEventKind::Press => rt::KeyEventKind::Press,
        KeyEventKind::Repeat => rt::KeyEventKind::Repeat,
        KeyEventKind::Release => rt::KeyEventKind::Release,
    }
}

fn from_runtime_key_code(code: rt::KeyCode) -> KeyCode {
    match code {
        rt::KeyCode::Backspace => KeyCode::Backspace,
        rt::KeyCode::Enter => KeyCode::Enter,
        rt::KeyCode::Left => KeyCode::Left,
        rt::KeyCode::Right => KeyCode::Right,
        rt::KeyCode::Up => KeyCode::Up,
        rt::KeyCode::Down => KeyCode::Down,
        rt::KeyCode::Home => KeyCode::Home,
        rt::KeyCode::End => KeyCode::End,
        rt::KeyCode::PageUp => KeyCode::PageUp,
        rt::KeyCode::PageDown => KeyCode::PageDown,
        rt::KeyCode::Tab => KeyCode::Tab,
        rt::KeyCode::BackTab => KeyCode::BackTab,
        rt::KeyCode::Delete => KeyCode::Delete,
        rt::KeyCode::Insert => KeyCode::Insert,
        rt::KeyCode::F(n) => KeyCode::F(n),
        rt::KeyCode::Char(c) => KeyCode::Char(c),
        rt::KeyCode::Null => KeyCode::Null,
        rt::KeyCode::Esc => KeyCode::Esc,
        rt::KeyCode::CapsLock => KeyCode::CapsLock,
        rt::KeyCode::ScrollLock => KeyCode::ScrollLock,
        rt::KeyCode::NumLock => KeyCode::NumLock,
        rt::KeyCode::PrintScreen => KeyCode::PrintScreen,
        rt::KeyCode::Pause => KeyCode::Pause,
        rt::KeyCode::Menu => KeyCode::Menu,
        rt::KeyCode::KeypadBegin => KeyCode::KeypadBegin,
        rt::KeyCode::Media(m) => KeyCode::Media(from_runtime_media_key(m)),
        rt::KeyCode::Modifier(m) => KeyCode::Modifier(from_runtime_modifier_key(m)),
    }
}

fn to_runtime_key_code(code: KeyCode) -> rt::KeyCode {
    match code {
        KeyCode::Backspace => rt::KeyCode::Backspace,
        KeyCode::Enter => rt::KeyCode::Enter,
        KeyCode::Left => rt::KeyCode::Left,
        KeyCode::Right => rt::KeyCode::Right,
        KeyCode::Up => rt::KeyCode::Up,
        KeyCode::Down => rt::KeyCode::Down,
        KeyCode::Home => rt::KeyCode::Home,
        KeyCode::End => rt::KeyCode::End,
        KeyCode::PageUp => rt::KeyCode::PageUp,
        KeyCode::PageDown => rt::KeyCode::PageDown,
        KeyCode::Tab => rt::KeyCode::Tab,
        KeyCode::BackTab => rt::KeyCode::BackTab,
        KeyCode::Delete => rt::KeyCode::Delete,
        KeyCode::Insert => rt::KeyCode::Insert,
        KeyCode::F(n) => rt::KeyCode::F(n),
        KeyCode::Char(c) => rt::KeyCode::Char(c),
        KeyCode::Null => rt::KeyCode::Null,
        KeyCode::Esc => rt::KeyCode::Esc,
        KeyCode::CapsLock => rt::KeyCode::CapsLock,
        KeyCode::ScrollLock => rt::KeyCode::ScrollLock,
        KeyCode::NumLock => rt::KeyCode::NumLock,
        KeyCode::PrintScreen => rt::KeyCode::PrintScreen,
        KeyCode::Pause => rt::KeyCode::Pause,
        KeyCode::Menu => rt::KeyCode::Menu,
        KeyCode::KeypadBegin => rt::KeyCode::KeypadBegin,
        KeyCode::Media(m) => rt::KeyCode::Media(to_runtime_media_key(m)),
        KeyCode::Modifier(m) => rt::KeyCode::Modifier(to_runtime_modifier_key(m)),
    }
}

fn from_runtime_media_key(code: rt::MediaKeyCode) -> MediaKeyCode {
    match code {
        rt::MediaKeyCode::Play => MediaKeyCode::Play,
        rt::MediaKeyCode::Pause => MediaKeyCode::Pause,
        rt::MediaKeyCode::PlayPause => MediaKeyCode::PlayPause,
        rt::MediaKeyCode::Reverse => MediaKeyCode::Reverse,
        rt::MediaKeyCode::Stop => MediaKeyCode::Stop,
        rt::MediaKeyCode::FastForward => MediaKeyCode::FastForward,
        rt::MediaKeyCode::Rewind => MediaKeyCode::Rewind,
        rt::MediaKeyCode::TrackNext => MediaKeyCode::TrackNext,
        rt::MediaKeyCode::TrackPrevious => MediaKeyCode::TrackPrevious,
        rt::MediaKeyCode::Record => MediaKeyCode::Record,
        rt::MediaKeyCode::LowerVolume => MediaKeyCode::LowerVolume,
        rt::MediaKeyCode::RaiseVolume => MediaKeyCode::RaiseVolume,
        rt::MediaKeyCode::MuteVolume => MediaKeyCode::MuteVolume,
    }
}

fn to_runtime_media_key(code: MediaKeyCode) -> rt::MediaKeyCode {
    match code {
        MediaKeyCode::Play => rt::MediaKeyCode::Play,
        MediaKeyCode::Pause => rt::MediaKeyCode::Pause,
        MediaKeyCode::PlayPause => rt::MediaKeyCode::PlayPause,
        MediaKeyCode::Reverse => rt::MediaKeyCode::Reverse,
        MediaKeyCode::Stop => rt::MediaKeyCode::Stop,
        MediaKeyCode::FastForward => rt::MediaKeyCode::FastForward,
        MediaKeyCode::Rewind => rt::MediaKeyCode::Rewind,
        MediaKeyCode::TrackNext => rt::MediaKeyCode::TrackNext,
        MediaKeyCode::TrackPrevious => rt::MediaKeyCode::TrackPrevious,
        MediaKeyCode::Record => rt::MediaKeyCode::Record,
        MediaKeyCode::LowerVolume => rt::MediaKeyCode::LowerVolume,
        MediaKeyCode::RaiseVolume => rt::MediaKeyCode::RaiseVolume,
        MediaKeyCode::MuteVolume => rt::MediaKeyCode::MuteVolume,
    }
}

fn from_runtime_modifier_key(code: rt::ModifierKeyCode) -> ModifierKeyCode {
    match code {
        rt::ModifierKeyCode::LeftShift => ModifierKeyCode::LeftShift,
        rt::ModifierKeyCode::LeftControl => ModifierKeyCode::LeftControl,
        rt::ModifierKeyCode::LeftAlt => ModifierKeyCode::LeftAlt,
        rt::ModifierKeyCode::LeftSuper => ModifierKeyCode::LeftSuper,
        rt::ModifierKeyCode::LeftHyper => ModifierKeyCode::LeftHyper,
        rt::ModifierKeyCode::LeftMeta => ModifierKeyCode::LeftMeta,
        rt::ModifierKeyCode::RightShift => ModifierKeyCode::RightShift,
        rt::ModifierKeyCode::RightControl => ModifierKeyCode::RightControl,
        rt::ModifierKeyCode::RightAlt => ModifierKeyCode::RightAlt,
        rt::ModifierKeyCode::RightSuper => ModifierKeyCode::RightSuper,
        rt::ModifierKeyCode::RightHyper => ModifierKeyCode::RightHyper,
        rt::ModifierKeyCode::RightMeta => ModifierKeyCode::RightMeta,
        rt::ModifierKeyCode::IsoLevel3Shift => ModifierKeyCode::IsoLevel3Shift,
        rt::ModifierKeyCode::IsoLevel5Shift => ModifierKeyCode::IsoLevel5Shift,
    }
}

fn to_runtime_modifier_key(code: ModifierKeyCode) -> rt::ModifierKeyCode {
    match code {
        ModifierKeyCode::LeftShift => rt::ModifierKeyCode::LeftShift,
        ModifierKeyCode::LeftControl => rt::ModifierKeyCode::LeftControl,
        ModifierKeyCode::LeftAlt => rt::ModifierKeyCode::LeftAlt,
        ModifierKeyCode::LeftSuper => rt::ModifierKeyCode::LeftSuper,
        ModifierKeyCode::LeftHyper => rt::ModifierKeyCode::LeftHyper,
        ModifierKeyCode::LeftMeta => rt::ModifierKeyCode::LeftMeta,
        ModifierKeyCode::RightShift => rt::ModifierKeyCode::RightShift,
        ModifierKeyCode::RightControl => rt::ModifierKeyCode::RightControl,
        ModifierKeyCode::RightAlt => rt::ModifierKeyCode::RightAlt,
        ModifierKeyCode::RightSuper => rt::ModifierKeyCode::RightSuper,
        ModifierKeyCode::RightHyper => rt::ModifierKeyCode::RightHyper,
        ModifierKeyCode::RightMeta => rt::ModifierKeyCode::RightMeta,
        ModifierKeyCode::IsoLevel3Shift => rt::ModifierKeyCode::IsoLevel3Shift,
        ModifierKeyCode::IsoLevel5Shift => rt::ModifierKeyCode::IsoLevel5Shift,
    }
}

fn from_runtime_key_modifiers(m: rt::KeyModifiers) -> KeyModifiers {
    KeyModifiers::from_bits_retain(m.bits())
}

fn to_runtime_key_modifiers(m: KeyModifiers) -> rt::KeyModifiers {
    rt::KeyModifiers::from_bits_retain(m.bits())
}

fn from_runtime_mouse_event(me: rt::MouseEvent) -> MouseEvent {
    MouseEvent {
        kind: from_runtime_mouse_kind(me.kind),
        column: me.column,
        row: me.row,
        modifiers: from_runtime_key_modifiers(me.modifiers),
    }
}

fn to_runtime_mouse_event(me: MouseEvent) -> rt::MouseEvent {
    rt::MouseEvent {
        kind: to_runtime_mouse_kind(me.kind),
        column: me.column,
        row: me.row,
        modifiers: to_runtime_key_modifiers(me.modifiers),
    }
}

fn from_runtime_mouse_kind(kind: rt::MouseEventKind) -> MouseEventKind {
    match kind {
        rt::MouseEventKind::Down(b) => MouseEventKind::Down(from_runtime_mouse_button(b)),
        rt::MouseEventKind::Up(b) => MouseEventKind::Up(from_runtime_mouse_button(b)),
        rt::MouseEventKind::Drag(b) => MouseEventKind::Drag(from_runtime_mouse_button(b)),
        rt::MouseEventKind::Moved => MouseEventKind::Moved,
        rt::MouseEventKind::ScrollDown => MouseEventKind::ScrollDown,
        rt::MouseEventKind::ScrollUp => MouseEventKind::ScrollUp,
        rt::MouseEventKind::ScrollLeft => MouseEventKind::ScrollLeft,
        rt::MouseEventKind::ScrollRight => MouseEventKind::ScrollRight,
    }
}

fn to_runtime_mouse_kind(kind: MouseEventKind) -> rt::MouseEventKind {
    match kind {
        MouseEventKind::Down(b) => rt::MouseEventKind::Down(to_runtime_mouse_button(b)),
        MouseEventKind::Up(b) => rt::MouseEventKind::Up(to_runtime_mouse_button(b)),
        MouseEventKind::Drag(b) => rt::MouseEventKind::Drag(to_runtime_mouse_button(b)),
        MouseEventKind::Moved => rt::MouseEventKind::Moved,
        MouseEventKind::ScrollDown => rt::MouseEventKind::ScrollDown,
        MouseEventKind::ScrollUp => rt::MouseEventKind::ScrollUp,
        MouseEventKind::ScrollLeft => rt::MouseEventKind::ScrollLeft,
        MouseEventKind::ScrollRight => rt::MouseEventKind::ScrollRight,
    }
}

fn from_runtime_mouse_button(button: rt::MouseButton) -> MouseButton {
    match button {
        rt::MouseButton::Left => MouseButton::Left,
        rt::MouseButton::Right => MouseButton::Right,
        rt::MouseButton::Middle => MouseButton::Middle,
        rt::MouseButton::Back => MouseButton::Back,
        rt::MouseButton::Forward => MouseButton::Forward,
        rt::MouseButton::Other(v) => MouseButton::Other(v),
    }
}

fn to_runtime_mouse_button(button: MouseButton) -> rt::MouseButton {
    match button {
        MouseButton::Left => rt::MouseButton::Left,
        MouseButton::Right => rt::MouseButton::Right,
        MouseButton::Middle => rt::MouseButton::Middle,
        MouseButton::Back => rt::MouseButton::Back,
        MouseButton::Forward => rt::MouseButton::Forward,
        MouseButton::Other(v) => rt::MouseButton::Other(v),
    }
}

fn format_key(key: &KeyEvent) -> String {
    let mut out = String::new();

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        out.push_str("ctrl+");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        out.push_str("alt+");
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        out.push_str("shift+");
    }
    if key.modifiers.contains(KeyModifiers::SUPER) {
        out.push_str("super+");
    }
    if key.modifiers.contains(KeyModifiers::HYPER) {
        out.push_str("hyper+");
    }
    if key.modifiers.contains(KeyModifiers::META) {
        out.push_str("meta+");
    }

    out.push_str(match key.code {
        KeyCode::Backspace => "backspace",
        KeyCode::Enter => "enter",
        KeyCode::Left => "left",
        KeyCode::Right => "right",
        KeyCode::Up => "up",
        KeyCode::Down => "down",
        KeyCode::Home => "home",
        KeyCode::End => "end",
        KeyCode::PageUp => "page_up",
        KeyCode::PageDown => "page_down",
        KeyCode::Tab => "tab",
        KeyCode::BackTab => "backtab",
        KeyCode::Delete => "delete",
        KeyCode::Insert => "insert",
        KeyCode::Null => "null",
        KeyCode::Esc => "esc",
        KeyCode::CapsLock => "caps_lock",
        KeyCode::ScrollLock => "scroll_lock",
        KeyCode::NumLock => "num_lock",
        KeyCode::PrintScreen => "print_screen",
        KeyCode::Pause => "pause",
        KeyCode::Menu => "menu",
        KeyCode::KeypadBegin => "keypad_begin",
        KeyCode::Char(c) => return format!("{out}{c}"),
        KeyCode::F(n) => return format!("{out}f{n}"),
        KeyCode::Media(media) => return format!("{out}media::{media:?}"),
        KeyCode::Modifier(modifier) => return format!("{out}modifier::{modifier:?}"),
    });

    match key.kind {
        KeyEventKind::Press => out.push_str(":press"),
        KeyEventKind::Repeat => out.push_str(":repeat"),
        KeyEventKind::Release => out.push_str(":release"),
    }

    out
}
