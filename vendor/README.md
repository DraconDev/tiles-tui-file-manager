```text
  _______   ______   .______      .___  ___.      ___
 |       | |   ___|  |   _  \     |   \/   |     /   \
 |.|   | | |  |__    |  |_)  |    |  \  /  |    /  ^  \
   |   |   |   __|   |      /     |  |\/|  |   /  /_\  \
   |   |   |  |____  |  |\  \----.|  |  |  |  /  _____  \
   |___|   |_______| | _| `._____||__|  |__| /__/     \__\

```

> **A terminal compositor engine for Rust.**

---

## What It Is

`dracon-terminal-engine` is a z-indexed, event-driven terminal runtime. Not a "TUI library" — an engine that owns the terminal, renders compositing layers, parses advanced input protocols, and ships with built-in widgets.

**Self-contained.** Contracts and input mapping are baked in — no external contract crates needed.

---

## Core

### 1. Compositor (Z-Indexed Layers)

Think in **layers**, not rows/columns. Spawn a `Plane`, set its Z-Index, float it above your app.

- **Layer 0**: Background / wallpaper
- **Layer 10**: Main application
- **Layer 100**: Modal dialogs & toasts
- **Layer 9000**: Debug overlays

### 2. Input

- **Kitty Keyboard Protocol**: Chords, modifiers, release events
- **SGR Mouse**: Click, drag, scroll, extra buttons
- **Contract types**: `InputEvent`, `KeyCode`, `KeyEvent`, `MouseEvent` — all in `input::mapping`

### 3. Visuals

- **Images**: High-res PNG/JPG via Kitty protocol
- **Procedural geometry**: Rounded rects, circles, gradients
- **TrueColor**: 24-bit by default

### 4. Editor Widget

- **Syntax highlighting**: `syntect` with built-in themes
- **Smart filters**: Live fuzzy-finding
- **Unlimited undo/redo**
- **Multi-selection**: Shift+Arrows batch edits

### 5. Ratatui Bridge

Drop-in `ratatui` integration via `integration::ratatui`.

---

## Installation

```toml
[dependencies]
dracon-terminal-engine = { git = "https://github.com/DraconDev/dracon-terminal-engine" }
```

## Quick Start

```rust
use dracon_terminal_engine::core::terminal::Terminal;
use dracon_terminal_engine::compositor::Plane;

fn main() -> std::io::Result<()> {
    let stdout = std::io::stdout();
    let mut terminal = Terminal::new(stdout)?;

    // Create a floating layer
    let mut hud = Plane::new(40, 10);
    hud.set_z_index(50);
    hud.put_str(2, 2, "SYSTEM ONLINE");

    // Compose and render
    // ...

    Ok(())
}
```

## Modules

| Module | What |
|---|---|
| `contracts` | Input types + traits (`InputEvent`, `UiRenderer`, `UiRuntime`) |
| `input::mapping` | Event conversion (runtime ↔ contract types) |
| `core::terminal::Terminal` | RAII terminal wrapper (raw mode, alt screen, cleanup) |
| `compositor` | Z-indexed layer engine |
| `input::parser` | Kitty keyboard + SGR mouse parsing |
| `widgets::editor` | Code editor with syntax highlighting |
| `widgets::input` | Text input widget |
| `integration::ratatui` | Ratatui bridge |
| `backend::tty` | Low-level terminal control |
| `visuals` | Images, tiles, icons, rich widgets |

## License

MIT
