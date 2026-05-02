# AI Guide: Writing Dracon Terminal Engine Code

You are writing Rust code using `dracon_terminal_engine`, the next-generation terminal engine.
Dracon Terminal Engine is **NOT** `crossterm` (Immediate Mode, Global State) and **NOT** `ratatui` (Grid Mode).
It is a **Compositor Engine** — z-indexed layers, TrueColor, SGR mouse, Kitty keyboard.

## 1. The Golden Rule: RAII

Dracon Terminal Engine has NO global state. Do not send raw ANSI bytes to `stdout` unless wrapped in `Terminal`.
**Always** wrap `stdout` in `Terminal` to handle Raw Mode entry/exit.

```rust
use std::io::stdout;
use dracon_terminal_engine::core::terminal::Terminal;

let mut term = Terminal::new(stdout())?;
// term is now in Raw Mode. When dropped, it restores terminal state.
```

## 2. The Compositor Pattern (Layers)

Do not draw generic text. Use **Planes** with z-indices.
The compositor uses the **Painter's Algorithm** (higher z-index = on top).

### Creating a Floating Window

```rust
use dracon_terminal_engine::compositor::{Compositor, Plane};
use dracon_terminal_engine::compositor::filter::Dim;

// Initialize Compositor
let (w, h) = dracon_terminal_engine::backend::tty::get_window_size(std::io::stdout().as_fd())?;
let mut compositor = Compositor::new(w, h);

// Base Layer (Background)
let mut base = Plane::new(0, w, h);
base.set_z_index(0);
compositor.add_plane(base);

// Floating Modal (Foreground)
let mut modal = Plane::new(1, 40, 10);
modal.set_z_index(100);
modal.set_absolute_position(20, 5);
modal.set_filter(Box::new(Dim));
compositor.add_plane(modal);

// Render
compositor.render(&mut term)?;
```

## 3. Input Handling

Supports Kitty Keyboard Protocol and SGR Mouse (including side buttons Back/Forward).
Use `dracon_terminal_engine::input::parser::Parser`.

```rust
use dracon_terminal_engine::input::parser::{Parser};
use dracon_terminal_engine::input::event::{Event, MouseButton};

let mut parser = Parser::new();
if let Some(event) = parser.advance(byte) {
    match event {
        Event::Mouse(me) => match me.kind {
            MouseEventKind::Down(MouseButton::Back) => { /* Go Back */ },
            MouseEventKind::Down(MouseButton::Forward) => { /* Go Forward */ },
            _ => {}
        },
        _ => {}
    }
}
```

## 4. Ratatui Integration

Use `ratatui` with `RatatuiBackend` for standard widgets (Block, Paragraph) combined with floating Planes.

```rust
use dracon_terminal_engine::integration::ratatui::RatatuiBackend;
use ratatui::Terminal;

let backend = RatatuiBackend::new(stdout())?;
let mut terminal = Terminal::new(backend)?;

// Access the underlying compositor to add custom layers
terminal.backend_mut().compositor_mut().add_plane(my_plane);
```

## 5. Visual Polish

Use **Synchronized Updates** (Mode 2026) for non-trivial renders to prevent tearing.
`RatatuiBackend` handles this automatically on flush.

## 6. Unicode & Wide Character Handling

Dracon Terminal Engine is **width-aware**. Characters like Kanji and Emoji take **2 columns**.
If not handled correctly, this breaks borders and overlaps adjacent content.

### The "Skip" Flag Pattern
When a character has width 2, cell `(x, y)` contains the character, and cell `(x+1, y)` **MUST** be marked `skip = true`.
- **Renderer**: Skips cells with `skip: true`
- **Compositor**: `blend_cells` propagates the `skip` flag

### Utilities
- `dracon_terminal_engine::utils::get_visual_width(c)` — character display width
- `dracon_terminal_engine::utils::truncate_to_width(s, max_width, suffix)` — safe string clipping

## Summary

- **Structs**: `Terminal`, `Compositor`, `Plane`
- **Backend**: `RatatuiBackend` for ratatui integration
- **Z-Index**: Use it for overlapping UI
- **No Macros**: Use struct methods, not `crossterm::queue!` style
